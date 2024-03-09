use anyhow::Result;
use core::borrow::Borrow;
use core::time::Duration;
use std::task::Poll;

use embedded_hal_0_2::timer::CountDown;
use esp_idf_svc::hal::delay::FreeRtos;

use esp_idf_svc::hal::spi::*;
use esp_idf_svc::hal::timer::TimerDriver;
use esp_idf_svc::sys::EspError;

use pn532::requests::{BorrowedRequest, Command};
use pn532::spi::{PN532_SPI_DATAREAD, PN532_SPI_DATAWRITE, PN532_SPI_READY, PN532_SPI_STATREAD};
use pn532::{requests::SAMMode, Interface, Request};

pub type Pn532Error = pn532::Error<EspError>;

pub struct Pn532<'d, S, const N: usize = 32>
where
    S: Borrow<SpiDriver<'d>> + 'd,
{
    pn532: pn532::Pn532<SpiWrapper<'d, S>, TimerWrapper<'d>, N>,
    timeout: Duration,
    target: u8,
}

impl<'d, S: Borrow<SpiDriver<'d>> + 'd, const N: usize> Pn532<'d, S, N> {
    pub fn new(device: SpiDeviceDriver<'d, S>, timer: TimerDriver<'d>) -> Self {
        let device_wrap = SpiWrapper::wrap(device);
        let timer_wrap = TimerWrapper::wrap(timer);
        let pn532: pn532::Pn532<_, _, N> = pn532::Pn532::new(device_wrap, timer_wrap);
        Self {
            pn532,
            timeout: Duration::from_millis(50),
            target: 0,
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn get_firmware_version(&mut self) -> Result<u32, Pn532Error> {
        match self.pn532.process(
            &Request::GET_FIRMWARE_VERSION,
            4,
            self.timeout,
            self.timeout,
        ) {
            Ok(res) => {
                let mut version: u32;
                version = res[0] as u32;
                version <<= 8;
                version |= res[1] as u32;
                version <<= 8;
                version |= res[2] as u32;
                version <<= 8;
                version |= res[3] as u32;

                Ok(version)
            }
            Err(e) => {
                log::error!("Could not get PN532 firmware version: {e:?}");
                Err(e)
            }
        }
    }

    pub fn print_firmware_version(&mut self) -> Result<(), Pn532Error> {
        let version = self.get_firmware_version()?;
        log::info!(
            "Firmware ver. {}.{}",
            (version >> 16) & 0xFF,
            (version >> 8) & 0xFF
        );
        Ok(())
    }

    pub fn sam_config(&mut self) -> Result<(), Pn532Error> {
        if let Err(e) = self.pn532.process(
            &Request::sam_configuration(SAMMode::Normal, false),
            0,
            self.timeout,
            self.timeout,
        ) {
            log::error!("Could not initialize PN532: {e:?}");
            return Err(e);
        }
        Ok(())
    }

    pub fn set_passive_activation_retries(&mut self, retries: u8) -> Result<(), Pn532Error> {
        let mut buf = [0u8; 4];
        buf[0] = 5; // Config item 5 (MaxRetries)
        buf[1] = 0xFF; // MxRtyATR (default = 0xFF)
        buf[2] = 0x01; // MxRtyPSL (default = 0x01)
        buf[3] = retries;

        if let Err(e) = self.pn532.process(
            &Request::new(Command::RFConfiguration, buf),
            0,
            self.timeout,
            self.timeout,
        ) {
            log::error!("Could not set passive activation retried: {e:?}");
            return Err(e);
        }
        Ok(())
    }

    pub fn inlist_passive_target(&mut self) -> Result<(), Pn532Error> {
        let mut target = 0;
        let response = match self.pn532.process(
            &Request::INLIST_ONE_ISO_A_TARGET,
            N - 9,
            Duration::from_millis(1000),
            Duration::from_millis(30000),
        ) {
            Ok(res) => {
                // ISO14443A card response should be in the following format:
                //
                // byte            index           Description
                // -------------   -------------   ------------------------------------------
                // b0..6           N/A (removed)   Frame header and preamble (cut from response)
                // b7              0               Tags Found
                // b8              1               Tag Number (only one used in this example)
                // b9..10          2..3            SENS_RES
                // b11             4               SEL_RES
                // b12             5               NFCID Length
                // b13..NFCIDLen   6..NFCIDLen     NFCID

                if res[0] != 1 {
                    log::warn!("Unhandled number of targets inlisted");
                    log::warn!("Number of tags inlisted: {}", res[7]);
                    return Err(pn532::Error::BadResponseFrame);
                }

                log::info!("Tag Number: {}", res[1]);
                target = res[1];

                let sens_res: u16 = (res[2] as u16) << 8 | res[3] as u16;
                log::debug!("ATQA: 0x{sens_res:02X}");
                log::debug!("SAK: 0x{:02X}", res[4]);

                let uid_length = res[5];
                log::info!("UID Length: {uid_length}");

                let uid = &res[6..6 + uid_length as usize];
                log::info!("UID Value: {uid:02X?}");
                Ok(())
            }
            Err(e) => {
                if let Pn532Error::TimeoutResponse = e {
                    // TimeoutResponse occurs if a tag has not been detected in time.
                    // This doesn't necessarily indicate an error, so we will debug log to prevent congestion.
                    log::debug!("Failed to inlist passive target: {e:?}");
                } else {
                    log::error!("Failed to inlist passive target: {e:?}");
                }
                Err(e)
            }
        };
        self.target = target;
        response
    }

    pub fn in_data_exchange(&mut self, send: &[u8], response: &mut [u8]) -> Result<u8, Pn532Error> {
        let send_length = send.len();
        let response_length = response.len();

        log::debug!("InDataExchange: Sending Bytes: {send:02X?} (size = {send_length})");
        log::debug!("InDataExchange: Expected Response Length: {response_length}");

        let mut buf = Vec::with_capacity(1 + send_length);
        buf.push(self.target); // Use the most recently detected target from inlist_passive_target
        buf.extend_from_slice(send);

        match self.pn532._process(
            // We cannot know the size of buf on compile-time,
            // so we must use BorrowedRequest for this command.
            BorrowedRequest::new(Command::InDataExchange, buf.as_slice()),
            N - 9,
            Duration::from_millis(1000),
            Duration::from_millis(1000),
        ) {
            Ok(res) => {
                log::debug!("InDataExchange: Received Bytes: {res:02X?}");
                if (res[0] & 0x3f) != 0 {
                    log::error!("Status code indicates an error");
                    return Err(pn532::Error::BadResponseFrame);
                }

                let mut length = res.len() as u8 - 1;

                if length > response_length as u8 {
                    length = response_length as u8 // silent truncation...
                }

                for i in 0..length {
                    response[i as usize] = res[(i + 1) as usize]
                }

                // The length of the actual response (truncated to the provided length if too long)
                // length <= response_length
                Ok(length)
            }
            Err(e) => {
                log::error!("Failed to process in data exchange command: {e:?}");
                Err(e)
            }
        }
    }
}

struct TimerWrapper<'d> {
    driver: TimerDriver<'d>,
    duration: Duration,
}

impl<'d> TimerWrapper<'d> {
    fn wrap(driver: TimerDriver<'d>) -> Self {
        Self {
            driver,
            duration: Duration::from_millis(0),
        }
    }
}

impl<'d> CountDown for TimerWrapper<'d> {
    type Time = Duration;

