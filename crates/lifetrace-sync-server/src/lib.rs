//! LifeTrace EPIC-03 sync server.
//!
//! An Axum prototype implementing the sync protocol v1 contract defined in
//! `lifetrace-contracts`:
//!
//! - `GET  /api/v1/sync/capabilities`
//! - `POST /api/v1/sync/push`
//! - `POST /api/v1/sync/pull`
//! - `POST /api/v1/sync/snapshot`
//!
//! Storage is currently an in-memory per-user state machine (the same
//! semantics validated by the reference testkit), behind a simple store
//! module so a PostgreSQL-backed implementation can replace it later.

pub mod config;
pub mod error;
pub mod routes;
pub mod state;
pub mod store;

pub use config::Config;
pub use error::ApiError;
pub use state::AppState;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

/// Build the full application router.
pub fn app(state: AppState) -> Router {
    routes::router(state.clone())
        .with_state(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(CorsLayer::permissive())
}
