//! LifeTrace EPIC-03 sync server.
//!
//! The production path is backed by PostgreSQL through SQLx. The protocol
//! surface remains the v1 contract defined in `lifetrace-contracts`.

pub mod api_rate_limit;
pub mod auth;
pub mod config;
pub mod error;
pub mod mail;
pub mod postgres_repository;
pub mod repository;
pub mod routes;
pub mod state;
pub mod store;
pub mod sync;

pub use config::Config;
pub use error::ApiError;
pub use state::{AppState, StartupError};

use axum::extract::Request;
use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE},
    HeaderName, HeaderValue, Method,
};
use axum::middleware::{from_fn, from_fn_with_state, Next};
use axum::response::Response;
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
    let rate_limit_state = state.clone();
    routes::router(state.clone())
        .with_state(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors)
        .layer(from_fn_with_state(
            rate_limit_state,
            api_rate_limit::enforce,
        ))
        .layer(from_fn(security_headers))
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Cloud serves an API surface, not arbitrary active content. Keep a strict
    // baseline here and let a future separately hosted Web shell define its
    // own CSP if it starts serving HTML from this origin.
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
        ),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=(), usb=()"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-site"),
    );
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

    response
}
