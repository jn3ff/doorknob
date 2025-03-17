use std::{
    cmp::min,
    env,
    error::Error,
    fmt,
    io::{Write, stdin, stdout},
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use more_asserts::assert_ge;
use once_cell::sync::Lazy;
use tokio::{
    sync::{
        Mutex,
        mpsc::{Receiver, Sender},
    },
    time::sleep,
};

use crate::rpi::{LED, LEDState, MotorDirection, StepMotor};

pub static STATE: Lazy<Mutex<LockState>> = Lazy::new(|| Mutex::new(LockState::from_env()));
static LOCK_IN_USE: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Clone, PartialEq)]
pub enum LockState {
    Unlocked,
    Locked,
}

#[derive(Debug, Clone)]
pub enum InstructionSource {
    Button,
    Api,
    AutoSensor,
}

#[derive(Debug, Clone)]
pub enum LockInstruction {
    EnsureLocked(InstructionSource),
    EnsureUnlocked(InstructionSource),
    Reverse(InstructionSource),
}

#[derive(Debug, Clone)]
pub enum LockAction {
    Lock,
    Unlock,
}

#[derive(Debug)]
pub struct LockInUse;

impl fmt::Display for LockInUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lock in use, collision at {:?}",
            Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
        )
    }
}

impl Error for LockInUse {}

impl LockState {
    pub fn to_action(&self, instruction: LockInstruction) -> Option<LockAction> {
        match (self, instruction) {
            (
                LockState::Unlocked,
                LockInstruction::EnsureLocked(_) | LockInstruction::Reverse(_),
            ) => Some(LockAction::Lock),
            (
                LockState::Locked,
                LockInstruction::EnsureUnlocked(_) | LockInstruction::Reverse(_),
            ) => Some(LockAction::Unlock),
            _ => None,
        }
    }

    pub fn set_reverse(&mut self) {
        match self {
            LockState::Unlocked => *self = LockState::Locked,
            LockState::Locked => *self = LockState::Unlocked,
        }
    }

    pub fn from_env() -> Self {
        let mut state = env::var("LOCK_STATE");
        while state.is_err() {
            println!("Please enter the current state of the lock. Options 'locked' or 'unlocked'");
            let _ = stdout().flush();
            let mut s: String = String::new();
            stdin().read_line(&mut s).unwrap();
            s = s.trim().to_lowercase();
            if s == "unlocked" || s == "locked" {
                state = Ok(s);
            } else {
                println!("You put in '{s}.' Spell an option correctly or no app for you.");
            }
        }

        match state
            .expect("err loop outvalidated now")
            .to_lowercase()
            .as_str()
        {
            "unlocked" => LockState::Unlocked,
            "locked" => LockState::Locked,
            x => panic!(
                "you fucked up the LOCK_STATE environment variable. '{x}' is not valid, use 'locked' or 'unlocked'"
            ),
        }
    }
}

impl From<LockAction> for MotorDirection {
    fn from(action: LockAction) -> MotorDirection {
        match action {
            LockAction::Lock => MotorDirection::CounterClockwise,
            LockAction::Unlock => MotorDirection::Clockwise,
        }
    }
}

const READY_LED_PIN: u8 = 17;
const IN_USE_LED_PIN: u8 = 22;

pub struct Lock {
    ready_led: LED,
    in_use_led: LED,
    motor: StepMotor,
}

impl Lock {
    pub fn new() -> Self {
        Self {
            ready_led: LED::new(READY_LED_PIN).with_state(LEDState::On),
            in_use_led: LED::new(IN_USE_LED_PIN).with_state(LEDState::Off),
            motor: StepMotor::new(),
        }
    }

    pub async fn act(&mut self, action: &LockAction) {
        println!("Currently taking {:?} action", action);
        self.ready_led.set_state(LEDState::Off);
        self.in_use_led.set_state(LEDState::On);
        self.motor.activate();
        self.motor.set_direction(action.clone().into());

        let steps: u64 = 60;
        let target_delay_ms: u64 = 30;
        let base_delay_ms: u64 = 40;
        let acceleration_factor: u64 = 1;
        for step in 0..steps {
            let delay = get_delay(
                step,
                steps,
                acceleration_factor,
                target_delay_ms,
                base_delay_ms,
            );

            println!("delay {}", delay.as_millis());
            self.motor.take_step(delay);
        }
        self.motor.deactivate();
        self.ready_led.set_state(LEDState::On);
        self.in_use_led.set_state(LEDState::Off);
        println!("done with {:?} action", action);
    }
}

fn get_delay(
    step: u64,
    steps: u64,
    accel_factor: u64,
    target_delay_ms: u64,
    base_delay_ms: u64,
) -> Duration {
    assert_ge!(base_delay_ms, target_delay_ms);
    let offset = min((steps - 1) - step, step) * accel_factor;
    let acceleration_space = base_delay_ms - target_delay_ms;
    if offset >= acceleration_space {
        return Duration::from_millis(target_delay_ms);
    }
    Duration::from_millis(base_delay_ms - offset)
}

pub async fn handle_lock_instruction(mut rx: Receiver<LockInstruction>) {
    let mut lock = Lock::new();
    loop {
        // main poll, lets us see if there's a message ready without actually consuming
        if rx.is_empty() {
            sleep(Duration::from_millis(100)).await;
            continue;
        }

        let _lock_guard = LOCK_IN_USE.lock().await;

        match rx.recv().await {
            Some(instruction) => {
                println!("Received lock instruction {:?}", instruction);
                let mut state = STATE.lock().await;
                if let Some(action) = state.to_action(instruction) {
                    lock.act(&action).await;
                    state.set_reverse();
                } else {
                    println!("No change to lock state needed")
                }
            }
            None => println!(
                "Receiving None after passing empty check should never happen. Channel is fucked"
            ),
        }

        println!("Lock use completed. Dropping mutex guard and channel is open for business again")
    }
}

pub trait LockInstructor {
    fn send_instruction(&self, instruction: LockInstruction) -> Result<(), LockInUse>;
}

impl LockInstructor for Arc<Sender<LockInstruction>> {
    fn send_instruction(&self, instruction: LockInstruction) -> Result<(), LockInUse> {
        match LOCK_IN_USE.try_lock() {
            Ok(_) => {
                if self.try_send(instruction.clone()).is_err() {
                    println!(
                        "Unexpected error on try_send instruction {:?}, Justin you fucked up the control flow.",
                        instruction
                    );
                    return Err(LockInUse);
                }
                Ok(())
            }
            Err(_) => Err(LockInUse),
        }
    }
}

impl Default for Lock {
    fn default() -> Self {
        Self::new()
    }
}
