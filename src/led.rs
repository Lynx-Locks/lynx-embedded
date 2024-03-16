use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_svc::hal::peripheral::Peripheral;

pub struct Led<'d, T: OutputPin> {
    pin: PinDriver<'d, T, Output>,
}

impl<'d, T: OutputPin> Led<'d, T> {
    pub fn new(pin: impl Peripheral<P = T> + 'd) -> Self {
        let pin = PinDriver::output(pin).expect("Cannot convert pin to output");
        Self { pin }
    }

    pub fn set(&mut self, enable: bool) {
        if enable {
            self.pin.set_high().expect("Cannot set pin to high");
        } else {
            self.pin.set_low().expect("Cannot set pin to low");
        }
    }

    pub fn toggle(&mut self) {
        self.pin.toggle().expect("Cannot toggle pin");
    }

    pub fn is_on(&self) -> bool {
        self.pin.is_set_high()
    }
}
