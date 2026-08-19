//! HTTP routes.

pub mod assistant;
pub mod auth;
pub mod beecount;
pub mod beecount_account;
pub mod beecount_attachments;
pub mod beecount_compat;
pub mod beecount_ws;
pub mod files;
pub mod health;
pub mod mail;
pub mod mail_attachment;
pub mod mail_list;
pub mod meta;
pub mod photo_challenge;
pub mod photo_challenge_desktop;
pub mod photo_staging;
pub mod privacy;
pub mod sync;
pub mod web_auth;

use axum::Router;

use crate::state::AppState;

/// Assemble all routes into one router.
///
/// Finance is intentionally served only through the BeeCount surfaces. The
/// former LifeTrace-specific finance router is no longer mounted, preventing a
/// second finance API/data path from diverging from BeeCount.
pub fn router(state: AppState) -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(beecount::router())
        .merge(beecount_account::router())
        .merge(beecount_attachments::router(
            state.config.beecount_attachment_max_upload_bytes,
        ))
        .merge(beecount_compat::router())
        .merge(beecount_ws::router())
        .merge(web_auth::router())
        .merge(assistant::router())
        .merge(meta::router())
        .merge(files::router())
        .merge(photo_staging::router())
        .merge(photo_challenge::router())
        .merge(photo_challenge_desktop::router())
        .merge(mail::router())
        .merge(mail_attachment::router())
        .merge(mail_list::router())
        .merge(privacy::router())
        .merge(sync::router())
}
