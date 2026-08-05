//! HTTP routes.

pub mod finance;
pub mod health;
pub mod meta;
pub mod sync;

use axum::Router;

use crate::state::AppState;

/// Assemble all routes into one router.
pub fn router(_state: AppState) -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(meta::router())
        .merge(finance::router())
        .merge(sync::router())
}
