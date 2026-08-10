//! EPIC-17 application-level API rate limiting.
//!
//! Authentication endpoints already have stricter credential-aware limits from
//! EPIC-04. This layer is the broad abuse-control baseline for the remaining
//! `/api/*` surface and deliberately does not replace endpoint-specific limits.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use lifetrace_contracts::ErrorCode;
use tokio::sync::Mutex;

use crate::auth::security::client_ip as resolve_client_ip;
use crate::{ApiError, Config};

const DEFAULT_REQUESTS_PER_MINUTE: u32 = 600;
const DEFAULT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct ApiRateLimiter {
    inner: Arc<Mutex<HashMap<IpAddr, WindowCounter>>>,
    limit: u32,
    window: Duration,
    trusted_proxy_cidrs: Arc<Vec<String>>,
}

#[derive(Debug, Clone, Copy)]
struct WindowCounter {
    started_at: Instant,
    requests: u32,
}

impl Default for ApiRateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_REQUESTS_PER_MINUTE, DEFAULT_WINDOW)
    }
}

impl ApiRateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self::with_trusted_proxies(limit, window, Vec::new())
    }

    pub fn from_config(config: &Config) -> Self {
        Self::with_trusted_proxies(
            DEFAULT_REQUESTS_PER_MINUTE,
            DEFAULT_WINDOW,
            config.auth_trusted_proxy_cidrs.clone(),
        )
    }

    fn with_trusted_proxies(
        limit: u32,
        window: Duration,
        trusted_proxy_cidrs: Vec<String>,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            limit: limit.max(1),
            window,
            trusted_proxy_cidrs: Arc::new(trusted_proxy_cidrs),
        }
    }

    async fn allow(&self, client: IpAddr, now: Instant) -> bool {
        let mut counters = self.inner.lock().await;
        if counters.len() > 10_000 {
            counters.retain(|_, counter| now.duration_since(counter.started_at) < self.window);
        }
        let counter = counters.entry(client).or_insert(WindowCounter {
            started_at: now,
            requests: 0,
        });
        if now.duration_since(counter.started_at) >= self.window {
            *counter = WindowCounter {
                started_at: now,
                requests: 0,
            };
        }
        if counter.requests >= self.limit {
            return false;
        }
        counter.requests += 1;
        true
    }

    fn request_client_ip(&self, request: &Request) -> IpAddr {
        let peer = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| *address);
        resolve_client_ip(peer, request.headers(), self.trusted_proxy_cidrs.as_ref())
            // `oneshot` integration tests do not have a transport peer. A
            // stable loopback fallback keeps the middleware deterministic.
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }

    fn retry_after_seconds(&self) -> u64 {
        self.window.as_secs().max(1)
    }
}

pub async fn middleware(
    State(limiter): State<ApiRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    if !request.uri().path().starts_with("/api/") {
        return next.run(request).await;
    }

    let client = limiter.request_client_ip(&request);
    if limiter.allow(client, Instant::now()).await {
        return next.run(request).await;
    }

    let mut response = ApiError::new(
        ErrorCode::RateLimited,
        "API request rate limit exceeded",
        StatusCode::TOO_MANY_REQUESTS,
    )
    .into_response();
    if let Ok(value) = HeaderValue::from_str(&limiter.retry_after_seconds().to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[tokio::test]
    async fn limiter_blocks_after_window_quota() {
        let limiter = ApiRateLimiter::new(2, Duration::from_secs(60));
        let client = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10));
        let now = Instant::now();
        assert!(limiter.allow(client, now).await);
        assert!(limiter.allow(client, now).await);
        assert!(!limiter.allow(client, now).await);
    }

    #[tokio::test]
    async fn limiter_isolated_by_client_address() {
        let limiter = ApiRateLimiter::new(1, Duration::from_secs(60));
        let now = Instant::now();
        let first = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10));
        let second = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 11));
        assert!(limiter.allow(first, now).await);
        assert!(!limiter.allow(first, now).await);
        assert!(limiter.allow(second, now).await);
    }

    #[tokio::test]
    async fn limiter_resets_after_window() {
        let window = Duration::from_secs(10);
        let limiter = ApiRateLimiter::new(1, window);
        let client = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10));
        let now = Instant::now();
        assert!(limiter.allow(client, now).await);
        assert!(!limiter.allow(client, now).await);
        assert!(limiter.allow(client, now + window).await);
    }

    #[test]
    fn trusted_proxy_resolution_reuses_auth_security_boundary() {
        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
            headers
        };
        let peer: SocketAddr = "10.0.0.2:1234".parse().unwrap();
        let resolved = resolve_client_ip(Some(peer), &headers, &["10.0.0.0/8".to_owned()]);
        assert_eq!(resolved, Some("203.0.113.9".parse().unwrap()));
    }
}
