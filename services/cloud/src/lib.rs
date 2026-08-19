//! LifeTrace EPIC-03 sync server.
//!
//! The production path is backed by PostgreSQL through SQLx. The protocol
//! surface remains the v1 contract defined in `lifetrace-contracts`.

pub mod api_rate_limit;
pub mod auth;
pub mod beecount_adapter;
pub mod beecount_attachments;
pub mod beecount_collaboration;
#[allow(clippy::too_many_arguments)]
pub mod beecount_compat;
pub mod beecount_realtime;
#[allow(clippy::unnecessary_map_or)]
pub mod beecount_sync;
pub mod config;
pub mod error;
pub mod mail;
pub mod object_storage;
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
use axum::{middleware, Router};
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

const TAURI_DESKTOP_ORIGINS: &[&str] = &["http://tauri.localhost", "https://tauri.localhost"];

fn cors_origins(config: &Config) -> Vec<axum::http::HeaderValue> {
    let mut values = config.cors_allowed_origins.clone();
    for origin in TAURI_DESKTOP_ORIGINS {
        if !values.iter().any(|value| value == origin) {
            values.push((*origin).to_owned());
        }
    }
    values
        .iter()
        .filter_map(|value| value.parse().ok())
        .collect()
}

/// Build the full application router.
pub fn app(state: AppState) -> Router {
    let origins = cors_origins(&state.config);
    let cors = if origins.is_empty() {
        CorsLayer::new()
    } else {
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
                HeaderName::from_static("x-photo-challenge-key"),
            ])
    };

    let production = state.config.is_production();
    let rate_limiter = api_rate_limit::ApiRateLimiter::from_config(&state.config);
    let mut router = routes::router(state.clone())
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            api_rate_limit::middleware,
        ))
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
