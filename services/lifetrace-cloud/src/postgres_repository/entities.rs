use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::registry::EntityType;
use lifetrace_contracts::sync::v1::EntitySnapshotV1;
use lifetrace_contracts::{EntityId, ServerVersion, UserId};
use serde_json::Value;
use sqlx::Row;

use super::PostgresRepository;
use crate::error::ApiError;
use crate::repository::StoredEntityRecord;

impl PostgresRepository {
    pub(super) async fn list_entities_impl(
        &self,
        user_id: &UserId,
        entity_type: &str,
    ) -> Result<Vec<EntitySnapshotV1>, ApiError> {
        let rows = sqlx::query(
            "SELECT entity_type, entity_id, server_version, payload \
             FROM sync_entities \
             WHERE user_id = $1 AND entity_type = $2 AND is_deleted = FALSE \
             ORDER BY entity_id",
        )
        .bind(Self::user_uuid(user_id))
        .bind(entity_type)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;
        rows.into_iter()
            .map(|row| {
                Ok(EntitySnapshotV1 {
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
                })
            })
            .collect()
    }

    pub(super) async fn entity_impl(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<StoredEntityRecord>, ApiError> {
        let row = sqlx::query(
            "SELECT entity_type, entity_id, server_version, payload, is_deleted \
             FROM sync_entities WHERE user_id = $1 AND entity_type = $2 AND entity_id = $3",
        )
        .bind(Self::user_uuid(user_id))
        .bind(entity_type)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db_error)?;
        row.map(|row| {
            let payload: Option<Value> = row.try_get("payload").map_err(Self::internal_error)?;
            Ok(StoredEntityRecord {
                entity_type: EntityType::new(
                    row.try_get::<String, _>("entity_type")
                        .map_err(Self::internal_error)?,
                ),
                entity_id: EntityId::new(
                    row.try_get::<String, _>("entity_id")
                        .map_err(Self::internal_error)?,
                ),
                server_version: row
                    .try_get::<i64, _>("server_version")
                    .map_err(Self::internal_error)?
                    .max(0) as u64,
                payload: JsonValue(payload.unwrap_or(Value::Null)),
                deleted: row.try_get("is_deleted").map_err(Self::internal_error)?,
            })
        })
        .transpose()
    }

    pub(super) async fn current_version_impl(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &EntityId,
    ) -> Result<Option<u64>, ApiError> {
        let value: Option<i64> = sqlx::query_scalar(
            "SELECT server_version FROM sync_entities \
             WHERE user_id = $1 AND entity_type = $2 AND entity_id = $3 AND is_deleted = FALSE",
        )
        .bind(Self::user_uuid(user_id))
        .bind(entity_type)
        .bind(entity_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db_error)?;
        Ok(value.map(|version| version.max(0) as u64))
    }

    pub(super) async fn change_count_impl(&self, user_id: &UserId) -> Result<usize, ApiError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM sync_change_log WHERE user_id = $1")
                .bind(Self::user_uuid(user_id))
                .fetch_one(&self.pool)
                .await
                .map_err(Self::db_error)?;
        Ok(count.max(0) as usize)
    }
}
