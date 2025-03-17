use std::sync::Arc;

use auth::setup_password;
use lock::{LockInstruction, STATE, handle_lock_instruction};
use once_cell::sync::Lazy;
use sensors::{expose_button_interface, expose_closed_detection_interface};
use tokio::{select, sync::mpsc::channel};

pub mod auth;
pub mod lock;
pub mod routes;
pub mod rpi;
pub mod sensors;
pub mod server;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), anyhow::Error> {
    println!("Setting password");
    setup_password().await;
    println!("Validating args");
    Lazy::force(&STATE); // if an env variable is not given for lock state, we need the user to set it

    let (lock_tx, lock_rx) = channel::<LockInstruction>(1); // buffer is flushed after first message is processed
    let arc_lock_tx = Arc::new(lock_tx);
    println!("Initialized lock channels, starting hot threads");

    select! {
        _ = handle_lock_instruction(lock_rx) => {},
        _ = server::run_app(Arc::clone(&arc_lock_tx)) => {},
        _ = expose_button_interface(Arc::clone(&arc_lock_tx)) => {},
        _ = expose_closed_detection_interface(Arc::clone(&arc_lock_tx)) => {}
    };

    Ok(())
}
