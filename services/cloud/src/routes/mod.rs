//! HTTP routes.

pub mod assistant;
pub mod auth;
pub mod finance;
pub mod finance_capture;
pub mod health;
pub mod mail;
pub mod mail_attachment;
pub mod mail_list;
pub mod meta;
pub mod privacy;
pub mod sync;
pub mod web_auth;

use axum::Router;

use crate::state::AppState;

/// Assemble all routes into one router.
pub fn router(_state: AppState) -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(web_auth::router())
        .merge(assistant::router())
        .merge(meta::router())
        .merge(finance::router())
        .merge(finance_capture::router())
        .merge(mail::router())
        .merge(mail_attachment::router())
        .merge(mail_list::router())
        .merge(privacy::router())
        .merge(sync::router())
}
