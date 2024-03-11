use anyhow::Result;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, Resolution};
use esp_idf_svc::hal::prelude::{FromValueType, Peripherals};
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use hyper::StatusCode;

use lynx_embedded::{reqwesp, wifi as espWifi, Led};

fn main() -> Result<()> {
    // Bind the log crate to the ESP Logging facilities
    EspLogger::initialize_default();

    // Configure Wifi
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    espWifi::connect(&mut wifi)?;
    log::info!("Wifi connected!");

    let mut led = Led::new(
        esp_idf_svc::sys::rmt_channel_t_RMT_CHANNEL_0,
        esp_idf_svc::sys::gpio_num_t_GPIO_NUM_8,
    )?;

    led.set_color(0x00, 0x00, 0x00)?;

    // Configure and Initialize LEDC Timer Driver
    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default()
            .frequency(50.Hz())
            .resolution(Resolution::Bits14),
    )
    .unwrap();

    // Configure and Initialize LEDC Driver
    let servo_driver = LedcDriver::new(
        peripherals.ledc.channel0,
        timer_driver,
        peripherals.pins.gpio10,
    )
    .unwrap();

    let start_position = DoorPosition::Neutral;
    let servo_delay = 12;

    let mut servo = ServoHandler::new(servo_driver, start_position, servo_delay);
    FreeRtos::delay_ms(100);

    let mut client = reqwesp::Client::new()?;
    // Endpoint for testing REST requests
    let url = "https://app.lynx-locks.com/api/doors/unlocked/1";

    loop {
        FreeRtos::delay_ms(500);

        let mut req = client.get(url);
        let res = req.send()?;

        if let StatusCode::OK = res.status() {
            log::info!("Door unlocked!");
            led.set_color(0x00, 0x10, 0x00)?;
            servo.set_position(DoorPosition::Unlocked);
            FreeRtos::delay_ms(10000);
            led.set_color(0x00, 0x00, 0x00)?;
            servo.set_position(DoorPosition::Locked);
            servo.set_position(DoorPosition::Neutral);
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DoorPosition {
    Neutral = 90,
    Unlocked = 37,
    Locked = 35,
}

struct ServoHandler<'a> {
    servo: LedcDriver<'a>,
    current_position: u32,
    max_duty: u32,
    servo_delay: u32,
}

impl<'a> ServoHandler<'a> {
    pub fn new(mut servo: LedcDriver<'a>, start_position: DoorPosition, servo_delay: u32) -> Self {
        let max_duty = servo.get_max_duty();
        let min_limit = max_duty * 25 / 1000;
        let max_limit = max_duty * 125 / 1000;
        servo
            .set_duty(Self::map(
                start_position as u32,
                0,
                180,
                min_limit,
                max_limit,
            ))
            .unwrap();
        Self {
            servo,
            current_position: start_position as u32,
            max_duty,
            servo_delay,
        }
    }

    pub fn set_position(&mut self, position: DoorPosition) {
        log::info!("Moving to {position:?} position...");
        for mut angle in Self::angle_range(self.current_position, position as u32) {
            if angle > 180 {
                angle = 180;
            }
            // Set the desired duty cycle
            self.set_duty(angle);
            // Give servo some time to update
            FreeRtos::delay_ms(self.servo_delay);
        }
        self.current_position = position as u32;
        log::info!("Finished moving to {position:?} position!");
    }

    fn set_duty(&mut self, position: u32) {
        log::info!("position: {position}");
        let min_limit = self.max_duty * 25 / 1000;
        let max_limit = self.max_duty * 125 / 1000;
        self.servo
            .set_duty(Self::map(position, 0, 180, min_limit, max_limit))
            .unwrap();
    }

    fn map(x: u32, in_min: u32, in_max: u32, out_min: u32, out_max: u32) -> u32 {
        (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
    }

    fn angle_range(a: u32, b: u32) -> Box<dyn Iterator<Item = u32>> {
        if b > a {
            Box::new(a..=b)
        } else {
            Box::new((b..=a).rev())
        }
    }
}
