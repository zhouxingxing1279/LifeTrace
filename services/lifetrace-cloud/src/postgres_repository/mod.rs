//! PostgreSQL-backed sync repository.
//!
//! Every protocol state transition is committed in PostgreSQL: identity and
//! device touch, idempotency, conflict evaluation, entity mutation, change
//! log append and processed result.

mod entities;
mod pull;
mod push;
mod snapshot;

use std::sync::Arc;

use async_trait::async_trait;
use axum::http::StatusCode;
use chrono::Utc;
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::{ChangeId, DeviceId, ErrorCode, SnapshotId, UserId};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::config::Config;
use crate::error::ApiError;
use crate::repository::{StoredEntityRecord, SyncRepository};
use crate::sync::cursor_codec::CursorCodec;
use crate::sync::page_token::PageTokenCodec;

#[derive(Clone)]
pub struct PostgresRepository {
    pub(super) pool: PgPool,
    pub(super) config: Arc<Config>,
    pub(super) cursor_codec: Arc<CursorCodec>,
    pub(super) page_token_codec: Arc<PageTokenCodec>,
}

impl PostgresRepository {
    pub fn new(
        pool: PgPool,
        config: Config,
        cursor_codec: CursorCodec,
        page_token_codec: PageTokenCodec,
    ) -> Self {
        Self {
            pool,
            config: Arc::new(config),
            cursor_codec: Arc::new(cursor_codec),
            page_token_codec: Arc::new(page_token_codec),
        }
    }

    fn now() -> chrono::DateTime<Utc> {
        Utc::now()
    }

