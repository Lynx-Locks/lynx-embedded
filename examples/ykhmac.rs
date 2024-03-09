use embedded_hal::spi::MODE_0;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::FromValueType;
use esp_idf_svc::hal::spi::config::BitOrder;
use esp_idf_svc::hal::spi::{config, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2};
use esp_idf_svc::hal::timer::{TimerConfig, TimerDriver};
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

use lynx_embedded::Pn532;
use lynx_embedded::{ykhmac, Led};

fn main() -> anyhow::Result<()> {
    // Bind the log crate to the ESP Logging facilities
    EspLogger::initialize_default();
    let _sys_loop = EspSystemEventLoop::take()?;
    let _nvs = EspDefaultNvsPartition::take()?;
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
    let device = SpiDeviceDriver::new(driver, Some(cs), &config)?;

    let timer = TimerDriver::new(peripherals.timer10, &TimerConfig::new())?;

    if let Err(e) = ykhmac::initialize_pn532(Pn532::new(device, timer)) {
        log::error!("Failed to initialize PN532: {e:?}");
        return Ok(());
    }

    let secret_key_str = "deadbeef";
    if let Err(e) = ykhmac::enroll_key(secret_key_str) {
        log::error!("Failed to enroll key! {e:?}");
        return Ok(());
    }

    let mut led = Led::new(
        esp_idf_svc::sys::rmt_channel_t_RMT_CHANNEL_0,
        esp_idf_svc::sys::gpio_num_t_GPIO_NUM_8,
    )?;
    let mut authenticated = true;

    log::info!("Waiting for NFC target...");
    loop {
        if authenticated {
            authenticated = false;
            led.set_color(0x10, 0x00, 0x00)?;
        }
        if ykhmac::authenticate() {
            authenticated = true;
            // Set LED to green for 3 seconds, then back to red.
            led.set_color(0x00, 0x10, 0x00)?;
            FreeRtos::delay_ms(3000)
        }
    }
}
