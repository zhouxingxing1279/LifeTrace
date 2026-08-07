use axum::http::StatusCode;
use lifetrace_contracts::domain::payload::EntityPayload;
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::{ConflictId, ErrorCode, ServerVersion, UserId};
use serde_json::Value;
use sqlx::{Postgres, Row, Transaction};

use super::super::PostgresRepository;
use crate::error::ApiError;
use crate::sync::payload_hash::{change_hash, empty_scope, sha256_hex};

impl PostgresRepository {
    pub(super) async fn process_change(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        client: &SyncClientInfo,
        change: &SyncChangeV1,
    ) -> Result<PushChangeResultV1, ApiError> {
        let user_uuid = Self::user_uuid(user_id);
        let change_uuid = Self::change_uuid(user_id, &change.change_id);
        let incoming_hash_hex = change_hash(change);
        let incoming_hash = hex::decode(&incoming_hash_hex).map_err(Self::internal_error)?;

        if let Some(row) = sqlx::query(
            "SELECT request_hash, result_json FROM sync_processed_changes \
             WHERE user_id = $1 AND change_id = $2",
        )
        .bind(user_uuid)
        .bind(change_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(Self::db_error)?
        {
            let stored_hash: Vec<u8> = row.try_get("request_hash").map_err(Self::internal_error)?;
            if stored_hash == incoming_hash {
                let value: Value = row.try_get("result_json").map_err(Self::internal_error)?;
                let result: PushChangeResultV1 =
                    serde_json::from_value(value).map_err(Self::internal_error)?;
                return Ok(Self::duplicate_result(result));
            }
            return Ok(PushChangeResultV1::Rejected {
                change_id: change.change_id.clone(),
                entity_type: change.entity_type.clone(),
                entity_id: change.entity_id.clone(),
                code: ErrorCode::ChangeIdReuse,
                message: format!(
                    "changeId {} was already used with a different payload",
                    change.change_id
                ),
                field_errors: vec![],
            });
        }

        let Some(descriptor) = lifetrace_contracts::registry::describe(change.entity_type.as_str())
        else {
            return Ok(PushChangeResultV1::Rejected {
                change_id: change.change_id.clone(),
                entity_type: change.entity_type.clone(),
                entity_id: change.entity_id.clone(),
                code: ErrorCode::UnknownEntityType,
                message: format!("unknown entity type: {}", change.entity_type),
                field_errors: vec![],
            });
        };
        if change.entity_schema_version != descriptor.schema_version {
            return Ok(PushChangeResultV1::Rejected {
                change_id: change.change_id.clone(),
                entity_type: change.entity_type.clone(),
                entity_id: change.entity_id.clone(),
                code: ErrorCode::SchemaUnsupported,
                message: format!(
                    "entity {} expects schema version {}, got {}",
                    change.entity_type, descriptor.schema_version, change.entity_schema_version
                ),
                field_errors: vec![],
            });
        }

        for dependency in &change.dependencies {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(\
                    SELECT 1 FROM sync_entities \
                    WHERE user_id = $1 AND entity_type = $2 AND entity_id = $3 AND is_deleted = FALSE\
                 )",
            )
            .bind(user_uuid)
            .bind(dependency.entity_type.as_str())
            .bind(dependency.entity_id.as_str())
            .fetch_one(&mut **tx)
            .await
            .map_err(Self::db_error)?;
            if !exists {
                return Ok(PushChangeResultV1::Rejected {
                    change_id: change.change_id.clone(),
                    entity_type: change.entity_type.clone(),
                    entity_id: change.entity_id.clone(),
                    code: ErrorCode::DependencyMissing,
                    message: format!(
                        "dependency {}:{} does not exist",
                        dependency.entity_type, dependency.entity_id
                    ),
                    field_errors: vec![],
                });
            }
        }

        let current = sqlx::query(
            "SELECT server_version, payload, is_deleted \
             FROM sync_entities \
             WHERE user_id = $1 AND entity_type = $2 AND entity_id = $3 \
             FOR UPDATE",
        )
        .bind(user_uuid)
        .bind(change.entity_type.as_str())
        .bind(change.entity_id.as_str())
        .fetch_optional(&mut **tx)
        .await
        .map_err(Self::db_error)?;

        let current_version = current
            .as_ref()
            .and_then(|row| row.try_get::<i64, _>("server_version").ok())
            .unwrap_or(0)
            .max(0) as u64;
        let current_deleted = current
            .as_ref()
            .and_then(|row| row.try_get::<bool, _>("is_deleted").ok())
            .unwrap_or(false);
        let current_payload = current
            .as_ref()
            .and_then(|row| row.try_get::<Option<Value>, _>("payload").ok())
            .flatten()
            .map(JsonValue);
        let base = change.base_server_version.to_u64().unwrap_or(u64::MAX);

        let decision = match change.operation.as_str() {
            ChangeOperation::UPSERT => {
                let Some(payload) = &change.payload else {
                    return Ok(PushChangeResultV1::Rejected {
                        change_id: change.change_id.clone(),
                        entity_type: change.entity_type.clone(),
                        entity_id: change.entity_id.clone(),
                        code: ErrorCode::InvalidEntityPayload,
                        message: "upsert requires a payload".to_owned(),
                        field_errors: vec![],
                    });
                };
                let parsed = match EntityPayload::try_from((&change.entity_type, payload.clone())) {
                    Ok(value) => value,
                    Err(message) => {
                        return Ok(PushChangeResultV1::Rejected {
                            change_id: change.change_id.clone(),
                            entity_type: change.entity_type.clone(),
                            entity_id: change.entity_id.clone(),
                            code: ErrorCode::InvalidEntityPayload,
                            message,
                            field_errors: vec![],
                        })
                    }
                };
                if parsed.entity_id() != &change.entity_id {
                    return Ok(PushChangeResultV1::Rejected {
                        change_id: change.change_id.clone(),
                        entity_type: change.entity_type.clone(),
                        entity_id: change.entity_id.clone(),
                        code: ErrorCode::InvalidEntityPayload,
                        message: "payload entity id does not match change entity id".to_owned(),
                        field_errors: vec![],
                    });
                }
                if current.is_none() {
                    if base == 0 {
                        DbDecision::Accept {
                            server_version: 1,
                            noop: false,
                        }
                    } else {
                        DbDecision::Conflict {
                            reason: ConflictReason::new(ConflictReason::BASE_VERSION_MISMATCH),
                        }
                    }
                } else if current_version != base {
                    DbDecision::Conflict {
                        reason: ConflictReason::new(if current_deleted {
                            ConflictReason::CLIENT_MODIFIED_SERVER_DELETED
                        } else {
                            ConflictReason::BASE_VERSION_MISMATCH
                        }),
                    }
                } else {
                    DbDecision::Accept {
                        server_version: current_version + 1,
                        noop: false,
                    }
                }
            }
            ChangeOperation::DELETE => {
                if current.is_none() {
                    if base == 0 {
                        DbDecision::Accept {
                            server_version: 0,
                            noop: true,
                        }
                    } else {
                        DbDecision::Conflict {
                            reason: ConflictReason::new(ConflictReason::BASE_VERSION_MISMATCH),
                        }
                    }
                } else if current_version != base {
                    DbDecision::Conflict {
                        reason: ConflictReason::new(if current_deleted {
                            ConflictReason::BOTH_DELETED
                        } else {
                            ConflictReason::CLIENT_DELETED_SERVER_MODIFIED
                        }),
                    }
                } else if current_deleted {
                    DbDecision::Accept {
                        server_version: current_version,
                        noop: true,
                    }
                } else {
                    DbDecision::Accept {
                        server_version: current_version + 1,
                        noop: false,
                    }
                }
            }
            other => {
                return Ok(PushChangeResultV1::Rejected {
                    change_id: change.change_id.clone(),
                    entity_type: change.entity_type.clone(),
                    entity_id: change.entity_id.clone(),
                    code: ErrorCode::InvalidEntityPayload,
                    message: format!("unsupported operation: {other}"),
                    field_errors: vec![],
                })
            }
        };

        match decision {
            DbDecision::Conflict { reason } => {
                let result = PushChangeResultV1::Conflict {
                    conflict_id: ConflictId::new(format!("conflict-{}", change.change_id)),
                    change_id: change.change_id.clone(),
                    entity_type: change.entity_type.clone(),
                    entity_id: change.entity_id.clone(),
                    client_base_server_version: ServerVersion::from_u64(base),
                    current_server_version: ServerVersion::from_u64(current_version),
                    server_entity: current_payload,
                    server_deleted: current_deleted,
                    reason,
                };
                self.insert_processed(tx, user_id, change, &incoming_hash, &result)
                    .await?;
                Ok(result)
            }
            DbDecision::Accept {
                server_version,
                noop,
            } => {
                let now = Self::now();
                if noop {
                    let cursor = self.latest_cursor_raw(&mut **tx, user_uuid).await?;
                    let result = PushChangeResultV1::Accepted {
                        change_id: change.change_id.clone(),
                        entity_type: change.entity_type.clone(),
                        entity_id: change.entity_id.clone(),
                        server_version: ServerVersion::from_u64(server_version),
                        cursor: self.cursor_codec.encode(user_id, &empty_scope(), cursor),
                        server_modified_at: now,
                    };
                    self.insert_processed(tx, user_id, change, &incoming_hash, &result)
                        .await?;
                    return Ok(result);
                }

                let device_uuid = Self::device_uuid(user_id, &client.device_id, &client.app_id);
                let payload_json = change.payload.as_ref().map(|value| value.0.clone());
                let payload_hash = payload_json
                    .as_ref()
                    .map(serde_json::to_vec)
                    .transpose()
                    .map_err(Self::internal_error)?
                    .map(|bytes| hex::decode(sha256_hex(&bytes)).expect("sha256 hex"));

                let tombstone = if change.operation.as_str() == ChangeOperation::DELETE {
                    Some(TombstoneV1 {
                        entity_type: change.entity_type.clone(),
                        entity_id: change.entity_id.clone(),
                        deleted_at: now,
                        server_version: ServerVersion::from_u64(server_version),
                        deleted_by_device: Some(client.device_id.clone()),
                    })
                } else {
                    None
                };
                let tombstone_json = tombstone
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(Self::internal_error)?;

                let cursor: i64 = sqlx::query_scalar(
                    "INSERT INTO sync_change_log (\
                        user_id, entity_type, entity_id, operation, entity_schema_version,\
                        server_version, payload, payload_hash, tombstone, origin_device_id,\
                        origin_device_external_id, client_modified_at, server_modified_at\
                     ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
                     RETURNING cursor",
                )
                .bind(user_uuid)
                .bind(change.entity_type.as_str())
                .bind(change.entity_id.as_str())
                .bind(change.operation.as_str())
                .bind(change.entity_schema_version as i32)
                .bind(server_version as i64)
                .bind(payload_json.clone())
                .bind(payload_hash.clone())
                .bind(tombstone_json)
                .bind(device_uuid)
                .bind(client.device_id.as_str())
                .bind(change.client_modified_at)
                .bind(now)
                .fetch_one(&mut **tx)
                .await
                .map_err(Self::db_error)?;

                if change.operation.as_str() == ChangeOperation::UPSERT {
                    sqlx::query(
                        "INSERT INTO sync_entities (\
                            user_id, entity_type, entity_id, entity_schema_version, server_version,\
                            payload, payload_hash, is_deleted, deleted_at, origin_device_id,\
                            origin_device_external_id, created_at, server_modified_at,\
                            client_modified_at, last_cursor\
                         ) VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, NULL, $8, $9, $10, $10, $11, $12) \
                         ON CONFLICT (user_id, entity_type, entity_id) DO UPDATE SET \
                            entity_schema_version = EXCLUDED.entity_schema_version,\
                            server_version = EXCLUDED.server_version,\
                            payload = EXCLUDED.payload,\
                            payload_hash = EXCLUDED.payload_hash,\
                            is_deleted = FALSE,\
                            deleted_at = NULL,\
                            origin_device_id = EXCLUDED.origin_device_id,\
                            origin_device_external_id = EXCLUDED.origin_device_external_id,\
                            server_modified_at = EXCLUDED.server_modified_at,\
                            client_modified_at = EXCLUDED.client_modified_at,\
                            last_cursor = EXCLUDED.last_cursor",
                    )
                    .bind(user_uuid)
                    .bind(change.entity_type.as_str())
                    .bind(change.entity_id.as_str())
                    .bind(change.entity_schema_version as i32)
                    .bind(server_version as i64)
                    .bind(payload_json)
                    .bind(payload_hash)
                    .bind(device_uuid)
                    .bind(client.device_id.as_str())
                    .bind(now)
                    .bind(change.client_modified_at)
                    .bind(cursor)
                    .execute(&mut **tx)
                    .await
                    .map_err(Self::db_error)?;
                } else {
                    sqlx::query(
                        "UPDATE sync_entities SET \
                            entity_schema_version = $4, server_version = $5, payload = NULL,\
                            payload_hash = NULL, is_deleted = TRUE, deleted_at = $6,\
                            origin_device_id = $7, origin_device_external_id = $8,\
                            server_modified_at = $6, client_modified_at = $9, last_cursor = $10 \
                         WHERE user_id = $1 AND entity_type = $2 AND entity_id = $3",
                    )
                    .bind(user_uuid)
                    .bind(change.entity_type.as_str())
                    .bind(change.entity_id.as_str())
                    .bind(change.entity_schema_version as i32)
                    .bind(server_version as i64)
                    .bind(now)
                    .bind(device_uuid)
                    .bind(client.device_id.as_str())
                    .bind(change.client_modified_at)
                    .bind(cursor)
                    .execute(&mut **tx)
                    .await
                    .map_err(Self::db_error)?;
                }

                let result = PushChangeResultV1::Accepted {
                    change_id: change.change_id.clone(),
                    entity_type: change.entity_type.clone(),
                    entity_id: change.entity_id.clone(),
                    server_version: ServerVersion::from_u64(server_version),
                    cursor: self
                        .cursor_codec
                        .encode(user_id, &empty_scope(), cursor.max(0) as u64),
                    server_modified_at: now,
                };
                self.insert_processed(tx, user_id, change, &incoming_hash, &result)
                    .await?;
                Ok(result)
            }
        }
    }
}

enum DbDecision {
    Accept { server_version: u64, noop: bool },
    Conflict { reason: ConflictReason },
}
