use axum::http::StatusCode;
use chrono::{Duration, Utc};
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::registry::EntityType;
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::{EntityId, ErrorCode, ServerVersion, SnapshotId, UserId};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use super::PostgresRepository;
use crate::error::ApiError;
use crate::sync::payload_hash::scope_hash;

impl PostgresRepository {
    pub(super) async fn snapshot_impl(
        &self,
        user_id: &UserId,
        request: &SnapshotRequestV1,
    ) -> Result<SnapshotResponseV1, ApiError> {
        self.validate_client(&request.client)?;
        let user_uuid = Self::user_uuid(user_id);
        let snapshot_id = match &request.snapshot_id {
            Some(id) => id.clone(),
            None => SnapshotId::new(Uuid::new_v4().to_string()),
        };
        let snapshot_uuid = Self::parse_snapshot_uuid(&snapshot_id)?;

        if request.snapshot_id.is_none() {
            let mut tx = self.pool.begin().await.map_err(Self::db_error)?;
            self.ensure_identity(&mut tx, user_id, &request.client)
                .await?;
            let cursor = self.latest_cursor_raw(&mut *tx, user_uuid).await?;
            let scope = scope_hash(&request.entity_types);
            let scope_bytes = hex::decode(scope).map_err(Self::internal_error)?;
            let expires_at = Self::now()
                + Duration::seconds(self.config.snapshot_ttl_seconds.min(i64::MAX as u64) as i64);
            sqlx::query(
                r#"
                INSERT INTO sync_snapshots (
                    id, user_id, scope_hash, snapshot_cursor, status, created_at, expires_at
                ) VALUES ($1, $2, $3, $4, 'building', now(), $5)
                "#,
            )
            .bind(snapshot_uuid)
            .bind(user_uuid)
            .bind(scope_bytes)
            .bind(cursor as i64)
            .bind(expires_at)
            .execute(&mut *tx)
            .await
            .map_err(Self::db_error)?;

            let filters: Vec<String> = request
                .entity_types
                .as_ref()
                .map(|types| {
                    types
                        .iter()
                        .map(|value| value.as_str().to_owned())
                        .collect()
                })
                .unwrap_or_default();
            let inserted = if request.entity_types.is_some() {
                sqlx::query(
                    r#"
                    INSERT INTO sync_snapshot_items (
                        snapshot_id, entity_type, entity_id, entity_schema_version,
                        server_version, payload, payload_hash, server_modified_at
                    )
                    SELECT
                        $1, entity_type, entity_id, entity_schema_version,
                        server_version, payload, payload_hash, server_modified_at
                    FROM sync_entities
                    WHERE user_id = $2
                      AND is_deleted = FALSE
                      AND entity_type = ANY($3)
                    "#,
                )
                .bind(snapshot_uuid)
                .bind(user_uuid)
                .bind(&filters)
                .execute(&mut *tx)
                .await
                .map_err(Self::db_error)?
                .rows_affected()
            } else {
                sqlx::query(
                    r#"
                    INSERT INTO sync_snapshot_items (
                        snapshot_id, entity_type, entity_id, entity_schema_version,
                        server_version, payload, payload_hash, server_modified_at
                    )
                    SELECT
                        $1, entity_type, entity_id, entity_schema_version,
                        server_version, payload, payload_hash, server_modified_at
                    FROM sync_entities
                    WHERE user_id = $2
                      AND is_deleted = FALSE
                    "#,
                )
                .bind(snapshot_uuid)
                .bind(user_uuid)
                .execute(&mut *tx)
                .await
                .map_err(Self::db_error)?
                .rows_affected()
            };
            sqlx::query(
                r#"
                UPDATE sync_snapshots
                SET status = 'ready', item_count = $2, completed_at = now()
                WHERE id = $1
                "#,
            )
            .bind(snapshot_uuid)
            .bind(inserted as i64)
            .execute(&mut *tx)
            .await
            .map_err(Self::db_error)?;
            tx.commit().await.map_err(Self::db_error)?;
        }

        let row = sqlx::query(
            r#"
            SELECT scope_hash, snapshot_cursor, status, expires_at
            FROM sync_snapshots
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(snapshot_uuid)
        .bind(user_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db_error)?
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                format!("unknown snapshot id {snapshot_id}"),
                StatusCode::BAD_REQUEST,
            )
        })?;
        let status: String = row.try_get("status").map_err(Self::internal_error)?;
        let expires_at: chrono::DateTime<Utc> =
            row.try_get("expires_at").map_err(Self::internal_error)?;
        if status != "ready" || expires_at <= Self::now() {
            return Err(ApiError::new(
                ErrorCode::SnapshotRequired,
                "snapshot is unavailable or expired",
                StatusCode::GONE,
            ));
        }
        let scope = hex::encode(
            row.try_get::<Vec<u8>, _>("scope_hash")
                .map_err(Self::internal_error)?,
        );
        let snapshot_cursor = row
            .try_get::<i64, _>("snapshot_cursor")
            .map_err(Self::internal_error)?
            .max(0) as u64;
        let offset = match &request.page_token {
            Some(token) => self.page_token_codec.decode(token, user_id, &snapshot_id)?,
            None => 0,
        };
        let page_size = (request.page_size as usize)
            .max(1)
            .min(self.config.snapshot_max_page_size);
        let rows = sqlx::query(
            r#"
            SELECT entity_type, entity_id, server_version, payload
            FROM sync_snapshot_items
            WHERE snapshot_id = $1
            ORDER BY entity_type, entity_id
            OFFSET $2 LIMIT $3
            "#,
        )
        .bind(snapshot_uuid)
        .bind(offset as i64)
        .bind((page_size + 1) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;
        let completed = rows.len() <= page_size;
        let mut items = Vec::with_capacity(rows.len().min(page_size));
        for row in rows.into_iter().take(page_size) {
            items.push(EntitySnapshotV1 {
                entity_type: EntityType::new(
                    row.try_get::<String, _>("entity_type")
                        .map_err(Self::internal_error)?,
                ),
                entity_id: EntityId::new(
                    row.try_get::<String, _>("entity_id")
                        .map_err(Self::internal_error)?,
                ),
                server_version: ServerVersion::from_u64(
                    row.try_get::<i64, _>("server_version")
                        .map_err(Self::internal_error)?
                        .max(0) as u64,
                ),
                payload: JsonValue(
                    row.try_get::<Value, _>("payload")
                        .map_err(Self::internal_error)?,
                ),
            });
        }
        let next_page_token = if completed {
            None
        } else {
            Some(
                self.page_token_codec
                    .encode(user_id, &snapshot_id, offset + page_size),
            )
        };
        Ok(SnapshotResponseV1 {
            request_id: request.request_id.clone(),
            snapshot_id,
            snapshot_cursor: self.cursor_codec.encode(user_id, &scope, snapshot_cursor),
            items,
            next_page_token,
            completed,
            server_time: Self::now(),
        })
    }
}
