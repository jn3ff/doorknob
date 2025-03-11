use std::sync::Arc;

use auth::setup_password;
use lock::{LockInstruction, LockState, expose_button_interface, handle_lock_instruction};
use once_cell::sync::Lazy;
use tokio::{
    select,
    sync::{Mutex, mpsc::channel},
};

pub mod auth;
pub mod lock;
pub mod routes;
pub mod rpi;
pub mod server;

pub static STATE: Lazy<Mutex<LockState>> = Lazy::new(|| Mutex::new(LockState::from_env()));

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), anyhow::Error> {
    println!("Setting password");
    setup_password();

    println!("Validating args");
    Lazy::force(&STATE);

    let (lock_tx, lock_rx) = channel::<LockInstruction>(1); // buffer is flushed after first message is processed
    let arc_lock_tx = Arc::new(lock_tx);
    println!("Initialized lock channels, starting hot threads");
    select! {
        _ = handle_lock_instruction(lock_rx) => {},
        _ = server::run_app(Arc::clone(&arc_lock_tx)) => {},
        _ = expose_button_interface(Arc::clone(&arc_lock_tx)) => {}
    };

    Ok(())
}
