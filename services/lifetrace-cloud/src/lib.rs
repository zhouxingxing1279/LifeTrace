//! LifeTrace EPIC-03 sync server.
//!
//! The production path is backed by PostgreSQL through SQLx. The protocol
//! surface remains the v1 contract defined in `lifetrace-contracts`.

pub mod auth;
pub mod config;
pub mod error;
pub mod postgres_repository;
pub mod repository;
pub mod routes;
pub mod state;
pub mod store;
pub mod sync;

pub use config::Config;
pub use error::ApiError;
pub use state::{AppState, StartupError};

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

/// Build the full application router.
pub fn app(state: AppState) -> Router {
    let cors = if state.config.cors_allowed_origins.is_empty() {
        CorsLayer::new()
    } else {
        let origins: Vec<axum::http::HeaderValue> = state
            .config
            .cors_allowed_origins
            .iter()
            .filter_map(|value| value.parse().ok())
            .collect();
        CorsLayer::new().allow_origin(origins)
    };
    routes::router(state.clone())
        .with_state(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors)
}
