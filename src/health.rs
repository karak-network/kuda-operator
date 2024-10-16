use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl IntoResponse for HealthCheck {
    fn into_response(self) -> Response {
        let code = match self.status {
            Status::Ok | Status::Warn => StatusCode::OK,
            Status::Fail => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, Json(self)).into_response()
    }
}

#[tracing::instrument]
pub async fn health_check(State(socket_io_connected): State<Arc<RwLock<bool>>>) -> HealthCheck {
    let socket_io_connected = socket_io_connected.read().await;
    if *socket_io_connected {
        HealthCheck {
            status: Status::Ok,
            description: None,
        }
    } else {
        HealthCheck {
            status: Status::Fail,
            description: Some("Socket IO not connected".to_string()),
        }
    }
}
