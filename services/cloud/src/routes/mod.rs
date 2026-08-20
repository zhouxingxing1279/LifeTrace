//! HTTP routes.

pub mod assistant;
pub mod auth;
pub mod beecount;
pub mod beecount_account;
pub mod beecount_attachments;
pub mod beecount_compat;
pub mod beecount_stats;
pub mod beecount_web;
pub mod beecount_ws;
pub mod files;
pub mod finance;
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
/// Database-backed Cloud deployments expose finance only through the BeeCount
/// surfaces. LifeTrace Web reads the same PostgreSQL BeeCount-compatible entity
/// store used by the stock BeeCount client. The historical LifeTrace finance
/// CRUD routes remain mounted solely for the in-memory protocol test harness.
pub fn router(state: AppState) -> Router<AppState> {
    let in_memory_protocol_harness = !state.database_enabled;
    let mut router = Router::<AppState>::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(beecount_web::router())
        .merge(beecount_account::router())
        .merge(beecount_attachments::router(
            state.config.beecount_attachment_max_upload_bytes,
        ))
        .merge(beecount_compat::router())
        .merge(beecount_stats::router())
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
        .merge(sync::router());

    if in_memory_protocol_harness {
        router = router.merge(finance::router());
    }

    router
}
