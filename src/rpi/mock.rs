#![allow(dead_code)]
pub struct MockGpio;

impl MockGpio {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(MockGpio)
    }

    pub fn get(&self, pin_id: u8) -> Result<MockPin, anyhow::Error> {
        Ok(MockPin::new(pin_id))
    }
}

pub struct MockPin {
    gpio_id: u8,
}

impl MockPin {
    fn new(gpio_id: u8) -> Self {
        Self { gpio_id }
    }

    pub fn into_output(self) -> MockOutputPin {
        self.into()
    }

    pub fn into_input_pullup(self) -> MockInputPin {
        self.into()
    }

    pub fn into_input(self) -> MockInputPin {
        self.into()
    }
}

pub struct MockOutputPin {
    gpio_id: u8,
}

impl MockOutputPin {
    pub fn new(id: u8) -> Self {
        Self { gpio_id: id }
    }

    // needs to be &mut for rppal compat
    pub fn set_high(&mut self) {
        //println!("set {} pin to high", self.gpio_id);
    }

    // needs to be &mut for rppal compat
    pub fn set_low(&mut self) {
        //println!("set {} pin to low", self.gpio_id)
    }
}

pub struct MockInputPin {
    gpio_id: u8,
}

impl MockInputPin {
    pub fn new(id: u8) -> Self {
        Self { gpio_id: id }
    }

    pub fn is_high(&self) -> bool {
        //println!("checking is high, this is unpressed state");
        true
    }

    pub fn is_low(&self) -> bool {
        //println!("checking is high, this is unpressed state");
        true
    }
}

impl From<MockPin> for MockOutputPin {
    fn from(value: MockPin) -> Self {
        MockOutputPin::new(value.gpio_id)
    }
}

impl From<MockPin> for MockInputPin {
    fn from(value: MockPin) -> Self {
        MockInputPin::new(value.gpio_id)
    }
}
