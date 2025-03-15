use std::{
    env,
    error::Error,
    fmt,
    io::{Write, stdin, stdout},
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use once_cell::sync::Lazy;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::sleep,
};

use crate::{
    STATE,
    rpi::{Button, LED, LEDState, MotorDirection, StepMotor},
};

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

impl From<LockAction> for LEDState {
    fn from(action: LockAction) -> LEDState {
        match action {
            LockAction::Lock => LEDState::Off,
            LockAction::Unlock => LEDState::On,
        }
    }
}

pub struct Lock {
    led: LED,
    motor: StepMotor,
}

impl Lock {
    pub fn new() -> Self {
        Self {
            led: LED::new(),
            motor: StepMotor::new(),
        }
    }

    pub async fn act(&mut self, action: &LockAction) {
        println!("Currently taking {:?} action", action);
        self.led.toggle(action.clone().into());
        for _ in 0..512 {
            self.motor.take_step(action.clone().into(), 3).await;
        }
        println!("done with {:?} action", action);
    }
}

pub async fn expose_button_interface(lock_tx: Arc<tokio::sync::mpsc::Sender<LockInstruction>>) {
    let button = Button::new();
    loop {
        if button.check_is_pressed_debounced().await {
            if let Err(e) =
                lock_tx.send_instruction(LockInstruction::Reverse(InstructionSource::Button))
            {
                println!("{}", e)
            };
        }
        sleep(Duration::from_millis(200)).await;
    }
}

pub async fn handle_lock_instruction(mut rx: Receiver<LockInstruction>) {
    let mut lock = Lock::new();
    loop {
        // main poll, lets us see if there's a message ready without actually consuming
        if rx.is_empty() {
            sleep(Duration::from_millis(100)).await;
            continue;
        }

        let _use_lock = LOCK_IN_USE.lock();

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
                if let Err(_) = self.try_send(instruction.clone()) {
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
