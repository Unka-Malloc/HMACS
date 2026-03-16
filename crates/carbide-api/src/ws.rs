use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use axum::Router;
use crate::state::AppState;

pub fn ws_router(state: AppState) -> Router {
    Router::new()
        .route("/", axum::routing::get(ws_handler))
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    TaskUpdated { task_id: String, status: String },
    BidReceived { task_id: String, bid_id: String },
    LeaseUpdate { lease_id: String, units_consumed: String },
    BalanceChanged { asset: String, available: String, frozen: String },
    Ping,
    Pong,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket connection established");

    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!(message = %text, "Received WebSocket message");
                let response = serde_json::to_string(&WsMessage::Pong)
                    .unwrap_or_else(|_| r#"{"type":"pong"}"#.to_string());
                if socket.send(Message::Text(response)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Ping(data)) => {
                if socket.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by client");
                break;
            }
            Err(e) => {
                warn!(error = %e, "WebSocket error");
                break;
            }
            _ => {}
        }
    }

    info!("WebSocket connection terminated");
}
