//! HTTP routes.

pub mod devices;
pub mod finance;
pub mod health;
pub mod sync;

use axum::http::HeaderMap;
use axum::Router;
use lifetrace_contracts::UserId;

use crate::state::AppState;

/// Assemble all routes into one router.
pub fn router(_state: AppState) -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(finance::router())
        .merge(sync::router())
        .merge(devices::router())
}

/// Resolve the acting user for a request.
///
/// EPIC-03 keeps an auth placeholder: the user is taken from the
/// `X-LifeTrace-User` header (development only) or falls back to
/// `dev-user`. EPIC-04 replaces this with real token authentication; the
/// per-user store already enforces isolation between user ids.
pub fn resolve_user(headers: &HeaderMap) -> UserId {
    headers
        .get("x-lifetrace-user")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(UserId::new)
        .unwrap_or_else(|| UserId::new("dev-user"))
}
