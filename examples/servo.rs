use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, Resolution};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::log::EspLogger;

/// Button:
/// Blue -> GPIO-9
/// Green -> GND

/// Servo:
/// Orange -> GPIO-10
/// Red -> PWR
/// Brown -> GND

fn main() {
    // Bind the log crate to the ESP Logging facilities
    EspLogger::initialize_default();

    // Take Peripherals
    let peripherals = Peripherals::take().unwrap();

    let mut button = PinDriver::input(peripherals.pins.gpio9).unwrap();
    button.set_pull(Pull::Down).unwrap();

    // Configure and Initialize LEDC Timer Driver
    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default()
            .frequency(50.Hz())
            .resolution(Resolution::Bits14),
    )
    .unwrap();

    // Configure and Initialize LEDC Driver
    let mut driver = LedcDriver::new(
        peripherals.ledc.channel0,
        timer_driver,
        peripherals.pins.gpio10,
    )
    .unwrap();

    // Get Max Duty and Calculate Upper and Lower Limits for Servo
    let max_duty = driver.get_max_duty();
    println!("Max Duty {}", max_duty);
    let min_limit = max_duty * 25 / 1000;
    println!("Min Limit {}", min_limit);
    let max_limit = max_duty * 125 / 1000;
    println!("Max Limit {}", max_limit);

    // Change this to modify the angle the servo starts at.
    // The servo will jump to this position when the program starts.
    let angle_start = 0;
    // Change this to modify the angle the servo sweeps
    let angle_change = 120;

    // Define Starting Position
    driver
        .set_duty(map(angle_start, 0, 180, min_limit, max_limit))
        .unwrap();
    // Give servo some time to update

    let mut toggle = Toggle::CCW;

    loop {
        FreeRtos::delay_ms(50);
        // button low = pressed
        // button high = not pressed

        if button.is_low() {
            match toggle {
                Toggle::CCW => {
                    println!("Moving CCW");
                    for mut angle in angle_start..(angle_start + angle_change) {
                        // Print Current Angle for visual verification
                        println!("Current Angle {} Degrees", angle);

                        if angle > 180 {
                            angle = 180;
                        }

                        // Set the desired duty cycle
                        driver
                            .set_duty(map(angle, 0, 180, min_limit, max_limit))
                            .unwrap();
                        // Give servo some time to update
                        FreeRtos::delay_ms(12); // Increase this delay to slow rotation speed
                        toggle = Toggle::CW;
                    }
                }
                Toggle::CW => {
                    println!("Moving CW");
                    for mut angle in (angle_start..(angle_start + angle_change)).rev() {
                        // Print Current Angle for visual verification
                        println!("Current Angle {} Degrees", angle);

                        if angle > 180 {
                            angle = 180;
                        }

                        // Set the desired duty cycle
                        driver
                            .set_duty(map(angle, 0, 180, min_limit, max_limit))
                            .unwrap();
                        // Give servo some time to update
                        FreeRtos::delay_ms(12);
                        toggle = Toggle::CCW;
                    }
                }
            }
        }
    }
}

enum Toggle {
    CCW,
    CW,
}

// Function that maps one range to another
fn map(x: u32, in_min: u32, in_max: u32, out_min: u32, out_max: u32) -> u32 {
    (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}
