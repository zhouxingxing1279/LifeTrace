use axum::http::StatusCode;
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::registry::{EntityType, REGISTRY};
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::{Cursor, DeviceId, EntityId, ErrorCode, ServerVersion, UserId};
use serde_json::Value;
use sqlx::Row;

use super::PostgresRepository;
use crate::error::ApiError;
use crate::sync::payload_hash::scope_hash;

impl PostgresRepository {
    pub(super) async fn capabilities_impl(&self) -> Result<CapabilitiesResponseV1, ApiError> {
        Ok(CapabilitiesResponseV1 {
            protocol_version: lifetrace_contracts::PROTOCOL_VERSION,
            supported_protocol_versions: vec![1],
            schema_version: lifetrace_contracts::SCHEMA_VERSION,
            minimum_schema_version: lifetrace_contracts::MINIMUM_SCHEMA_VERSION,
            minimum_client_versions: vec![MinimumClientVersion {
                app_id: AppId::new(AppId::DESKTOP),
                client_version: "0.2.1".to_owned(),
            }],
            maximum_push_batch_size: self.config.push_max_changes as u32,
            maximum_pull_batch_size: self.config.pull_max_changes as u32,
            maximum_request_bytes: self.config.request_body_limit_bytes as u64,
            maximum_snapshot_page_size: self.config.snapshot_max_page_size as u32,
            maximum_atomic_group_size: self.config.maximum_atomic_group_size as u32,
            tombstone_retention_days: 90,
            supported_entity_types: REGISTRY
                .iter()
                .map(|descriptor| descriptor.entity_type.to_owned())
                .collect(),
            server_time: Self::now(),
        })
    }

    pub(super) async fn pull_impl(
        &self,
        user_id: &UserId,
        request: &PullRequestV1,
    ) -> Result<PullResponseV1, ApiError> {
        self.validate_client(&request.client)?;
        let user_uuid = Self::user_uuid(user_id);
        let scope = scope_hash(&request.entity_types);
        let latest = self.latest_cursor_raw(&self.pool, user_uuid).await?;
        let minimum = self.min_valid_cursor(&self.pool, user_uuid).await?;
        let after = match &request.after_cursor {
            Some(cursor) => {
                let value = self.cursor_codec.decode(cursor, user_id, &scope)?;
                if value > latest {
                    return Err(ApiError::new(
                        ErrorCode::CursorInvalid,
                        "cursor is ahead of the server",
                        StatusCode::BAD_REQUEST,
                    ));
                }
                if value < minimum {
                    return Err(ApiError::new(
                        ErrorCode::CursorExpired,
                        "cursor has expired; snapshot is required",
                        StatusCode::GONE,
                    ));
                }
                value
            }
            None => minimum,
        };
        let limit = (request.limit as usize)
            .min(self.config.pull_max_changes)
            .max(1);
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

        let rows = if request.entity_types.is_some() {
            sqlx::query(
                "SELECT cursor, entity_type, entity_id, operation, server_version,\
                        server_modified_at, payload, tombstone, origin_device_external_id \
                 FROM sync_change_log \
                 WHERE user_id = $1 AND cursor > $2 AND entity_type = ANY($3) \
                 ORDER BY cursor ASC LIMIT $4",
            )
            .bind(user_uuid)
            .bind(after as i64)
            .bind(&filters)
            .bind((limit + 1) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::db_error)?
        } else {
            sqlx::query(
                "SELECT cursor, entity_type, entity_id, operation, server_version,\
                        server_modified_at, payload, tombstone, origin_device_external_id \
                 FROM sync_change_log \
                 WHERE user_id = $1 AND cursor > $2 \
                 ORDER BY cursor ASC LIMIT $3",
            )
            .bind(user_uuid)
            .bind(after as i64)
            .bind((limit + 1) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::db_error)?
        };

        let has_more = rows.len() > limit;
        let mut changes = Vec::with_capacity(rows.len().min(limit));
        for row in rows.into_iter().take(limit) {
            let raw_cursor: i64 = row.try_get("cursor").map_err(Self::internal_error)?;
            let payload: Option<Value> = row.try_get("payload").map_err(Self::internal_error)?;
            let tombstone: Option<Value> =
                row.try_get("tombstone").map_err(Self::internal_error)?;
            changes.push(ServerChangeV1 {
                cursor: Cursor::new(raw_cursor.to_string()),
                entity_type: EntityType::new(
                    row.try_get::<String, _>("entity_type")
                        .map_err(Self::internal_error)?,
                ),
                entity_id: EntityId::new(
                    row.try_get::<String, _>("entity_id")
                        .map_err(Self::internal_error)?,
                ),
                operation: ChangeOperation::new(
                    row.try_get::<String, _>("operation")
                        .map_err(Self::internal_error)?,
                ),
                server_version: ServerVersion::from_u64(
                    row.try_get::<i64, _>("server_version")
                        .map_err(Self::internal_error)?
                        .max(0) as u64,
                ),
                server_modified_at: row
                    .try_get("server_modified_at")
                    .map_err(Self::internal_error)?,
                payload: payload.map(JsonValue),
                tombstone: tombstone
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(Self::internal_error)?,
                origin_device_id: row
                    .try_get::<Option<String>, _>("origin_device_external_id")
                    .map_err(Self::internal_error)?
                    .map(DeviceId::new),
            });
        }
        let next_position = changes
            .last()
            .and_then(|change| change.cursor.as_str().parse::<u64>().ok())
            .unwrap_or(after);
        Ok(PullResponseV1 {
            request_id: request.request_id.clone(),
            server_time: Self::now(),
            changes,
            next_cursor: self.cursor_codec.encode(user_id, &scope, next_position),
            has_more,
        })
    }
}
