//! Process-local API rate limiting for the Cloud HTTP surface.
//!
//! The limiter is intentionally independent from the authentication login
//! limiter. Login keeps its stricter PostgreSQL-backed account/IP policy,
//! while this middleware protects the complete `/api/` surface from request
//! floods before expensive handlers run.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use lifetrace_contracts::ErrorCode;

use crate::auth::security::client_ip;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Default)]
pub struct ApiRateLimiter {
    windows: Arc<Mutex<HashMap<String, Window>>>,
}

#[derive(Debug, Clone, Copy)]
struct Window {
    started_at: Instant,
    requests: usize,
}

impl ApiRateLimiter {
    fn allow(&self, key: String, limit: usize, window: Duration, now: Instant) -> bool {
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if windows.len() > 4096 {
            windows.retain(|_, value| now.duration_since(value.started_at) < window * 2);
        }

        let entry = windows.entry(key).or_insert(Window {
            started_at: now,
            requests: 0,
        });
        if now.duration_since(entry.started_at) >= window {
            *entry = Window {
                started_at: now,
                requests: 0,
            };
        }
        if entry.requests >= limit {
            return false;
        }
        entry.requests += 1;
        true
    }
}

pub async fn enforce(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() == Method::OPTIONS || !request.uri().path().starts_with("/api/") {
        return next.run(request).await;
    }

    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0);
    let key = client_ip(
        peer,
        request.headers(),
        &state.config.auth_trusted_proxy_cidrs,
    )
    .map(|ip| ip.to_string())
    .unwrap_or_else(|| "unknown-client".to_owned());
    let window = Duration::from_secs(state.config.api_rate_limit_window_seconds);

    if state.api_rate_limiter.allow(
        key,
        state.config.api_rate_limit_requests,
        window,
        Instant::now(),
    ) {
        return next.run(request).await;
    }

    let mut response = ApiError::new(
        ErrorCode::RateLimited,
        "too many API requests",
        StatusCode::TOO_MANY_REQUESTS,
    )
    .into_response();
    if let Ok(value) = HeaderValue::from_str(&state.config.api_rate_limit_window_seconds.to_string())
    {
        response.headers_mut().insert(RETRY_AFTER, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_resets_after_window() {
        let limiter = ApiRateLimiter::default();
        let start = Instant::now();
        assert!(limiter.allow("client".to_owned(), 2, Duration::from_secs(10), start));
        assert!(limiter.allow("client".to_owned(), 2, Duration::from_secs(10), start));
        assert!(!limiter.allow("client".to_owned(), 2, Duration::from_secs(10), start));
        assert!(limiter.allow(
            "client".to_owned(),
            2,
            Duration::from_secs(10),
            start + Duration::from_secs(10)
        ));
    }

    #[test]
    fn limiter_isolated_by_client_key() {
        let limiter = ApiRateLimiter::default();
        let now = Instant::now();
        assert!(limiter.allow("a".to_owned(), 1, Duration::from_secs(10), now));
        assert!(!limiter.allow("a".to_owned(), 1, Duration::from_secs(10), now));
        assert!(limiter.allow("b".to_owned(), 1, Duration::from_secs(10), now));
    }
}
