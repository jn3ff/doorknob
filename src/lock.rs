use std::{
    env,
    io::{Write, stdin, stdout},
    sync::Arc,
    time::Duration,
};

use tokio::{sync::mpsc::Receiver, time::sleep};

use crate::{
    STATE,
    rpi::{Button, LED, LEDState, MotorDirection, StepMotor},
};

#[derive(Debug, Clone, PartialEq)]
pub enum LockState {
    Unlocked,
    Locked,
}

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
    Filler, // filler is sent just after any instruction to keep the channel full
}

#[derive(Debug, Clone)]
pub enum LockAction {
    Lock,
    Unlock,
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
            if let Err(_) = lock_tx.try_send(LockInstruction::Reverse(InstructionSource::Button)) {
                println!("Button toggle discarded because the queue is full")
            } else {
                let _ = lock_tx.send(LockInstruction::Filler).await;
            }
        }
        sleep(Duration::from_millis(200)).await;
    }
}

pub async fn handle_lock_instruction(mut rx: Receiver<LockInstruction>) {
    let mut lock = Lock::new();
    loop {
        if let Some(instruction) = rx.recv().await {
            println!("Received lock instruction {:?}", instruction);
            let mut state = STATE.lock().await;
            if let Some(action) = state.to_action(instruction) {
                lock.act(&action).await;
                state.set_reverse();
            } else {
                println!("No change to lock state needed")
            }
            while rx.try_recv().is_ok() {
                //flush filler, noop
            }
        }
    }
}
