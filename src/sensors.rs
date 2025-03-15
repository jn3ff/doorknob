use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{sync::mpsc::Sender, time::sleep};

use crate::{
    lock::{InstructionSource, LockInstruction, LockInstructor},
    rpi::{Button, UltrasonicSensor},
};

pub async fn expose_button_interface(lock_tx: Arc<Sender<LockInstruction>>) {
    let button = Button::new();
    loop {
        if button.check_is_pressed_debounced().await {
            if let Err(e) =
                lock_tx.send_instruction(LockInstruction::Reverse(InstructionSource::Button))
            {
                println!("{}", e)
            };
        }
        sleep(Duration::from_millis(100)).await;
    }
}

pub async fn expose_closed_detection_interface(lock_tx: Arc<Sender<LockInstruction>>) {
    let mut ultrasonic_sensor = UltrasonicSensor::new();
    let mut start_timer = Instant::now();
    let frame_threshold_cm = 6;
    let autolock_interval_sec = 60;
    let err_tolerance = 3;
    let mut errs = 0;
    loop {
        match ultrasonic_sensor.read_distance() {
            Ok(distance) => match distance.as_cm_u64() < frame_threshold_cm {
                true => {
                    if start_timer.elapsed().as_secs() >= autolock_interval_sec {
                        if let Err(e) = lock_tx.send_instruction(LockInstruction::EnsureLocked(
                            InstructionSource::AutoSensor,
                        )) {
                            println!("Autolock instruction dropped. {e}");
                        }
                        sleep(Duration::from_millis(5000)).await; // auto lock can take a long break after triggering
                        start_timer = Instant::now();
                        errs = 0;
                    }
                }
                false => {
                    errs += 1;
                    if errs > err_tolerance {
                        start_timer = Instant::now();
                        errs = 0;
                    }
                }
            },
            Err(_) => {
                println!("Error reading distance from ultrasonic sensor")
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
