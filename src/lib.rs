use std::sync::Arc;

use axum::{routing::get, Router};
use clap::ValueEnum;
use serde::Deserialize;
use tokio::sync::RwLock;

pub mod contracts;
pub mod da;
pub mod health;
pub mod kms;
pub mod operator;
pub mod register;
pub mod run;
pub mod socketio;

#[derive(Deserialize, Clone, Copy, Debug, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Kms {
    Local,
    Aws,
}

pub fn routes(socket_io_connected: Arc<RwLock<bool>>) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .with_state(socket_io_connected)
}