    fn start<T>(&mut self, duration: T)
    where
        T: Into<Self::Time>,
    {
        self.duration = duration.into();
        self.driver.set_counter(0).unwrap();
        self.driver.counter().unwrap();
        self.driver.enable(true).unwrap();
    }

    fn wait(&mut self) -> nb::Result<(), void::Void> {
        let count = self.driver.counter().unwrap();
        if count >= self.duration.as_micros() as u64 {
            return Ok(());
        }
        Err(nb::Error::WouldBlock)
    }
}

struct SpiWrapper<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    device: SpiDeviceDriver<'d, T>,
}

impl<'d, T> SpiWrapper<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    fn wrap(device: SpiDeviceDriver<'d, T>) -> Self {
        Self { device }
    }
}

impl<'d, T> Interface for SpiWrapper<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    type Error = EspError;

    fn write(&mut self, frame: &[u8]) -> std::result::Result<(), Self::Error> {
        self.device.transaction(&mut [
            Operation::Write(&[PN532_SPI_DATAWRITE]),
            Operation::Write(frame),
        ])
    }

    fn wait_ready(&mut self) -> Poll<std::result::Result<(), Self::Error>> {
        FreeRtos::delay_ms(1); // Required to stop ESP32 watchdogs from triggering
        let mut buf = [0u8];
        self.device.transaction(&mut [
            Operation::Write(&[PN532_SPI_STATREAD]),
            Operation::Read(&mut buf),
        ])?;

        if buf[0] == PN532_SPI_READY {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<(), Self::Error> {
        self.device.transaction(&mut [
            Operation::Write(&[PN532_SPI_DATAREAD]),
            Operation::Read(buf),
        ])
    }
}
