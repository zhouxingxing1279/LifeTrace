//! Health check endpoint.

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/healthz", get(health))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
