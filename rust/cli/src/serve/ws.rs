//! WebSocket handler for real-time BLE notification forwarding.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};

use super::state::AppState;

/// WebSocket upgrade handler at `/ws`.
///
/// On connect, subscribes to the broadcast channel and forwards all
/// BLE notifications as JSON messages to the WebSocket client.
pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an established WebSocket connection.
async fn handle_socket(socket: WebSocket, state: AppState) {
    let mut rx = state.notifications_tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task to forward broadcast notifications to the WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Consume incoming messages (pings/pongs/close) to keep the connection alive
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(_msg)) = receiver.next().await {
            // We don't process incoming messages from the client;
            // this loop just keeps the connection alive and handles close frames.
        }
    });

    // Wait for either task to finish (client disconnect or broadcast channel close)
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}
