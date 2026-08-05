//! In-memory per-user sync server state machine.
//!
//! This is the production-shaped counterpart of the reference testkit in
//! `lifetrace-contracts`: it implements the same protocol semantics
//! (push idempotency, base-version conflicts, tombstones, cursor ordering,
//! snapshot consistency, atomic groups) but is wired behind an HTTP API and
//! keyed per user so multi-tenant isolation logic is explicit.
//!
//! Persistence is intentionally out of scope for this prototype: a future
//! PostgreSQL-backed implementation replaces the in-memory maps while
//! keeping the same public methods.

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use chrono::Utc;
use lifetrace_contracts::domain::payload::EntityPayload;
use lifetrace_contracts::error::{ApiErrorV1, ErrorCode, FieldError};
use lifetrace_contracts::ids::{
    AtomicGroupId, ChangeId, ConflictId, Cursor, DeviceId, EntityId, ServerVersion, SnapshotId,
    UserId,
};
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::registry::{EntityType, REGISTRY};
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::time::UtcTimestamp;

use crate::config::Config;
use crate::error::ApiError;

/// Current stored server state of one entity.
#[derive(Debug, Clone)]
pub struct StoredEntity {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub server_version: u64,
    pub payload: JsonValue,
    pub deleted: bool,
    pub deleted_at: Option<UtcTimestamp>,
    pub deleted_by_device: Option<DeviceId>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    pub origin_device_id: Option<DeviceId>,
}

/// Recorded first result for a changeId (idempotency).
#[derive(Debug, Clone)]
struct StoredChangeResult {
    result: PushChangeResultV1,
    payload_json: Option<String>,
}

#[derive(Debug, Clone)]
struct ServerChangeRecord {
    cursor: u64,
    entity_type: EntityType,
    entity_id: EntityId,
    operation: ChangeOperation,
    server_version: ServerVersion,
    server_modified_at: UtcTimestamp,
    payload: Option<JsonValue>,
    tombstone: Option<TombstoneV1>,
    origin_device_id: Option<DeviceId>,
}

#[derive(Debug, Clone)]
struct StoredSnapshot {
    cursor: u64,
    items: Vec<EntitySnapshotV1>,
}

/// Per-user state machine state.
#[derive(Debug, Default)]
struct UserState {
    entities: HashMap<(String, EntityId), StoredEntity>,
    processed: HashMap<ChangeId, StoredChangeResult>,
    change_log: Vec<ServerChangeRecord>,
    next_cursor: u64,
    snapshots: HashMap<SnapshotId, StoredSnapshot>,
}

/// Multi-user in-memory sync store.
#[derive(Debug)]
pub struct Store {
    users: HashMap<UserId, UserState>,
    snapshot_counter: u64,
    config: Arc<Config>,
}

impl Store {
    pub fn new(config: Config) -> Self {
        Self {
            users: HashMap::new(),
            snapshot_counter: 0,
            config: Arc::new(config),
        }
    }

    fn user_mut(&mut self, user_id: &UserId) -> &mut UserState {
        self.users.entry(user_id.clone()).or_default()
    }

    fn now() -> UtcTimestamp {
        Utc::now()
    }

    fn entity_key(entity_type: &str, entity_id: &EntityId) -> (String, EntityId) {
        (entity_type.to_owned(), entity_id.clone())
    }

    // ------------------------------------------------------------------
    // Capabilities
    // ------------------------------------------------------------------

