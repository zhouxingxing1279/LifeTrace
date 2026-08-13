//! BeeCount-compatible WebSocket authentication, heartbeat and sync fan-out.

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use lifetrace_contracts::sync::v1::AppId;
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::state::AppState;

const PATH: &str = "/api/v1/integrations/beecount/compat/ws";

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route(PATH, get(upgrade))
}

#[derive(Debug, Deserialize)]
struct WebSocketQuery {
    #[serde(default)]
    token: String,
}

async fn upgrade(
    State(state): State<AppState>,
    Query(query): Query<WebSocketQuery>,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket.on_upgrade(move |socket| session(socket, state, query.token))
}

async fn session(mut socket: WebSocket, state: AppState, token: String) {
    let Some(principal) = authenticate(&state, &token).await else {
        close_policy_violation(&mut socket).await;
        return;
    };
    let mut receiver = state.beecount_realtime.subscribe();
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) if text.contains("\"ping\"") => {
                        if socket.send(Message::Text("{\"type\":\"pong\"}".into())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(value))) => {
                        if socket.send(Message::Pong(value)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}
                }
            }
            event = receiver.recv() => {
                match event {
                    Ok(event) if event.user_id == principal.user_id.as_str() => {
                        if socket.send(Message::Text(event.payload.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn authenticate(state: &AppState, token: &str) -> Option<AuthenticatedPrincipal> {
    if token.is_empty() {
        return None;
    }
    let authorization = format!("Bearer {token}");
    let principal = state
        .auth
        .authenticate(AuthCredential::Bearer(Some(&authorization)))
        .await
        .ok()?;
    if principal.app_id.as_str() != AppId::BEECOUNT
        || principal.require_scope("sync:write").is_err()
    {
        return None;
    }
    Some(principal)
}

async fn close_policy_violation(socket: &mut WebSocket) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: 1008,
            reason: "invalid BeeCount session".into(),
        })))
        .await;
}