    fn stable_uuid(kind: &str, value: &str) -> Uuid {
        Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("https://lifetrace.local/{kind}/{value}").as_bytes(),
        )
    }

    fn user_uuid(user_id: &UserId) -> Uuid {
        Self::stable_uuid("users", user_id.as_str())
    }

    fn device_uuid(user_id: &UserId, device_id: &DeviceId, app_id: &AppId) -> Uuid {
        Self::stable_uuid(
            "devices",
            &format!("{}:{}:{}", user_id.as_str(), app_id.as_str(), device_id.as_str()),
        )
    }

    fn change_uuid(user_id: &UserId, change_id: &ChangeId) -> Uuid {
        Self::stable_uuid(
            "changes",
            &format!("{}:{}", user_id.as_str(), change_id.as_str()),
        )
    }

    fn parse_snapshot_uuid(snapshot_id: &SnapshotId) -> Result<Uuid, ApiError> {
        Uuid::parse_str(snapshot_id.as_str()).map_err(|_| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                "invalid snapshot id",
                StatusCode::BAD_REQUEST,
            )
        })
    }

    fn db_error(error: sqlx::Error) -> ApiError {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            format!("database operation failed: {error}"),
            StatusCode::SERVICE_UNAVAILABLE,
        )
    }

    fn internal_error(error: impl std::fmt::Display) -> ApiError {
        ApiError::new(
            ErrorCode::InternalError,
            error.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    }

    fn validate_client(&self, client: &SyncClientInfo) -> Result<(), ApiError> {
        if client.protocol_version != 1 {
            return Err(ApiError::new(
                ErrorCode::ProtocolUnsupported,
                format!("protocol version {} is not supported", client.protocol_version),
                StatusCode::UPGRADE_REQUIRED,
            ));
        }
        if client.schema_version < lifetrace_contracts::MINIMUM_SCHEMA_VERSION {
            return Err(ApiError::new(
                ErrorCode::SchemaUnsupported,
                format!(
                    "schema version {} is below minimum {}",
                    client.schema_version,
                    lifetrace_contracts::MINIMUM_SCHEMA_VERSION
                ),
                StatusCode::BAD_REQUEST,
            ));
        }
        Ok(())
    }

    async fn ensure_identity(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        client: &SyncClientInfo,
    ) -> Result<(), ApiError> {
        let user_uuid = Self::user_uuid(user_id);
        sqlx::query(
            "INSERT INTO cloud_users (id, status) VALUES ($1, 'active') \
             ON CONFLICT (id) DO UPDATE SET updated_at = now()",
        )
        .bind(user_uuid)
        .execute(&mut **tx)
        .await
        .map_err(Self::db_error)?;

        let device_uuid = Self::device_uuid(user_id, &client.device_id, &client.app_id);
        sqlx::query(
            "INSERT INTO cloud_devices (\
                 id, user_id, app_id, platform, client_version, protocol_version, schema_version,\
                 status, external_device_id, first_seen_at, last_seen_at\
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', $8, now(), now()) \
             ON CONFLICT (id) DO UPDATE SET \
                 client_version = EXCLUDED.client_version,\
                 protocol_version = EXCLUDED.protocol_version,\
                 schema_version = EXCLUDED.schema_version,\
                 platform = EXCLUDED.platform,\
                 external_device_id = EXCLUDED.external_device_id,\
                 last_seen_at = now()",
        )
        .bind(device_uuid)
        .bind(user_uuid)
        .bind(client.app_id.as_str())
        .bind(client.platform.as_str())
        .bind(&client.client_version)
        .bind(client.protocol_version as i32)
        .bind(client.schema_version as i32)
        .bind(client.device_id.as_str())
        .execute(&mut **tx)
        .await
        .map_err(Self::db_error)?;
        Ok(())
    }

    async fn latest_cursor_raw<'e, E>(&self, executor: E, user_uuid: Uuid) -> Result<u64, ApiError>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        let value: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(cursor), 0)::BIGINT FROM sync_change_log WHERE user_id = $1",
        )
        .bind(user_uuid)
        .fetch_one(executor)
        .await
        .map_err(Self::db_error)?;
        Ok(value.max(0) as u64)
    }

    async fn min_valid_cursor<'e, E>(&self, executor: E, user_uuid: Uuid) -> Result<u64, ApiError>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        let value: Option<i64> = sqlx::query_scalar(
            "SELECT MIN(cursor)::BIGINT FROM sync_change_log WHERE user_id = $1",
        )
        .bind(user_uuid)
        .fetch_one(executor)
        .await
        .map_err(Self::db_error)?;
        Ok(value
            .map(|cursor| cursor.saturating_sub(1).max(0) as u64)
            .unwrap_or(0))
    }

    fn duplicate_result(result: PushChangeResultV1) -> PushChangeResultV1 {
        match result {
            PushChangeResultV1::Accepted {
                change_id,
                entity_type,
                entity_id,
                server_version,
                cursor,
                server_modified_at,
            } => PushChangeResultV1::Duplicate {
                change_id,
                entity_type,
                entity_id,
                server_version,
                cursor,
                server_modified_at,
            },
            other => other,
        }
    }

    fn result_status(result: &PushChangeResultV1) -> &'static str {
        match result {
            PushChangeResultV1::Accepted { .. } => "accepted",
            PushChangeResultV1::Duplicate { .. } => "duplicate",
            PushChangeResultV1::Conflict { .. } => "conflict",
            PushChangeResultV1::Rejected { .. } => "rejected",
        }
    }

    async fn insert_processed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        change: &SyncChangeV1,
        hash_bytes: &[u8],
        result: &PushChangeResultV1,
    ) -> Result<(), ApiError> {
        let result_json = serde_json::to_value(result).map_err(Self::internal_error)?;
        sqlx::query(
            "INSERT INTO sync_processed_changes (\
                 user_id, change_id, request_hash, result_status, result_json, entity_type, entity_id\
             ) VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (user_id, change_id) DO NOTHING",
        )
        .bind(Self::user_uuid(user_id))
        .bind(Self::change_uuid(user_id, &change.change_id))
        .bind(hash_bytes)
        .bind(Self::result_status(result))
        .bind(result_json)
        .bind(change.entity_type.as_str())
        .bind(change.entity_id.as_str())
        .execute(&mut **tx)
        .await
        .map_err(Self::db_error)?;
        Ok(())
    }
}

#[async_trait]
impl SyncRepository for PostgresRepository {
    async fn capabilities(&self) -> Result<CapabilitiesResponseV1, ApiError> {
        self.capabilities_impl().await
    }

    async fn push(
        &self,
        user_id: &UserId,
        request: &PushRequestV1,
    ) -> Result<PushResponseV1, ApiError> {
        self.push_impl(user_id, request).await
    }

    async fn pull(
        &self,
        user_id: &UserId,
        request: &PullRequestV1,
    ) -> Result<PullResponseV1, ApiError> {
        self.pull_impl(user_id, request).await
    }

    async fn snapshot(
        &self,
        user_id: &UserId,
        request: &SnapshotRequestV1,
    ) -> Result<SnapshotResponseV1, ApiError> {
        self.snapshot_impl(user_id, request).await
    }

    async fn list_entities(
        &self,
        user_id: &UserId,
        entity_type: &str,
    ) -> Result<Vec<EntitySnapshotV1>, ApiError> {
        self.list_entities_impl(user_id, entity_type).await
    }

    async fn entity(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<StoredEntityRecord>, ApiError> {
        self.entity_impl(user_id, entity_type, entity_id).await
    }

    async fn current_version(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &lifetrace_contracts::EntityId,
    ) -> Result<Option<u64>, ApiError> {
        self.current_version_impl(user_id, entity_type, entity_id)
            .await
    }

    async fn change_count(&self, user_id: &UserId) -> Result<usize, ApiError> {
        self.change_count_impl(user_id).await
    }
}
