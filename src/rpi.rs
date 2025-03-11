#[cfg(not(feature = "hardware"))]
use mock::mock::{MockInputPin, MockOutputPin};
use more_asserts::assert_ge;
#[cfg(feature = "hardware")]
use rppal::gpio::{Gpio, InputPin, OutputPin};
use std::time::Duration;
use tokio::time::sleep;

mod mock;

const LED_PIN: u8 = 17;
const MOTOR_PINS: [u8; 4] = [18, 23, 24, 25];
const BUTTON_PIN: u8 = 21;

#[derive(Debug)]
pub enum LEDState {
    On,
    Off,
}

#[cfg(feature = "hardware")]
pub struct LED {
    pin: OutputPin,
}

#[cfg(feature = "hardware")]
impl LED {
    pub fn new() -> Self {
        let gpio = Gpio::new().unwrap();
        let pin = gpio.get(LED_PIN).unwrap().into_output();
        Self { pin }
    }
}

#[cfg(not(feature = "hardware"))]
pub struct LED {
    pin: MockOutputPin,
}

#[cfg(not(feature = "hardware"))]
impl LED {
    pub fn new() -> Self {
        let pin = MockOutputPin::new(LED_PIN);
        Self { pin }
    }
}

impl LED {
    pub fn toggle(&mut self, state: LEDState) {
        println!("Toggling LED {:?}", state);
        match state {
            LEDState::On => self.pin.set_high(),
            LEDState::Off => self.pin.set_low(),
        }
    }
}

const CCW_STEP: [u8; 4] = [0b0001, 0b0010, 0b0100, 0b1000];
const CW_STEP: [u8; 4] = [0b1000, 0b0100, 0b0010, 0b0001];

#[derive(Debug, Clone)]
pub enum MotorDirection {
    Clockwise,
    CounterClockwise,
}

impl From<MotorDirection> for [u8; 4] {
    fn from(dir: MotorDirection) -> [u8; 4] {
        match dir {
            MotorDirection::Clockwise => CW_STEP,
            MotorDirection::CounterClockwise => CCW_STEP,
        }
    }
}

#[cfg(feature = "hardware")]
pub struct StepMotor {
    motor_pins: Vec<OutputPin>,
}
#[cfg(feature = "hardware")]
impl StepMotor {
    pub fn new() -> Self {
        let gpio = Gpio::new().unwrap();
        Self {
            motor_pins: MOTOR_PINS
                .iter()
                .map(|&pin| gpio.get(pin).unwrap().into_output())
                .collect(),
        }
    }
}

#[cfg(not(feature = "hardware"))]
pub struct StepMotor {
    motor_pins: Vec<MockOutputPin>,
}

#[cfg(not(feature = "hardware"))]
impl StepMotor {
    pub fn new() -> Self {
        Self {
            motor_pins: MOTOR_PINS
                .iter()
                .map(|&pin| MockOutputPin::new(pin))
                .collect(),
        }
    }
}

impl StepMotor {
    pub async fn take_step(&mut self, direction: MotorDirection, ms: u64) {
        assert_ge!(ms, 3);

        let stepper: [u8; 4] = direction.into();
        for i in 0..4 {
            for j in 0..4 {
                match stepper[i] == 1 << j {
                    true => self.motor_pins[j].set_high(),
                    false => self.motor_pins[j].set_low(),
                }
            }
            sleep(Duration::from_millis(ms)).await;
        }
    }
}

#[cfg(feature = "hardware")]
pub struct Button {
    pin: InputPin,
}

#[cfg(feature = "hardware")]
impl Button {
    pub fn new() -> Self {
        let gpio = Gpio::new().unwrap();
        Self {
            pin: gpio.get(BUTTON_PIN).unwrap().into_input_pullup(),
        }
    }
}

#[cfg(not(feature = "hardware"))]
pub struct Button {
    pin: MockInputPin,
}

#[cfg(not(feature = "hardware"))]
impl Button {
    pub fn new() -> Self {
        Self {
            pin: MockInputPin::new(BUTTON_PIN),
        }
    }
}

impl Button {
    pub async fn check_is_pressed_debounced(&self) -> bool {
        if self.pin.is_high() {
            return false;
        };
        println!("debouncing");
        sleep(Duration::from_millis(50)).await;
        if self.pin.is_high() {
            return false;
        }
        println!("good, returning true");
        true
    }
}
