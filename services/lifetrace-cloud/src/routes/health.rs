//! Liveness and readiness endpoints.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

/// Process liveness: does not touch the database.
async fn live() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Readiness: auth provider initialized and configuration valid.
/// (The PostgreSQL connectivity check is added together with the database
/// layer; with in-memory storage the store is always ready.)
async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let mut checks = serde_json::Map::new();
    checks.insert(
        "authProvider".to_owned(),
        json!(state.auth.authenticate(None).is_err()), // initialized (rejects)
    );
    checks.insert("configValid".to_owned(), json!(state.config.validate().is_ok()));
    checks.insert("storage".to_owned(), json!("memory"));
    if state.config.validate().is_ok() {
        (StatusCode::OK, Json(json!({ "status": "ready", "checks": checks })))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({ "status": "not_ready", "checks": checks })))
    }
}
