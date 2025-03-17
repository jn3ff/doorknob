#[cfg(not(feature = "hardware"))]
mod mock;

#[cfg(not(feature = "hardware"))]
mod gpio {
    pub use super::mock::{MockGpio as Gpio, MockInputPin as InputPin, MockOutputPin as OutputPin};
}

#[cfg(feature = "hardware")]
mod gpio {
    pub use rppal::gpio::{Gpio, InputPin, OutputPin};
}

use gpio::{Gpio, InputPin, OutputPin};

use more_asserts::assert_ge;
use std::{
    error::Error,
    fmt::Display,
    thread,
    time::{Duration, Instant},
};
use tokio::time::sleep;

const LED_PINS: [u8; 2] = [17, 22];
const MOTOR_DIR_PIN: u8 = 23;
const MOTOR_STEP_PIN: u8 = 24;
const MOTOR_ENABLE_PIN: u8 = 18;
const MOTOR_SLEEP_PIN: u8 = 4;
const BUTTON_PIN: u8 = 21;
const ULTRASONIC_TRIGGER_PIN: u8 = 16;
const ULTRASONIC_ECHO_PIN: u8 = 20;

#[derive(Debug)]
pub enum LEDState {
    On,
    Off,
}

pub struct LED {
    pin: OutputPin,
}

impl LED {
    pub fn new(pin: u8) -> Self {
        assert!(LED_PINS.contains(&pin));
        let gpio = Gpio::new().unwrap();
        let pin = gpio.get(pin).unwrap().into_output();
        Self { pin }
    }

    pub fn set_state(&mut self, state: LEDState) {
        match state {
            LEDState::On => self.pin.set_high(),
            LEDState::Off => self.pin.set_low(),
        }
    }

    pub fn with_state(mut self, state: LEDState) -> Self {
        self.set_state(state);
        self
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

pub struct StepMotor {
    pub dir_pin: OutputPin,
    pub step_pin: OutputPin,
    enable_pin: OutputPin,
    sleep_pin: OutputPin,
}

impl StepMotor {
    pub fn new() -> Self {
        let gpio = Gpio::new().unwrap();
        let mut t = Self {
            dir_pin: gpio.get(MOTOR_DIR_PIN).unwrap().into_output(),
            step_pin: gpio.get(MOTOR_STEP_PIN).unwrap().into_output(),
            enable_pin: gpio.get(MOTOR_ENABLE_PIN).unwrap().into_output(),
            sleep_pin: gpio.get(MOTOR_SLEEP_PIN).unwrap().into_output(),
        };

        t.step_pin.set_low();
        t.deactivate();
        t
    }

    pub fn activate(&mut self) {
        self.sleep_pin.set_high();
        std::thread::sleep(Duration::from_millis(3));
        self.enable_pin.set_low();
        std::thread::sleep(Duration::from_millis(3));
    }

    pub fn deactivate(&mut self) {
        self.sleep_pin.set_low();
        std::thread::sleep(Duration::from_millis(3));
        self.enable_pin.set_high();
        std::thread::sleep(Duration::from_millis(3));
    }

    pub fn set_direction(&mut self, direction: MotorDirection) {
        match direction {
            MotorDirection::Clockwise => self.dir_pin.set_high(),
            MotorDirection::CounterClockwise => self.dir_pin.set_low(),
        }
        std::thread::sleep(Duration::from_millis(5)); // thinking maybe the lock is moving before dir is set
    }

    pub fn take_step(&mut self, step_delay: Duration) {
        assert_ge!(step_delay.as_millis(), 3);
        self.step_pin.set_high();
        std::thread::sleep(Duration::from_micros(50)); //at least 1.9us
        self.step_pin.set_low();
        std::thread::sleep(step_delay);
    }
}

pub struct Button {
    pin: InputPin,
}

impl Button {
    pub fn new() -> Self {
        let gpio = Gpio::new().unwrap();
        Self {
            pin: gpio.get(BUTTON_PIN).unwrap().into_input_pullup(),
        }
    }
    pub async fn check_is_pressed_debounced(&self) -> bool {
        if self.pin.is_high() {
            return false;
        };
        sleep(Duration::from_millis(50)).await;
        if self.pin.is_high() {
            return false;
        }
        println!("good, returning true");
        true
    }
}

pub struct UltrasonicSensor {
    pub trigger_pin: OutputPin,
    pub echo_pin: InputPin,
}

#[derive(Debug, Clone)]
pub struct SonicDistance {
    value_mm: f64, //
}

impl SonicDistance {
    pub fn as_cm_u64(&self) -> u64 {
        (self.value_mm.round() / 10.0) as u64
    }
    pub fn as_cm_f64(&self) -> f64 {
        self.value_mm.round() / 10.0
    }
}
impl From<Duration> for SonicDistance {
    fn from(duration: Duration) -> Self {
        // the duration of an echo has some properties we need to encode here
        // the formula is d = vt / 2 where d is distance, v is speed of sound, t is time. the 2 is obvious
        let v: f64 = 343.0; // speed of sound in meters per second
        let t = duration.as_secs_f64();

        let distance_m = v * t / 2.0;
        SonicDistance {
            value_mm: distance_m * 1000.0,
        }
    }
}

#[derive(Debug)]
pub enum ReadEchoError {
    Timeout,
}

impl Display for ReadEchoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadEchoError::Timeout => write!(f, "Echo read timed out"),
        }
    }
}

impl Error for ReadEchoError {}

impl UltrasonicSensor {
    pub fn new() -> Self {
        let gpio = Gpio::new().unwrap();
        Self {
            trigger_pin: gpio.get(ULTRASONIC_TRIGGER_PIN).unwrap().into_output(),
            echo_pin: gpio.get(ULTRASONIC_ECHO_PIN).unwrap().into_input(),
        }
    }
    fn send_trigger(&mut self, micros: u64) {
        self.trigger_pin.set_high();
        thread::sleep(Duration::from_micros(micros));
        self.trigger_pin.set_low();
    }
    // needs to be called immediately after send_trigger, or measurement will be wrong
    fn read_echo(&self) -> Result<Duration, ReadEchoError> {
        let pre = Instant::now();
        let timeout = Duration::from_millis(100);

        while self.echo_pin.is_low() {
            if pre.elapsed() > timeout {
                // timeout reached
                println!("read echo timed out");
                return Err(ReadEchoError::Timeout);
            }
        }
        let start_time = Instant::now();
        while self.echo_pin.is_high() {
            // do nothing, let time pass
        }
        Ok(start_time.elapsed())
    }

    pub fn read_distance(&mut self) -> Result<SonicDistance, ReadEchoError> {
        self.send_trigger(10);
        let echo_time = self.read_echo()?;
        Ok(SonicDistance::from(echo_time))
    }
}

impl Default for Button {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StepMotor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for UltrasonicSensor {
    fn default() -> Self {
        Self::new()
    }
}
