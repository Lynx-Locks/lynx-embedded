use std::thread;
use std::time::Duration;

use rand::random;
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::SmartLedsWrite;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::log::EspLogger;

fn main() -> ! {
    // Bind the log crate to the ESP Logging facilities
    EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let led_pin = peripherals.pins.gpio3;
    let channel = peripherals.rmt.channel0;
    let mut ws2812 = Ws2812Esp32Rmt::new(channel, led_pin).unwrap();

    log::info!("Start rainbow!");

    let mut hue = random();
    loop {
        let pixels = std::iter::repeat(hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 8,
        }))
        .take(25);
        ws2812.write(pixels).unwrap();

        thread::sleep(Duration::from_millis(10));

        hue = hue.wrapping_add(1);
    }
}
