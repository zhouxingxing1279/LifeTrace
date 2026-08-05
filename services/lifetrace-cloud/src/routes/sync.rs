//! Sync protocol v1 endpoints.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use lifetrace_contracts::sync::v1::{
    CapabilitiesResponseV1, PullRequestV1, PullResponseV1, PushRequestV1, PushResponseV1,
    SnapshotRequestV1, SnapshotResponseV1,
};

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/sync/capabilities", get(capabilities))
        .route("/api/v1/sync/push", post(push))
        .route("/api/v1/sync/pull", post(pull))
        .route("/api/v1/sync/snapshot", post(snapshot))
}

async fn capabilities(State(state): State<AppState>) -> Json<CapabilitiesResponseV1> {
    let store = state.store.read().expect("store lock poisoned");
    Json(store.capabilities())
}

async fn push(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<PushRequestV1>,
) -> Result<Json<PushResponseV1>, ApiError> {
    let mut store = state.store.write().expect("store lock poisoned");
    store.push(&principal.user_id, &request).map(Json)
}

async fn pull(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<PullRequestV1>,
) -> Result<Json<PullResponseV1>, ApiError> {
    let store = state.store.read().expect("store lock poisoned");
    store.pull(&principal.user_id, &request).map(Json)
}

async fn snapshot(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<SnapshotRequestV1>,
) -> Result<Json<SnapshotResponseV1>, ApiError> {
    let mut store = state.store.write().expect("store lock poisoned");
    store.snapshot(&principal.user_id, &request).map(Json)
}