    pub fn capabilities(&self) -> CapabilitiesResponseV1 {
        CapabilitiesResponseV1 {
            protocol_version: lifetrace_contracts::PROTOCOL_VERSION,
            supported_protocol_versions: vec![1],
            schema_version: lifetrace_contracts::SCHEMA_VERSION,
            minimum_schema_version: lifetrace_contracts::MINIMUM_SCHEMA_VERSION,
            minimum_client_versions: vec![MinimumClientVersion {
                app_id: AppId::new(AppId::DESKTOP),
                client_version: "0.2.1".to_owned(),
            }],
            maximum_push_batch_size: self.config.max_push_batch_size as u32,
            maximum_pull_batch_size: self.config.max_pull_batch_size as u32,
            maximum_request_bytes: self.config.max_request_bytes as u64,
            maximum_snapshot_page_size: self.config.max_snapshot_page_size as u32,
            maximum_atomic_group_size: self.config.max_atomic_group_size as u32,
            tombstone_retention_days: 90,
            supported_entity_types: REGISTRY
                .iter()
                .map(|descriptor| descriptor.entity_type.to_owned())
                .collect(),
            server_time: Self::now(),
        }
    }

    /// Latest cursor for a user (test helper).
    pub fn latest_cursor(&self, user_id: &UserId) -> Cursor {
        let next_cursor = self
            .users
            .get(user_id)
            .map(|user| user.next_cursor)
            .unwrap_or(0);
        Cursor::new(next_cursor.to_string())
    }

    /// Current stored state for a key (test helper).
    pub fn entity(&self, user_id: &UserId, entity_type: &str, entity_id: &str) -> Option<&StoredEntity> {
        self.users
            .get(user_id)?
            .entities
            .get(&Self::entity_key(entity_type, &EntityId::new(entity_id)))
    }

