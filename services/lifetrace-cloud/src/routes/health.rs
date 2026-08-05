//! Liveness and readiness endpoints.

use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio::time::timeout;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

async fn live() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let config_valid = state.config.validate().is_ok();
    let mut checks = serde_json::Map::new();
    checks.insert("configValid".to_owned(), json!(config_valid));

    if !state.database_enabled {
        checks.insert("storage".to_owned(), json!("memory"));
        return if config_valid {
            (
                StatusCode::OK,
                Json(json!({ "status": "ready", "checks": checks })),
            )
        } else {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "not_ready", "checks": checks })),
            )
        };
    }

    checks.insert("storage".to_owned(), json!("postgresql"));
    let database_ready = matches!(
        timeout(
            Duration::from_secs(2),
            sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool),
        )
        .await,
        Ok(Ok(1))
    );
    checks.insert("postgresql".to_owned(), json!(database_ready));

    if config_valid && database_ready {
        (
            StatusCode::OK,
            Json(json!({ "status": "ready", "checks": checks })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "checks": checks })),
        )
    }
}
