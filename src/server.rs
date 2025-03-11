use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tokio::sync::mpsc::Sender;

use crate::{
    lock::LockInstruction,
    routes::{door_control, home},
};

pub async fn run_app(lock_tx: Arc<Sender<LockInstruction>>) -> Result<(), anyhow::Error> {
    let app = Router::new()
        .route("/home", get(home))
        .route("/door-control", post(door_control))
        .with_state(lock_tx);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Serving routes on port 3000");
    axum::serve(listener, app).await?;
    Ok(())
}