    /// List current (non-deleted) entities of one type, sorted by entity id.
    pub fn list_entities(&self, user_id: &UserId, entity_type: &str) -> Vec<EntitySnapshotV1> {
        let mut items: Vec<EntitySnapshotV1> = self
            .users
            .get(user_id)
            .map(|user| {
                user.entities
                    .iter()
                    .filter(|(_, entity)| !entity.deleted)
                    .filter(|(_, entity)| entity.entity_type.as_str() == entity_type)
                    .map(|(_, entity)| EntitySnapshotV1 {
                        entity_type: entity.entity_type.clone(),
                        entity_id: entity.entity_id.clone(),
                        server_version: ServerVersion::from_u64(entity.server_version),
                        payload: entity.payload.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        items.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
        items
    }

    /// Current server version of a non-deleted entity, if any.
    pub fn current_version(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &EntityId,
    ) -> Option<u64> {
        self.users
            .get(user_id)
            .and_then(|user| user.entities.get(&Self::entity_key(entity_type, entity_id)))
            .filter(|entity| !entity.deleted)
            .map(|entity| entity.server_version)
    }

    pub fn change_count(&self, user_id: &UserId) -> usize {
        self.users
            .get(user_id)
            .map(|user| user.change_log.len())
            .unwrap_or(0)
    }

    // ------------------------------------------------------------------
    // Push
    // ------------------------------------------------------------------

    pub fn push(
        &mut self,
        user_id: &UserId,
        request: &PushRequestV1,
    ) -> Result<PushResponseV1, ApiError> {
        if request.client.protocol_version != 1 {
            return Err(ApiError::new(
                ErrorCode::ProtocolUnsupported,
                format!(
                    "protocol version {} is not supported",
                    request.client.protocol_version
                ),
                StatusCode::UPGRADE_REQUIRED,
            ));
        }
        if request.client.schema_version < lifetrace_contracts::MINIMUM_SCHEMA_VERSION {
            return Err(ApiError::new(
                ErrorCode::SchemaUnsupported,
                format!(
                    "schema version {} is below minimum {}",
                    request.client.schema_version,
                    lifetrace_contracts::MINIMUM_SCHEMA_VERSION
                ),
                StatusCode::BAD_REQUEST,
            ));
        }
        if request.changes.len() > self.config.max_push_batch_size {
            return Err(ApiError::new(
                ErrorCode::BatchTooLarge,
                format!(
                    "push batch of {} exceeds maximum {}",
                    request.changes.len(),
                    self.config.max_push_batch_size
                ),
                StatusCode::BAD_REQUEST,
            ));
        }
        let wire_size = serde_json::to_vec(request)
            .map_err(|error| {
                ApiError::new(ErrorCode::InternalError, error.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            })?
            .len();
        if wire_size > self.config.max_request_bytes {
            return Err(ApiError::new(
                ErrorCode::PayloadTooLarge,
                format!("request body of {wire_size} bytes exceeds maximum"),
                StatusCode::PAYLOAD_TOO_LARGE,
            ));
        }

        // Atomic groups must fit within the maximum group size.
        let mut group_sizes: HashMap<Option<&AtomicGroupId>, usize> = HashMap::new();
        for change in &request.changes {
            *group_sizes.entry(change.atomic_group_id.as_ref()).or_insert(0) += 1;
        }
        for (group, size) in &group_sizes {
            if let Some(group_id) = group {
                if *size > self.config.max_atomic_group_size {
                    return Err(ApiError::new(
                        ErrorCode::BatchTooLarge,
                        format!(
                            "atomic group {group_id} has {size} changes (maximum {})",
                            self.config.max_atomic_group_size
                        ),
                        StatusCode::BAD_REQUEST,
                    ));
                }
            }
        }

        let mut group_indices: HashMap<Option<&AtomicGroupId>, Vec<usize>> = HashMap::new();
        for (index, change) in request.changes.iter().enumerate() {
            group_indices
                .entry(change.atomic_group_id.as_ref())
                .or_default()
                .push(index);
        }

        let mut results: Vec<PushChangeResultV1> = Vec::with_capacity(request.changes.len());
        let mut ordered_keys: Vec<Option<&AtomicGroupId>> = Vec::with_capacity(group_indices.len());
        for change in &request.changes {
            let key = change.atomic_group_id.as_ref();
            if !ordered_keys.contains(&key) {
                ordered_keys.push(key);
            }
        }

        for group_key in ordered_keys {
            let indices = &group_indices[&group_key];
            if indices.len() == 1 {
                let index = indices[0];
                let change = &request.changes[index];
                let outcome = self.evaluate_change(user_id, change)?;
                results.push(self.apply_outcome(
                    user_id,
                    change,
                    outcome,
                    request.client.device_id.clone(),
                ));
            } else {
                let mut outcomes = Vec::with_capacity(indices.len());
                let mut failure: Option<String> = None;
                for index in indices {
                    let change = &request.changes[*index];
                    match self.evaluate_change(user_id, change) {
                        Ok(outcome)
                            if matches!(
                                outcome,
                                ApplyOutcome::Accepted(_) | ApplyOutcome::Duplicate(_)
                            ) =>
                        {
                            outcomes.push(outcome);
                        }
                        Ok(other) => {
                            failure = Some(match other {
                                ApplyOutcome::Conflict { reason, .. } => {
                                    format!("atomic group failed: {reason}")
                                }
                                ApplyOutcome::Rejected { message, .. } => message,
                                _ => unreachable!(),
                            });
                            break;
                        }
                        Err(error) => return Err(error),
                    }
                }
                if let Some(message) = failure {
                    for index in indices {
                        let change = &request.changes[*index];
                        results.push(PushChangeResultV1::Rejected {
                            change_id: change.change_id.clone(),
                            entity_type: change.entity_type.clone(),
                            entity_id: change.entity_id.clone(),
                            code: ErrorCode::AtomicGroupFailed,
                            message: message.clone(),
                            field_errors: vec![],
                        });
                    }
                } else {
                    for (index, outcome) in indices.iter().zip(outcomes) {
                        let change = &request.changes[*index];
                        results.push(self.apply_outcome(
                            user_id,
                            change,
                            outcome,
                            request.client.device_id.clone(),
                        ));
                    }
                }
            }
        }

        self.prune_change_log(user_id);
        Ok(PushResponseV1 {
            request_id: request.request_id.clone(),
            server_time: Self::now(),
            results,
            latest_cursor: self.latest_cursor(user_id),
        })
    }

    fn evaluate_change(
        &self,
        user_id: &UserId,
        change: &SyncChangeV1,
    ) -> Result<ApplyOutcome, ApiError> {
        let user = self.users.get(user_id);
        if let Some(stored) = user.and_then(|state| state.processed.get(&change.change_id)) {
            let incoming_payload = change
                .payload
                .as_ref()
                .map(|value| serde_json::to_string(&value.0).unwrap_or_default());
            return if stored.payload_json == incoming_payload {
                Ok(ApplyOutcome::Duplicate(stored.result.clone()))
            } else {
                Ok(ApplyOutcome::Rejected {
                    code: ErrorCode::ChangeIdReuse,
                    message: format!(
                        "changeId {} was already used with a different payload",
                        change.change_id
                    ),
                    field_errors: vec![],
                })
            };
        }

        let Some(descriptor) = lifetrace_contracts::registry::describe(change.entity_type.as_str())
        else {
            return Ok(ApplyOutcome::Rejected {
                code: ErrorCode::UnknownEntityType,
                message: format!("unknown entity type: {}", change.entity_type),
                field_errors: vec![],
            });
        };
        if change.entity_schema_version != descriptor.schema_version {
            return Ok(ApplyOutcome::Rejected {
                code: ErrorCode::SchemaUnsupported,
                message: format!(
                    "entity {} expects schema version {}, got {}",
                    change.entity_type, descriptor.schema_version, change.entity_schema_version
                ),
                field_errors: vec![],
            });
        }

        for dependency in &change.dependencies {
            let exists = user
                .map(|state| {
                    state
                        .entities
                        .get(&Self::entity_key(
                            dependency.entity_type.as_str(),
                            &dependency.entity_id,
                        ))
                        .is_some_and(|entity| !entity.deleted)
                })
                .unwrap_or(false);
            if !exists {
                return Ok(ApplyOutcome::Rejected {
                    code: ErrorCode::DependencyMissing,
                    message: format!(
                        "dependency {}:{} does not exist",
                        dependency.entity_type, dependency.entity_id
                    ),
                    field_errors: vec![],
                });
            }
        }

        let key = Self::entity_key(change.entity_type.as_str(), &change.entity_id);
        let stored = user.and_then(|state| state.entities.get(&key));
        let base = change.base_server_version.to_u64().unwrap_or(u64::MAX);

        match change.operation.as_str() {
            ChangeOperation::UPSERT => {
                let Some(payload) = &change.payload else {
                    return Ok(ApplyOutcome::Rejected {
                        code: ErrorCode::InvalidEntityPayload,
                        message: "upsert requires a payload".to_owned(),
                        field_errors: vec![],
                    });
                };
                let parsed = match EntityPayload::try_from((&change.entity_type, payload.clone())) {
                    Ok(value) => value,
                    Err(message) => {
                        return Ok(ApplyOutcome::Rejected {
                            code: ErrorCode::InvalidEntityPayload,
                            message,
                            field_errors: vec![],
                        })
                    }
                };
                if parsed.entity_id() != &change.entity_id {
                    return Ok(ApplyOutcome::Rejected {
                        code: ErrorCode::InvalidEntityPayload,
                        message: "payload entity id does not match change entity id".to_owned(),
                        field_errors: vec![],
                    });
                }

                match stored {
                    Some(entity) if !entity.deleted => {
                        if entity.server_version != base {
                            Ok(ApplyOutcome::Conflict {
                                reason: ConflictReason::new(ConflictReason::BASE_VERSION_MISMATCH),
                                server_entity: Some(entity.payload.clone()),
                                server_deleted: false,
                                current_server_version: entity.server_version,
                                client_base_server_version: base,
                            })
                        } else {
                            Ok(ApplyOutcome::Accepted(AcceptedPlan {
                                server_version: entity.server_version + 1,
                                noop: false,
                            }))
                        }
                    }
                    Some(entity) if entity.deleted => {
                        if entity.server_version != base {
                            Ok(ApplyOutcome::Conflict {
                                reason: ConflictReason::new(
                                    ConflictReason::CLIENT_MODIFIED_SERVER_DELETED,
                                ),
                                server_entity: None,
                                server_deleted: true,
                                current_server_version: entity.server_version,
                                client_base_server_version: base,
                            })
                        } else {
                            Ok(ApplyOutcome::Accepted(AcceptedPlan {
                                server_version: entity.server_version + 1,
                                noop: false,
                            }))
                        }
                    }
                    Some(_) => unreachable!("deleted flag is boolean"),
                    None => {
                        if base != 0 {
                            Ok(ApplyOutcome::Conflict {
                                reason: ConflictReason::new(ConflictReason::BASE_VERSION_MISMATCH),
                                server_entity: None,
                                server_deleted: false,
                                current_server_version: 0,
                                client_base_server_version: base,
                            })
                        } else {
                            Ok(ApplyOutcome::Accepted(AcceptedPlan {
                                server_version: 1,
                                noop: false,
                            }))
                        }
                    }
                }
            }
            ChangeOperation::DELETE => match stored {
                Some(entity) if !entity.deleted => {
                    if entity.server_version != base {
                        Ok(ApplyOutcome::Conflict {
                            reason: ConflictReason::new(ConflictReason::CLIENT_DELETED_SERVER_MODIFIED),
                            server_entity: Some(entity.payload.clone()),
                            server_deleted: false,
                            current_server_version: entity.server_version,
                            client_base_server_version: base,
                        })
                    } else {
                        Ok(ApplyOutcome::Accepted(AcceptedPlan {
                            server_version: entity.server_version + 1,
                            noop: false,
                        }))
                    }
                }
                Some(entity) if entity.deleted => {
                    if entity.server_version != base {
                        Ok(ApplyOutcome::Conflict {
                            reason: ConflictReason::new(ConflictReason::BOTH_DELETED),
                            server_entity: None,
                            server_deleted: true,
                            current_server_version: entity.server_version,
                            client_base_server_version: base,
                        })
                    } else {
                        Ok(ApplyOutcome::Accepted(AcceptedPlan {
                            server_version: entity.server_version,
                            noop: true,
                        }))
                    }
                }
                Some(_) => unreachable!("deleted flag is boolean"),
                None => {
                    if base != 0 {
                        Ok(ApplyOutcome::Conflict {
                            reason: ConflictReason::new(ConflictReason::BASE_VERSION_MISMATCH),
                            server_entity: None,
                            server_deleted: false,
                            current_server_version: 0,
                            client_base_server_version: base,
                        })
                    } else {
                        Ok(ApplyOutcome::Accepted(AcceptedPlan {
                            server_version: 0,
                            noop: true,
                        }))
                    }
                }
            },
            other => Ok(ApplyOutcome::Rejected {
                code: ErrorCode::InvalidEntityPayload,
                message: format!("unsupported operation: {other}"),
                field_errors: vec![],
            }),
        }
    }

    fn apply_outcome(
        &mut self,
        user_id: &UserId,
        change: &SyncChangeV1,
        outcome: ApplyOutcome,
        device_id: DeviceId,
    ) -> PushChangeResultV1 {
        match outcome {
            ApplyOutcome::Duplicate(result) => match result {
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
            },
            ApplyOutcome::Rejected {
                code,
                message,
                field_errors,
            } => PushChangeResultV1::Rejected {
                change_id: change.change_id.clone(),
                entity_type: change.entity_type.clone(),
                entity_id: change.entity_id.clone(),
                code,
                message,
                field_errors,
            },
            ApplyOutcome::Conflict {
                reason,
                server_entity,
                server_deleted,
                current_server_version,
                client_base_server_version,
            } => {
                let result = PushChangeResultV1::Conflict {
                    conflict_id: ConflictId::new(format!("conflict-{}", change.change_id)),
                    change_id: change.change_id.clone(),
                    entity_type: change.entity_type.clone(),
                    entity_id: change.entity_id.clone(),
                    client_base_server_version: ServerVersion::from_u64(client_base_server_version),
                    current_server_version: ServerVersion::from_u64(current_server_version),
                    server_entity,
                    server_deleted,
                    reason,
                };
                let payload_json = change
                    .payload
                    .as_ref()
                    .map(|value| serde_json::to_string(&value.0).unwrap_or_default());
                let user = self.user_mut(user_id);
                user.processed.insert(
                    change.change_id.clone(),
                    StoredChangeResult {
                        result: result.clone(),
                        payload_json,
                    },
                );
                result
            }
            ApplyOutcome::Accepted(plan) => {
                if plan.noop {
                    let latest_cursor = self.latest_cursor(user_id);
                    let result = PushChangeResultV1::Accepted {
                        change_id: change.change_id.clone(),
                        entity_type: change.entity_type.clone(),
                        entity_id: change.entity_id.clone(),
                        server_version: ServerVersion::from_u64(plan.server_version),
                        cursor: latest_cursor,
                        server_modified_at: Self::now(),
                    };
                    let payload_json = change
                        .payload
                        .as_ref()
                        .map(|value| serde_json::to_string(&value.0).unwrap_or_default());
                    let user = self.user_mut(user_id);
                    user.processed.insert(
                        change.change_id.clone(),
                        StoredChangeResult {
                            result: result.clone(),
                            payload_json,
                        },
                    );
                    return result;
                }

                let server_version = plan.server_version;
                let user = self.user_mut(user_id);
                user.next_cursor += 1;
                let cursor = user.next_cursor;
                let key = Self::entity_key(change.entity_type.as_str(), &change.entity_id);
                let now = Self::now();
                let origin_device_id = Some(device_id.clone());

                match change.operation.as_str() {
                    ChangeOperation::UPSERT => {
                        let created_at = user
                            .entities
                            .get(&key)
                            .map(|entity| entity.created_at)
                            .unwrap_or(now);
                        user.entities.insert(
                            key,
                            StoredEntity {
                                entity_type: change.entity_type.clone(),
                                entity_id: change.entity_id.clone(),
                                server_version,
                                payload: change.payload.clone().unwrap(),
                                deleted: false,
                                deleted_at: None,
                                deleted_by_device: None,
                                created_at,
                                updated_at: now,
                                origin_device_id: origin_device_id.clone(),
                            },
                        );
                        user.change_log.push(ServerChangeRecord {
                            cursor,
                            entity_type: change.entity_type.clone(),
                            entity_id: change.entity_id.clone(),
                            operation: ChangeOperation::new(ChangeOperation::UPSERT),
                            server_version: ServerVersion::from_u64(server_version),
                            server_modified_at: now,
                            payload: change.payload.clone(),
                            tombstone: None,
                            origin_device_id,
                        });
                    }
                    ChangeOperation::DELETE => {
                        let entity = user
                            .entities
                            .get_mut(&key)
                            .expect("delete target exists");
                        entity.server_version = server_version;
                        entity.deleted = true;
                        entity.deleted_at = Some(now);
                        entity.deleted_by_device = origin_device_id.clone();
                        entity.updated_at = now;
                        let tombstone = TombstoneV1 {
                            entity_type: change.entity_type.clone(),
                            entity_id: change.entity_id.clone(),
                            deleted_at: now,
                            server_version: ServerVersion::from_u64(server_version),
                            deleted_by_device: origin_device_id.clone(),
                        };
                        user.change_log.push(ServerChangeRecord {
                            cursor,
                            entity_type: change.entity_type.clone(),
                            entity_id: change.entity_id.clone(),
                            operation: ChangeOperation::new(ChangeOperation::DELETE),
                            server_version: ServerVersion::from_u64(server_version),
                            server_modified_at: now,
                            payload: None,
                            tombstone: Some(tombstone),
                            origin_device_id,
                        });
                    }
                    _ => unreachable!("unsupported operations are rejected earlier"),
                }

                let result = PushChangeResultV1::Accepted {
                    change_id: change.change_id.clone(),
                    entity_type: change.entity_type.clone(),
                    entity_id: change.entity_id.clone(),
                    server_version: ServerVersion::from_u64(server_version),
                    cursor: Cursor::new(cursor.to_string()),
                    server_modified_at: now,
                };
                let payload_json = change
                    .payload
                    .as_ref()
                    .map(|value| serde_json::to_string(&value.0).unwrap_or_default());
                user.processed.insert(
                    change.change_id.clone(),
                    StoredChangeResult {
                        result: result.clone(),
                        payload_json,
                    },
                );
                result
            }
        }
    }

    fn prune_change_log(&mut self, user_id: &UserId) {
        let retention_entries = self.config.retention_entries;
        let user = self.user_mut(user_id);
        if user.change_log.len() > retention_entries {
            let excess = user.change_log.len() - retention_entries;
            user.change_log.drain(..excess);
        }
    }

    fn min_valid_cursor(&self, user_id: &UserId) -> u64 {
        self.users
            .get(user_id)
            .and_then(|user| user.change_log.first())
            .map(|record| record.cursor.saturating_sub(1))
            .unwrap_or(0)
    }

    // ------------------------------------------------------------------
    // Pull
    // ------------------------------------------------------------------

    pub fn pull(
        &self,
        user_id: &UserId,
        request: &PullRequestV1,
    ) -> Result<PullResponseV1, ApiError> {
        let user = self.users.get(user_id);
        let after = match &request.after_cursor {
            Some(cursor) => {
                let value = cursor.as_str().parse::<u64>().map_err(|_| {
                    ApiError::new(
                        ErrorCode::CursorInvalid,
                        "cursor is not valid",
                        StatusCode::BAD_REQUEST,
                    )
                })?;
                if value > user.map(|state| state.next_cursor).unwrap_or(0) {
                    return Err(ApiError::new(
                        ErrorCode::CursorInvalid,
                        "cursor is ahead of the server",
                        StatusCode::BAD_REQUEST,
                    ));
                }
                if value < self.min_valid_cursor(user_id) {
                    return Err(ApiError::new(
                        ErrorCode::CursorExpired,
                        "cursor has expired; snapshot is required",
                        StatusCode::GONE,
                    ));
                }
                value
            }
            None => self.min_valid_cursor(user_id).saturating_sub(1),
        };

        let filters: Option<Vec<String>> = request
            .entity_types
            .as_ref()
            .map(|types| types.iter().map(|value| value.as_str().to_owned()).collect());
        let limit = (request.limit as usize)
            .min(self.config.max_pull_batch_size)
            .max(1);
        let mut changes = Vec::new();
        let mut has_more = false;
        for record in user
            .iter()
            .flat_map(|state| state.change_log.iter())
            .filter(|record| record.cursor > after)
        {
            if let Some(filters) = &filters {
                if !filters.iter().any(|value| value == record.entity_type.as_str()) {
                    continue;
                }
            }
            if changes.len() == limit {
                has_more = true;
                break;
            }
            changes.push(ServerChangeV1 {
                cursor: Cursor::new(record.cursor.to_string()),
                entity_type: record.entity_type.clone(),
                entity_id: record.entity_id.clone(),
                operation: record.operation.clone(),
                server_version: record.server_version.clone(),
                server_modified_at: record.server_modified_at,
                payload: record.payload.clone(),
                tombstone: record.tombstone.clone(),
                origin_device_id: record.origin_device_id.clone(),
            });
        }

        let next_cursor = changes
            .last()
            .map(|change| change.cursor.clone())
            .or_else(|| request.after_cursor.clone())
            .unwrap_or_else(|| Cursor::new("0".to_owned()));
        Ok(PullResponseV1 {
            request_id: request.request_id.clone(),
            server_time: Self::now(),
            changes,
            next_cursor,
            has_more,
        })
    }

    // ------------------------------------------------------------------
    // Snapshot
    // ------------------------------------------------------------------

    pub fn snapshot(
        &mut self,
        user_id: &UserId,
        request: &SnapshotRequestV1,
    ) -> Result<SnapshotResponseV1, ApiError> {
        let first_page = request.snapshot_id.is_none();
        let snapshot_id = match &request.snapshot_id {
            Some(id) => id.clone(),
            None => {
                self.snapshot_counter += 1;
                SnapshotId::new(format!("snapshot-{}", self.snapshot_counter))
            }
        };

        let view_cursor = if first_page {
            let filters: Option<Vec<String>> = request
                .entity_types
                .as_ref()
                .map(|types| types.iter().map(|value| value.as_str().to_owned()).collect());
            let mut items: Vec<EntitySnapshotV1> = self
                .users
                .get(user_id)
                .map(|user| {
                    user.entities
                        .iter()
                        .filter(|(_, entity)| !entity.deleted)
                        .filter(|(_, entity)| {
                            filters.as_ref().map(|filters| {
                                filters.iter().any(|value| value == entity.entity_type.as_str())
                            }).unwrap_or(true)
                        })
                        .map(|(_, entity)| EntitySnapshotV1 {
                            entity_type: entity.entity_type.clone(),
                            entity_id: entity.entity_id.clone(),
                            server_version: ServerVersion::from_u64(entity.server_version),
                            payload: entity.payload.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            items.sort_by(|left, right| {
                left.entity_type
                    .as_str()
                    .cmp(right.entity_type.as_str())
                    .then(left.entity_id.as_str().cmp(right.entity_id.as_str()))
            });
            let cursor = self
                .users
                .get(user_id)
                .map(|user| user.next_cursor)
                .unwrap_or(0);
            self.user_mut(user_id).snapshots.insert(
                snapshot_id.clone(),
                StoredSnapshot { cursor, items },
            );
            cursor
        } else {
            let stored = self
                .users
                .get(user_id)
                .and_then(|user| user.snapshots.get(&snapshot_id))
                .ok_or_else(|| {
                    ApiError::new(
                        ErrorCode::InvalidRequest,
                        format!("unknown snapshot id {snapshot_id}"),
                        StatusCode::BAD_REQUEST,
                    )
                })?;
            stored.cursor
        };

        let stored = self
            .users
            .get(user_id)
            .and_then(|user| user.snapshots.get(&snapshot_id))
            .expect("snapshot exists");
        let offset = match &request.page_token {
            None => 0,
            Some(token) => token
                .strip_prefix("page-")
                .and_then(|value| value.parse::<usize>().ok())
                .ok_or_else(|| {
                    ApiError::new(
                        ErrorCode::CursorInvalid,
                        "invalid page token",
                        StatusCode::BAD_REQUEST,
                    )
                })?,
        };
        let page_size = request.page_size.max(1) as usize;
        let end = (offset + page_size).min(stored.items.len());
        let items = stored.items[offset.min(stored.items.len())..end].to_vec();
        let completed = end >= stored.items.len();
        let next_page_token = if completed {
            None
        } else {
            Some(format!("page-{end}"))
        };

        Ok(SnapshotResponseV1 {
            request_id: request.request_id.clone(),
            snapshot_id,
            snapshot_cursor: Cursor::new(view_cursor.to_string()),
            items,
            next_page_token,
            completed,
            server_time: Self::now(),
        })
    }
}

struct AcceptedPlan {
    server_version: u64,
    noop: bool,
}

enum ApplyOutcome {
    Accepted(AcceptedPlan),
    Duplicate(PushChangeResultV1),
    Conflict {
        reason: ConflictReason,
        server_entity: Option<JsonValue>,
        server_deleted: bool,
        current_server_version: u64,
        client_base_server_version: u64,
    },
    Rejected {
        code: ErrorCode,
        message: String,
        field_errors: Vec<FieldError>,
    },
}

/// Keep `ApiErrorV1` referenced so the error model stays part of the public
/// surface of this crate even if unused directly by callers yet.
#[allow(dead_code)]
fn _api_error_type(_: &ApiErrorV1) {}
