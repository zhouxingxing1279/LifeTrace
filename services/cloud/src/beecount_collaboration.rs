//! Shared-ledger access metadata over the authoritative LifeTrace entity log.

use axum::http::StatusCode;
use lifetrace_contracts::ErrorCode;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::beecount_compat::lifetrace_entity_id;
use crate::error::ApiError;

pub const ROLE_OWNER: &str = "owner";
pub const ROLE_EDITOR: &str = "editor";
pub const MEMBER_LIMIT: i64 = 5;

#[derive(Debug, Clone)]
pub struct LedgerAccess {
    pub storage_user_id: Uuid,
    pub role: String,
}

pub async fn resolve_ledger_access(
    pool: &PgPool,
    actor_user_id: Uuid,
    ledger_id: &str,
    write: bool,
) -> Result<LedgerAccess, ApiError> {
    let mut tx = pool.begin().await.map_err(db_error)?;
    let access = resolve_ledger_access_tx(&mut tx, actor_user_id, ledger_id, write).await?;
    tx.commit().await.map_err(db_error)?;
    Ok(access)
}

pub async fn resolve_ledger_access_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor_user_id: Uuid,
    ledger_id: &str,
    write: bool,
) -> Result<LedgerAccess, ApiError> {
    validate_ledger_id(ledger_id)?;
    if let Some(row) = sqlx::query(
        "SELECT s.storage_user_id,m.role \
         FROM beecount_shared_ledgers s \
         JOIN beecount_ledger_members m ON m.ledger_id=s.ledger_id \
         WHERE s.ledger_id=$1 AND m.user_id=$2 \
           AND EXISTS (SELECT 1 FROM sync_entities e \
             WHERE e.user_id=s.storage_user_id AND e.entity_type='finance.ledger' \
               AND e.is_deleted=FALSE \
               AND (e.entity_id=$3 OR e.payload->>'beecountLedgerId'=$1))",
    )
    .bind(ledger_id)
    .bind(actor_user_id)
    .bind(lifetrace_entity_id(ledger_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?
    {
        let role: String = row.try_get("role").map_err(internal)?;
        if write && !matches!(role.as_str(), ROLE_OWNER | ROLE_EDITOR) {
            return Err(not_found("Ledger not found"));
        }
        return Ok(LedgerAccess {
            storage_user_id: row.try_get("storage_user_id").map_err(internal)?,
            role,
        });
    }

    if ledger_exists_tx(tx, actor_user_id, ledger_id).await? {
        ensure_owner_registry_tx(tx, actor_user_id, ledger_id).await?;
        return Ok(LedgerAccess {
            storage_user_id: actor_user_id,
            role: ROLE_OWNER.to_owned(),
        });
    }
    Err(not_found("Ledger not found"))
}

pub async fn ensure_owner_registry(
    pool: &PgPool,
    owner_user_id: Uuid,
    ledger_id: &str,
) -> Result<(), ApiError> {
    let mut tx = pool.begin().await.map_err(db_error)?;
    if !ledger_exists_tx(&mut tx, owner_user_id, ledger_id).await? {
        return Err(not_found("Ledger not found"));
    }
    ensure_owner_registry_tx(&mut tx, owner_user_id, ledger_id).await?;
    tx.commit().await.map_err(db_error)
}

pub async fn ensure_owner_registry_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_user_id: Uuid,
    ledger_id: &str,
) -> Result<(), ApiError> {
    validate_ledger_id(ledger_id)?;
    let registered_owner: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO beecount_shared_ledgers (ledger_id,storage_user_id) \
         VALUES ($1,$2) ON CONFLICT (ledger_id) DO UPDATE \
         SET storage_user_id=beecount_shared_ledgers.storage_user_id \
         RETURNING storage_user_id",
    )
    .bind(ledger_id)
    .bind(owner_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    if registered_owner != Some(owner_user_id) {
        return Err(conflict(
            "Ledger identifier already belongs to another owner",
        ));
    }
    sqlx::query(
        "INSERT INTO beecount_ledger_members (ledger_id,user_id,role) \
         SELECT $1,$2,'owner' WHERE NOT EXISTS ( \
           SELECT 1 FROM beecount_ledger_members WHERE ledger_id=$1 AND role='owner') \
         ON CONFLICT (ledger_id,user_id) DO NOTHING",
    )
    .bind(ledger_id)
    .bind(owner_user_id)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn member_user_ids(pool: &PgPool, ledger_id: &str) -> Result<Vec<Uuid>, ApiError> {
    let rows = sqlx::query_scalar(
        "SELECT user_id FROM beecount_ledger_members WHERE ledger_id=$1 ORDER BY joined_at",
    )
    .bind(ledger_id)
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    Ok(rows)
}

pub async fn editor_members_for_owner(
    pool: &PgPool,
    owner_user_id: Uuid,
) -> Result<Vec<(String, Uuid)>, ApiError> {
    let rows = sqlx::query(
        "SELECT s.ledger_id,m.user_id FROM beecount_shared_ledgers s \
         JOIN beecount_ledger_members owner_m \
           ON owner_m.ledger_id=s.ledger_id AND owner_m.role='owner' \
         JOIN beecount_ledger_members m ON m.ledger_id=s.ledger_id \
         WHERE owner_m.user_id=$1 AND m.role='editor' ORDER BY s.ledger_id,m.joined_at",
    )
    .bind(owner_user_id)
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("ledger_id").map_err(internal)?,
                row.try_get("user_id").map_err(internal)?,
            ))
        })
        .collect()
}

pub async fn beecount_user_id(pool: &PgPool, user_id: Uuid) -> Result<String, ApiError> {
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT beecount_user_id FROM beecount_identity_links WHERE user_id=$1),$1::text)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

pub async fn internal_user_id(pool: &PgPool, wire_user_id: &str) -> Result<Uuid, ApiError> {
    let parsed = Uuid::parse_str(wire_user_id).ok();
    sqlx::query_scalar(
        "SELECT user_id FROM beecount_identity_links WHERE beecount_user_id=$1 \
         UNION ALL SELECT id FROM cloud_users WHERE id=$2 LIMIT 1",
    )
    .bind(wire_user_id)
    .bind(parsed)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| not_found("User not found"))
}

pub fn parse_user_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| unauthorized("invalid BeeCount user"))
}

pub fn validate_ledger_id(ledger_id: &str) -> Result<(), ApiError> {
    if ledger_id.is_empty()
        || ledger_id.len() > 256
        || ledger_id.trim() != ledger_id
        || ledger_id.chars().any(char::is_control)
    {
        return Err(not_found("Ledger not found"));
    }
    Ok(())
}

async fn ledger_exists_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_user_id: Uuid,
    ledger_id: &str,
) -> Result<bool, ApiError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sync_entities \
         WHERE user_id=$1 AND entity_type='finance.ledger' AND is_deleted=FALSE \
           AND (entity_id=$2 OR payload->>'beecountLedgerId'=$3))",
    )
    .bind(owner_user_id)
    .bind(lifetrace_entity_id(ledger_id))
    .bind(ledger_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_error)
}

pub fn db_error(_error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "BeeCount collaboration storage temporarily unavailable",
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

pub fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        ErrorCode::InternalError,
        error.to_string(),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

pub fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

pub fn not_found(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::NOT_FOUND)
}

pub fn conflict(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::CONFLICT)
}

pub fn unauthorized(message: &str) -> ApiError {
    ApiError::new(ErrorCode::AuthInvalid, message, StatusCode::UNAUTHORIZED)
}
