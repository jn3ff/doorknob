#[allow(dead_code)]
#[cfg(not(feature = "hardware"))]
pub mod mock {
    pub struct MockOutputPin {
        gpio_id: u8,
    }

    impl MockOutputPin {
        pub fn new(id: u8) -> Self {
            Self { gpio_id: id }
        }

        pub fn set_high(&mut self) {
            //println!("set {} pin to high", self.gpio_id);
        }

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
            return true;
        }

        pub fn is_low(&self) -> bool {
            //println!("checking is high, this is unpressed state");
            return true;
        }
    }
}
