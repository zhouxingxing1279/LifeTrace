//! LifeTrace EPIC-03 sync server.
//!
//! The production path is backed by PostgreSQL through SQLx. The protocol
//! surface remains the v1 contract defined in `lifetrace-contracts`.

pub mod auth;
pub mod config;
pub mod error;
pub mod mail;
pub mod postgres_repository;
pub mod repository;
pub mod routes;
pub mod security;
pub mod state;
pub mod store;
pub mod sync;

pub use config::Config;
pub use error::ApiError;
pub use state::{AppState, StartupError};

use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderName, Method,
};
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
        CorsLayer::new()
            .allow_origin(origins)
            .allow_credentials(true)
            .allow_methods([
                Method::GET,
                Method::HEAD,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                ACCEPT,
                AUTHORIZATION,
                CONTENT_TYPE,
                HeaderName::from_static("x-csrf-token"),
                HeaderName::from_static("x-request-id"),
            ])
    };

    let production = state.config.is_production();
    let mut router = routes::router(state.clone())
        .with_state(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors);

    for layer in security::response_security_layers() {
        router = router.layer(layer);
    }
    if production {
        router = router.layer(security::hsts_layer());
    }
    router
}
