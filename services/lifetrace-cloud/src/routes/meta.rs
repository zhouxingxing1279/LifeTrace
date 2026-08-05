//! Service metadata endpoint.

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/api/v1/meta/version", get(version))
}

async fn version() -> Json<Value> {
    Json(json!({
        "name": "lifetrace-cloud",
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": lifetrace_contracts::PROTOCOL_VERSION,
        "schemaVersion": lifetrace_contracts::SCHEMA_VERSION,
    }))
}
