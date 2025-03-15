#[cfg(not(feature = "hardware"))]
use mock::mock::{MockInputPin, MockOutputPin};
use more_asserts::assert_ge;
#[cfg(feature = "hardware")]
use rppal::gpio::{Gpio, InputPin, OutputPin};
use std::{
    error::Error,
    fmt::Display,
    thread,
    time::{Duration, Instant},
};
use tokio::time::sleep;

mod mock;

const LED_PINS: [u8; 2] = [17, 22];
const MOTOR_PINS: [u8; 4] = [18, 23, 24, 25];
const BUTTON_PIN: u8 = 21;
const ULTRASONIC_TRIGGER_PIN: u8 = 16;
const ULTRASONIC_ECHO_PIN: u8 = 20;

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
    pub fn new(pin: u8) -> Self {
        assert!(LED_PINS.contains(&pin));
        let gpio = Gpio::new().unwrap();
        let pin = gpio.get(pin).unwrap().into_output();
        Self { pin }
    }
}

#[cfg(not(feature = "hardware"))]
pub struct LED {
    pin: MockOutputPin,
}

#[cfg(not(feature = "hardware"))]
impl LED {
    pub fn new(pin: u8) -> Self {
        assert!(LED_PINS.contains(&pin));
        let pin = MockOutputPin::new(pin);
        Self { pin }
    }
}

impl LED {
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
        sleep(Duration::from_millis(50)).await;
        if self.pin.is_high() {
            return false;
        }
        println!("good, returning true");
        true
    }
}

#[cfg(feature = "hardware")]
pub struct UltrasonicSensor {
    pub trigger_pin: OutputPin,
    pub echo_pin: InputPin,
}
#[cfg(feature = "hardware")]
impl UltrasonicSensor {
    pub fn new() -> Self {
        let gpio = Gpio::new().unwrap();
        Self {
            trigger_pin: gpio.get(ULTRASONIC_TRIGGER_PIN).unwrap().into_output(),
            echo_pin: gpio.get(ULTRASONIC_ECHO_PIN).unwrap().into_input(),
        }
    }
}

#[cfg(not(feature = "hardware"))]
pub struct UltrasonicSensor {
    pub trigger_pin: MockOutputPin,
    pub echo_pin: MockInputPin,
}

#[cfg(not(feature = "hardware"))]
impl UltrasonicSensor {
    pub fn new() -> Self {
        Self {
            trigger_pin: MockOutputPin::new(ULTRASONIC_TRIGGER_PIN),
            echo_pin: MockInputPin::new(ULTRASONIC_ECHO_PIN),
        }
    }
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
        Ok(Duration::from(start_time.elapsed()))
    }

    pub fn read_distance(&mut self) -> Result<SonicDistance, ReadEchoError> {
        self.send_trigger(10);
        let echo_time = self.read_echo()?;
        Ok(SonicDistance::from(echo_time))
    }
}
