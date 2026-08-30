//! WebSocket hub (v5 §11): subscription + event push + ping/pong.

use crate::op::ServerEvent;
use crate::state::SharedState;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::StreamExt;
use std::sync::Arc;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: SharedState) {
    state.lease.client_connected();

    // Send a `ready` message with the current status.
    let ready = serde_json::json!({
        "type": "ready",
        "session": state.executor.status(),
    });
    if socket
        .send(Message::Text(ready.to_string().into()))
        .await
        .is_err()
    {
        state.lease.client_disconnected();
        return;
    }

    let mut events = state.events.subscribe();
    let executor = state.executor.clone();
    let lease = Arc::clone(&state.lease);

    loop {
        tokio::select! {
            // Inbound client messages.
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                            match v.get("type").and_then(|t| t.as_str()) {
                                Some("ping") => {
                                    let _ = socket.send(Message::Text("{\"type\":\"pong\"}".into())).await;
                                }
                                Some("subscribe") => {
                                    // Topics are accepted; filtering is a no-op for now
                                    // (all events are pushed to all clients in V0.1).
                                    let _ = v;
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            // Outbound server events.
            ev = events.recv() => {
                match ev {
                    Ok(event) => {
                        let payload = serde_json::json!({
                            "type": "event",
                            "event": event.event_name(),
                            "data": event,
                        });
                        if socket.send(Message::Text(payload.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Client too slow; resend a fresh status snapshot.
                        let payload = serde_json::json!({
                            "type": "event",
                            "event": "target_state_changed",
                            "data": executor.status(),
                        });
                        if socket.send(Message::Text(payload.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    lease.client_disconnected();
    // Grace period before the probe is released (browser refresh support).
    lease.schedule_release(executor);
}

/// Serialize helper used by tests and other handlers.
pub fn event_json(event: &ServerEvent) -> String {
    serde_json::json!({
        "type": "event",
        "event": event.event_name(),
        "data": event,
    })
    .to_string()
}
