//! Stock BeeCount ledger-statistics surface under the internal cutover prefix.
//!
//! Production Caddy rewrites `/api/v1/read/ledgers/{ledger_id}/stats` here.
//! Statistics are computed from the same PostgreSQL entity and attachment stores
//! used by BeeCount sync, so the stock client never needs a separate backend.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::ErrorCode;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::beecount_collaboration::resolve_ledger_access;
use crate::beecount_sync::BeeCountSyncService;
use crate::error::ApiError;
use crate::state::AppState;

const PREFIX: &str = "/api/v1/integrations/beecount/compat/read/ledgers";

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route(&format!("{PREFIX}/{{ledger_id}}/stats"), get(stats))
}

#[derive(Debug, Serialize)]
struct BeeCountLedgerStats {
    transaction_count: i64,
    transaction_total: i64,
    transaction_attachment_count: i64,
    transaction_attachment_total: i64,
    budget_count: i64,
    budget_total: i64,
    account_count: i64,
    account_total: i64,
    category_count: i64,
    category_total: i64,
    category_attachment_count: i64,
    category_attachment_total: i64,
    tag_count: i64,
    tag_total: i64,
}

async fn stats(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(ledger_id): Path<String>,
) -> Result<Json<BeeCountLedgerStats>, ApiError> {
    authorize(&principal)?;
    if ledger_id.is_empty() || ledger_id.len() > 256 {
        return Err(invalid("invalid BeeCount ledger id"));
    }

    // `full` is the canonical access/existence check used by the BeeCount sync
    // boundary. Do not leak statistics for a ledger that the principal cannot
    // read or that no longer has an active ledger entity.
    let full = BeeCountSyncService::new(state.pool.clone())
        .full(&principal.user_id, &ledger_id)
        .await?;
    if full.snapshot.is_none() {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "BeeCount ledger not found",
            StatusCode::NOT_FOUND,
        ));
    }

    let actor_uuid = Uuid::parse_str(principal.user_id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InternalError,
            "invalid BeeCount user identity",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let access = resolve_ledger_access(&state.pool, actor_uuid, &ledger_id, false).await?;

    let (transaction_count, budget_count): (i64, i64) = sqlx::query_as(
        "SELECT \
           COUNT(*) FILTER (WHERE entity_type='finance.transaction')::BIGINT, \
           COUNT(*) FILTER (WHERE entity_type='finance.budget')::BIGINT \
         FROM sync_entities \
         WHERE user_id=$1 AND is_deleted=FALSE \
           AND entity_type IN ('finance.transaction','finance.budget') \
           AND payload->>'beecountLedgerId'=$2",
    )
    .bind(access.storage_user_id)
    .bind(&ledger_id)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;

    // Totals include every ledger visible to this account. Private/native
    // entities owned directly by the user are included even if they pre-date
    // the BeeCount collaboration registry; shared-ledger entities are admitted
    // only through the membership + storage-owner mapping.
    let (transaction_total, budget_total): (i64, i64) = sqlx::query_as(
        "SELECT \
           COUNT(*) FILTER (WHERE e.entity_type='finance.transaction')::BIGINT, \
           COUNT(*) FILTER (WHERE e.entity_type='finance.budget')::BIGINT \
         FROM sync_entities e \
         WHERE e.is_deleted=FALSE \
           AND e.entity_type IN ('finance.transaction','finance.budget') \
           AND (e.user_id=$1 OR EXISTS ( \
             SELECT 1 \
             FROM beecount_ledger_members m \
             JOIN beecount_shared_ledgers s ON s.ledger_id=m.ledger_id \
             WHERE m.user_id=$1 AND s.storage_user_id=e.user_id \
               AND e.payload->>'beecountLedgerId'=m.ledger_id \
           ))",
    )
    .bind(actor_uuid)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;

    let (account_total, category_total, tag_total): (i64, i64, i64) = sqlx::query_as(
        "SELECT \
           COUNT(*) FILTER (WHERE entity_type='finance.account')::BIGINT, \
           COUNT(*) FILTER (WHERE entity_type='finance.category')::BIGINT, \
           COUNT(*) FILTER (WHERE entity_type='finance.tag')::BIGINT \
         FROM sync_entities \
         WHERE user_id=$1 AND is_deleted=FALSE \
           AND entity_type IN ('finance.account','finance.category','finance.tag')",
    )
    .bind(actor_uuid)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;

    let transaction_attachment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM cloud_file_blobs \
         WHERE user_id=$1 AND ledger_id=$2 \
           AND attachment_kind='transaction_attachment'",
    )
    .bind(access.storage_user_id)
    .bind(&ledger_id)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;

    let transaction_attachment_total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT \
         FROM cloud_file_blobs b \
         WHERE b.attachment_kind='transaction_attachment' \
           AND (b.user_id=$1 OR EXISTS ( \
             SELECT 1 \
             FROM beecount_ledger_members m \
             JOIN beecount_shared_ledgers s ON s.ledger_id=m.ledger_id \
             WHERE m.user_id=$1 AND s.storage_user_id=b.user_id \
               AND m.ledger_id=b.ledger_id \
           ))",
    )
    .bind(actor_uuid)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;

    let category_attachment_total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM cloud_file_blobs \
         WHERE user_id=$1 AND attachment_kind='category_icon'",
    )
    .bind(actor_uuid)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;

    Ok(Json(BeeCountLedgerStats {
        transaction_count,
        transaction_total,
        transaction_attachment_count,
        transaction_attachment_total,
        budget_count,
        budget_total,
        account_count: account_total,
        account_total,
        category_count: category_total,
        category_total,
        category_attachment_count: category_attachment_total,
        category_attachment_total,
        tag_count: tag_total,
        tag_total,
    }))
}

fn authorize(principal: &AuthenticatedPrincipal) -> Result<(), ApiError> {
    if principal.app_id.as_str() != AppId::BEECOUNT {
        return Err(ApiError::new(
            ErrorCode::AuthInvalid,
            "BeeCount session required",
            StatusCode::UNAUTHORIZED,
        ));
    }
    principal.require_scope("sync:read")
}

fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

fn db_error(_error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "BeeCount ledger stats temporarily unavailable",
        StatusCode::SERVICE_UNAVAILABLE,
    )
}
