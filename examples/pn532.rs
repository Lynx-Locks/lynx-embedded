use anyhow::Result;
use embedded_hal::spi::MODE_0;

use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::FromValueType;
use esp_idf_svc::hal::spi::config::BitOrder;
use esp_idf_svc::hal::spi::{config, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2};
use esp_idf_svc::hal::timer::{TimerConfig, TimerDriver};
use esp_idf_svc::log::EspLogger;

use lynx_embedded::Pn532;

fn main() -> Result<()> {
    // Bind the log crate to the ESP Logging facilities
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

    let spi = peripherals.spi2;

    let sclk = peripherals.pins.gpio7;
    let miso = peripherals.pins.gpio6; // SDI
    let mosi = peripherals.pins.gpio5; // SDO
    let cs = peripherals.pins.gpio4;

    let driver = SpiDriver::new::<SPI2>(spi, sclk, mosi, Some(miso), &SpiDriverConfig::new())?;
    let config = config::Config::new()
        .baudrate(100000.Hz())
        .data_mode(MODE_0)
        .bit_order(BitOrder::LsbFirst);
    let device = SpiDeviceDriver::new(&driver, Some(cs), &config)?;

    let timer = TimerDriver::new(peripherals.timer10, &TimerConfig::new())?;

    let mut pn532: Pn532<_, 64> = Pn532::new(device, timer);

    if let Err(e) = pn532.print_firmware_version() {
        log::error!("Cannot get firmware version! {e:?}");
        return Ok(());
    }

    if let Err(e) = pn532.sam_config() {
        log::error!("Cannot set SAM config! {e:?}");
        return Ok(());
    }
    if let Err(e) = pn532.set_passive_activation_retries(0xFF) {
        log::error!("Cannot set retries! {e:?}");
        return Ok(());
    }

    log::info!("Waiting for NFC target...");
    loop {
        pn532.inlist_passive_target().ok();
    }
}
