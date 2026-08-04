//! In-memory sync server reference implementation (TEST ONLY).
//!
//! This module validates the sync protocol v1 semantics: push idempotency,
//! base-version conflicts, tombstones, pull cursor ordering, snapshot
//! consistency, atomic groups and error codes.
//!
//! It is intentionally NOT a production server: no persistence, no auth, no
//! device registration, no networking. Production code must never use it.

use std::collections::HashMap;

use chrono::Utc;

use crate::domain::payload::EntityPayload;
use crate::error::{ApiErrorV1, ErrorCode, FieldError};
use crate::ids::{
    AtomicGroupId, ChangeId, ConflictId, Cursor, DeviceId, EntityId, ServerVersion, SnapshotId,
    UserId,
};
use crate::json_value::JsonValue;
use crate::registry::{EntityType, REGISTRY};
use crate::sync::v1::*;
use crate::time::UtcTimestamp;

/// API-level error raised by the reference implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct TestKitError {
    pub http_status: u16,
    pub error: ApiErrorV1,
}

impl TestKitError {
    pub fn new(code: ErrorCode, message: impl Into<String>, http_status: u16) -> Self {
        Self {
            http_status,
            error: ApiErrorV1::new(code, message),
        }
    }
}

/// Current stored server state of one entity.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct StoredChangeResult {
    pub result: PushChangeResultV1,
    pub payload_json: Option<String>,
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

/// In-memory sync server state machine.
#[derive(Debug)]
pub struct SyncServer {
    user_id: UserId,
    entities: HashMap<(String, EntityId), StoredEntity>,
    processed: HashMap<(UserId, ChangeId), StoredChangeResult>,
    change_log: Vec<ServerChangeRecord>,
    next_cursor: u64,
    snapshots: HashMap<SnapshotId, StoredSnapshot>,
    snapshot_counter: u64,
    retention_entries: usize,
    max_push_batch_size: usize,
    max_pull_batch_size: usize,
    max_request_bytes: usize,
    max_snapshot_page_size: usize,
    max_atomic_group_size: usize,
}

impl Default for SyncServer {
    fn default() -> Self {
        Self::new(UserId::new("test-user"))
    }
}

impl SyncServer {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            entities: HashMap::new(),
            processed: HashMap::new(),
            change_log: Vec::new(),
            next_cursor: 0,
            snapshots: HashMap::new(),
            snapshot_counter: 0,
            retention_entries: 1000,
            max_push_batch_size: 500,
            max_pull_batch_size: 200,
            max_request_bytes: 4 * 1024 * 1024,
            max_snapshot_page_size: 200,
            max_atomic_group_size: 50,
        }
    }

    /// Override the change-log retention (entries kept) to test cursor expiry.
    pub fn with_retention(mut self, entries: usize) -> Self {
        self.retention_entries = entries;
        self
    }

    pub fn capabilities(&self) -> CapabilitiesResponseV1 {
        CapabilitiesResponseV1 {
            protocol_version: crate::PROTOCOL_VERSION,
            supported_protocol_versions: vec![1],
            schema_version: crate::SCHEMA_VERSION,
            minimum_schema_version: crate::MINIMUM_SCHEMA_VERSION,
            minimum_client_versions: vec![MinimumClientVersion {
                app_id: AppId::new(AppId::DESKTOP),
                client_version: "0.2.1".to_owned(),
            }],
            maximum_push_batch_size: self.max_push_batch_size as u32,
            maximum_pull_batch_size: self.max_pull_batch_size as u32,
            maximum_request_bytes: self.max_request_bytes as u64,
            maximum_snapshot_page_size: self.max_snapshot_page_size as u32,
            maximum_atomic_group_size: self.max_atomic_group_size as u32,
            tombstone_retention_days: 90,
            supported_entity_types: REGISTRY
                .iter()
                .map(|descriptor| descriptor.entity_type.to_owned())
                .collect(),
            server_time: Utc::now(),
        }
    }

    /// Current stored state for a key (test helper).
    pub fn entity(&self, entity_type: &str, entity_id: &str) -> Option<&StoredEntity> {
        self.entities.get(&(entity_type.to_owned(), EntityId::new(entity_id)))
    }

    /// Recorded result for a changeId (test helper).
    pub fn processed(&self, change_id: &str) -> Option<&StoredChangeResult> {
        self.processed
            .get(&(self.user_id.clone(), ChangeId::new(change_id)))
    }

    pub fn latest_cursor(&self) -> Cursor {
        Cursor::new(self.next_cursor.to_string())
    }

    pub fn change_count(&self) -> usize {
        self.change_log.len()
    }

    fn now(&self) -> UtcTimestamp {
        Utc::now()
    }

    fn entity_key(entity_type: &str, entity_id: &EntityId) -> (String, EntityId) {
        (entity_type.to_owned(), entity_id.clone())
    }

    // ------------------------------------------------------------------
    // Push
    // ------------------------------------------------------------------

    pub fn push(&mut self, request: &PushRequestV1) -> Result<PushResponseV1, TestKitError> {
        if request.client.protocol_version != 1 {
            return Err(TestKitError::new(
                ErrorCode::ProtocolUnsupported,
                format!(
                    "protocol version {} is not supported",
                    request.client.protocol_version
                ),
                426,
            ));
        }
        if request.client.schema_version < crate::MINIMUM_SCHEMA_VERSION {
            return Err(TestKitError::new(
                ErrorCode::SchemaUnsupported,
                format!(
                    "schema version {} is below minimum {}",
                    request.client.schema_version,
                    crate::MINIMUM_SCHEMA_VERSION
                ),
                400,
            ));
        }
        if request.changes.len() > self.max_push_batch_size {
            return Err(TestKitError::new(
                ErrorCode::BatchTooLarge,
                format!(
                    "push batch of {} exceeds maximum {}",
                    request.changes.len(),
                    self.max_push_batch_size
                ),
                400,
            ));
        }
        let wire_size = serde_json::to_vec(request)
            .map_err(|error| {
                TestKitError::new(ErrorCode::InternalError, error.to_string(), 500)
            })?
            .len();
        if wire_size > self.max_request_bytes {
            return Err(TestKitError::new(
                ErrorCode::PayloadTooLarge,
                format!("request body of {wire_size} bytes exceeds maximum"),
                413,
            ));
        }

        // Atomic groups must fit within the maximum group size.
        let mut group_sizes: HashMap<Option<&AtomicGroupId>, usize> = HashMap::new();
        for change in &request.changes {
            *group_sizes.entry(change.atomic_group_id.as_ref()).or_insert(0) += 1;
        }
        for (group, size) in &group_sizes {
            if let Some(group_id) = group {
                if *size > self.max_atomic_group_size {
                    return Err(TestKitError::new(
                        ErrorCode::BatchTooLarge,
                        format!(
                            "atomic group {group_id} has {size} changes (maximum {})",
                            self.max_atomic_group_size
                        ),
                        400,
                    ));
                }
            }
        }

        // Group change indices by atomic_group_id so groups are all-or-nothing.
        let mut group_indices: HashMap<Option<&AtomicGroupId>, Vec<usize>> = HashMap::new();
        for (index, change) in request.changes.iter().enumerate() {
            group_indices
                .entry(change.atomic_group_id.as_ref())
                .or_default()
                .push(index);
        }

        let mut results: Vec<PushChangeResultV1> = Vec::with_capacity(request.changes.len());
        // Process groups (non-grouped changes are groups of one).
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
                let outcome = self.evaluate_change(change)?;
                results.push(self.apply_outcome(change, outcome, request.client.device_id.clone()));
            } else {
                // Evaluate the whole group first; any failure fails the group.
                let mut outcomes = Vec::with_capacity(indices.len());
                let mut failure: Option<String> = None;
                for index in indices {
                    let change = &request.changes[*index];
                    match self.evaluate_change(change) {
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
                        results.push(self.apply_outcome(change, outcome, request.client.device_id.clone()));
                    }
                }
            }
        }

        self.prune_change_log();
        Ok(PushResponseV1 {
            request_id: request.request_id.clone(),
            server_time: self.now(),
            results,
            latest_cursor: self.latest_cursor(),
        })
    }

    /// Evaluate one change without mutating state.
    fn evaluate_change(&self, change: &SyncChangeV1) -> Result<ApplyOutcome, TestKitError> {
        // Idempotency first: same changeId + same payload replays the first
        // result; same changeId + different payload is LIFETRACE_CHANGE_ID_REUSE.
        if let Some(stored) = self.processed.get(&(self.user_id.clone(), change.change_id.clone())) {
            let incoming_payload = change.payload.as_ref().map(|value| serde_json::to_string(&value.0).unwrap_or_default());
            return if stored.payload_json == incoming_payload {
                Ok(ApplyOutcome::Duplicate(stored.result.clone()))
            } else {
                Ok(ApplyOutcome::Rejected {
                    code: ErrorCode::ChangeIdReuse,
                    message: format!("changeId {} was already used with a different payload", change.change_id),
                    field_errors: vec![],
                })
            };
        }

        // Registry check: unknown entity types are rejected per change.
        let Some(descriptor) = crate::registry::describe(change.entity_type.as_str()) else {
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

        // Dependencies must exist and not be deleted.
        for dependency in &change.dependencies {
            let exists = self
                .entities
                .get(&Self::entity_key(dependency.entity_type.as_str(), &dependency.entity_id))
                .is_some_and(|entity| !entity.deleted);
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
        let stored = self.entities.get(&key);
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
                                reason: ConflictReason::new(ConflictReason::CLIENT_MODIFIED_SERVER_DELETED),
                                server_entity: None,
                                server_deleted: true,
                                current_server_version: entity.server_version,
                                client_base_server_version: base,
                            })
                        } else {
                            // Explicit restore based on the tombstone version.
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
            ChangeOperation::DELETE => {
                match stored {
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
                            // Both sides deleted with matching versions: idempotent
                            // success, no new change log entry.
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
                }
            }
            other => Ok(ApplyOutcome::Rejected {
                code: ErrorCode::InvalidEntityPayload,
                message: format!("unsupported operation: {other}"),
                field_errors: vec![],
            }),
        }
    }

    fn apply_outcome(
        &mut self,
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
            ApplyOutcome::Rejected { code, message, field_errors } => PushChangeResultV1::Rejected {
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
                let payload_json = change.payload.as_ref().map(|value| {
                    serde_json::to_string(&value.0).unwrap_or_default()
                });
                self.processed.insert(
                    (self.user_id.clone(), change.change_id.clone()),
                    StoredChangeResult {
                        result: result.clone(),
                        payload_json,
                    },
                );
                result
            }
            ApplyOutcome::Accepted(plan) => {
                if plan.noop {
                    let result = PushChangeResultV1::Accepted {
                        change_id: change.change_id.clone(),
                        entity_type: change.entity_type.clone(),
                        entity_id: change.entity_id.clone(),
                        server_version: ServerVersion::from_u64(plan.server_version),
                        cursor: self.latest_cursor(),
                        server_modified_at: self.now(),
                    };
                    self.processed.insert(
                        (self.user_id.clone(), change.change_id.clone()),
                        StoredChangeResult {
                            result: result.clone(),
                            payload_json: change.payload.as_ref().map(|value| {
                                serde_json::to_string(&value.0).unwrap_or_default()
                            }),
                        },
                    );
                    return result;
                }

                let server_version = plan.server_version;
                self.next_cursor += 1;
                let cursor = self.next_cursor;
                let key = Self::entity_key(change.entity_type.as_str(), &change.entity_id);
                let now = self.now();
                let origin_device_id = Some(device_id.clone());

                match change.operation.as_str() {
                    ChangeOperation::UPSERT => {
                        let created_at = self
                            .entities
                            .get(&key)
                            .map(|entity| entity.created_at)
                            .unwrap_or(now);
                        self.entities.insert(
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
                        self.change_log.push(ServerChangeRecord {
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
                        let entity = self.entities.get_mut(&key).expect("delete target exists");
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
                        self.change_log.push(ServerChangeRecord {
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
                let payload_json = change.payload.as_ref().map(|value| {
                    serde_json::to_string(&value.0).unwrap_or_default()
                });
                self.processed.insert(
                    (self.user_id.clone(), change.change_id.clone()),
                    StoredChangeResult {
                        result: result.clone(),
                        payload_json,
                    },
                );
                result
            }
        }
    }

    fn prune_change_log(&mut self) {
        if self.change_log.len() > self.retention_entries {
            let excess = self.change_log.len() - self.retention_entries;
            self.change_log.drain(..excess);
        }
    }

    fn min_valid_cursor(&self) -> u64 {
        // The client may request "after" the cursor just before the first
        // retained record; anything older has been pruned.
        self.change_log
            .first()
            .map(|record| record.cursor.saturating_sub(1))
            .unwrap_or(0)
    }

    // ------------------------------------------------------------------
    // Pull
    // ------------------------------------------------------------------

    pub fn pull(&self, request: &PullRequestV1) -> Result<PullResponseV1, TestKitError> {
        let after = match &request.after_cursor {
            Some(cursor) => {
                let value = cursor.as_str().parse::<u64>().map_err(|_| {
                    TestKitError::new(ErrorCode::CursorInvalid, "cursor is not valid", 400)
                })?;
                if value > self.next_cursor {
                    return Err(TestKitError::new(
                        ErrorCode::CursorInvalid,
                        "cursor is ahead of the server",
                        400,
                    ));
                }
                if value < self.min_valid_cursor() {
                    return Err(TestKitError::new(
                        ErrorCode::CursorExpired,
                        "cursor has expired; snapshot is required",
                        410,
                    ));
                }
                value
            }
            None => self.min_valid_cursor().saturating_sub(1),
        };

        let filters: Option<Vec<String>> = request
            .entity_types
            .as_ref()
            .map(|types| types.iter().map(|value| value.as_str().to_owned()).collect());
        let limit = (request.limit as usize).min(self.max_pull_batch_size).max(1);
        let mut changes = Vec::new();
        let mut has_more = false;
        for record in self.change_log.iter().filter(|record| record.cursor > after) {
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
            server_time: self.now(),
            changes,
            next_cursor,
            has_more,
        })
    }

    // ------------------------------------------------------------------
    // Snapshot
    // ------------------------------------------------------------------

    pub fn snapshot(&mut self, request: &SnapshotRequestV1) -> Result<SnapshotResponseV1, TestKitError> {
        let first_page = request.snapshot_id.is_none();
        let snapshot_id = match &request.snapshot_id {
            Some(id) => id.clone(),
            None => {
                self.snapshot_counter += 1;
                SnapshotId::new(format!("snapshot-{}", self.snapshot_counter))
            }
        };

        let view_cursor = if first_page {
            // Capture a consistent view at this instant.
            let filters: Option<Vec<String>> = request
                .entity_types
                .as_ref()
                .map(|types| types.iter().map(|value| value.as_str().to_owned()).collect());
            let mut items: Vec<EntitySnapshotV1> = self
                .entities
                .iter()
                .filter(|(_, entity)| !entity.deleted)
                .filter(|(_, entity)| {
                    filters
                        .as_ref()
                        .map(|filters| filters.iter().any(|value| value == entity.entity_type.as_str()))
                        .unwrap_or(true)
                })
                .map(|(_, entity)| EntitySnapshotV1 {
                    entity_type: entity.entity_type.clone(),
                    entity_id: entity.entity_id.clone(),
                    server_version: ServerVersion::from_u64(entity.server_version),
                    payload: entity.payload.clone(),
                })
                .collect();
            items.sort_by(|left, right| {
                left.entity_type
                    .as_str()
                    .cmp(right.entity_type.as_str())
                    .then(left.entity_id.as_str().cmp(right.entity_id.as_str()))
            });
            let cursor = self.next_cursor;
            self.snapshots.insert(
                snapshot_id.clone(),
                StoredSnapshot {
                    cursor,
                    items,
                },
            );
            cursor
        } else {
            let stored = self.snapshots.get(&snapshot_id).ok_or_else(|| {
                TestKitError::new(
                    ErrorCode::InvalidRequest,
                    format!("unknown snapshot id {snapshot_id}"),
                    400,
                )
            })?;
            stored.cursor
        };

        let stored = self.snapshots.get(&snapshot_id).expect("snapshot exists");
        let offset = match &request.page_token {
            None => 0,
            Some(token) => token
                .strip_prefix("page-")
                .and_then(|value| value.parse::<usize>().ok())
                .ok_or_else(|| {
                    TestKitError::new(
                        ErrorCode::CursorInvalid,
                        "invalid page token",
                        400,
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
            server_time: self.now(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use crate::domain::*;

    fn stamp() -> UtcTimestamp {
        "2026-08-04T15:30:00Z".parse().unwrap()
    }

    fn meta(id: &str) -> EntityMeta {
        EntityMeta {
            id: EntityId::new(id),
            user_id: UserId::new("test-user"),
            created_at: stamp(),
            updated_at: stamp(),
            deleted_at: None,
            local_version: 1,
            server_version: None,
            modified_by_device: None,
        }
    }

    fn client() -> SyncClientInfo {
        SyncClientInfo {
            app_id: AppId::new(AppId::DESKTOP),
            client_version: "0.2.1".to_owned(),
            platform: ClientPlatform::new(ClientPlatform::WINDOWS),
            protocol_version: 1,
            schema_version: 1,
            device_id: DeviceId::new("device-1"),
        }
    }

    fn upsert_change(
        change_id: &str,
        entity_id: &str,
        amount_cents: i64,
        base: u64,
        group: Option<&str>,
    ) -> SyncChangeV1 {
        let transaction = Transaction {
            meta: meta(entity_id),
            transaction_type: TransactionType::new(TransactionType::EXPENSE),
            amount_cents,
            currency: CurrencyCode::cny(),
            account_id: None,
            to_account_id: None,
            category_id: None,
            counterparty: None,
            merchant: None,
            item: None,
            note: None,
            occurred_at: stamp(),
            local_date: LocalDate::new("2026-08-04").unwrap(),
            status: TransactionStatus::new(TransactionStatus::CONFIRMED),
            source_type: "manual".to_owned(),
            external_transaction_id: None,
        };
        SyncChangeV1 {
            change_id: ChangeId::new(change_id),
            entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
            entity_id: EntityId::new(entity_id),
            operation: ChangeOperation::new(ChangeOperation::UPSERT),
            base_server_version: ServerVersion::from_u64(base),
            entity_schema_version: 1,
            client_modified_at: stamp(),
            payload: Some(serde_json::to_value(transaction).unwrap().into()),
            atomic_group_id: group.map(AtomicGroupId::new),
            dependencies: vec![],
        }
    }

    fn delete_change(change_id: &str, entity_id: &str, base: u64) -> SyncChangeV1 {
        SyncChangeV1 {
            change_id: ChangeId::new(change_id),
            entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
            entity_id: EntityId::new(entity_id),
            operation: ChangeOperation::new(ChangeOperation::DELETE),
            base_server_version: ServerVersion::from_u64(base),
            entity_schema_version: 1,
            client_modified_at: stamp(),
            payload: None,
            atomic_group_id: None,
            dependencies: vec![],
        }
    }

    fn push_request(changes: Vec<SyncChangeV1>) -> PushRequestV1 {
        PushRequestV1 {
            request_id: RequestId::new("req-1"),
            client: client(),
            changes,
        }
    }

    fn accepted_version(result: &PushChangeResultV1) -> u64 {
        match result {
            PushChangeResultV1::Accepted { server_version, .. }
            | PushChangeResultV1::Duplicate { server_version, .. } => {
                server_version.to_u64().unwrap()
            }
            _ => panic!("expected accepted result, got {result:?}"),
        }
    }

    #[test]
    fn create() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        let response = server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(accepted_version(&response.results[0]), 1);
        let entity = server.entity(EntityType::FINANCE_TRANSACTION, "tx-1").unwrap();
        assert!(!entity.deleted);
        assert_eq!(entity.server_version, 1);
    }

    #[test]
    fn update() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        let response = server
            .push(&push_request(vec![upsert_change("c2", "tx-1", 200, 1, None)]))
            .unwrap();
        assert_eq!(accepted_version(&response.results[0]), 2);
        let entity = server.entity(EntityType::FINANCE_TRANSACTION, "tx-1").unwrap();
        assert_eq!(entity.server_version, 2);
        let payload: Transaction = serde_json::from_value(entity.payload.0.clone()).unwrap();
        assert_eq!(payload.amount_cents, 200);
    }

    #[test]
    fn base_version_conflict() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        server.push(&push_request(vec![upsert_change("c2", "tx-1", 200, 1, None)])).unwrap();
        let response = server
            .push(&push_request(vec![upsert_change("c3", "tx-1", 300, 1, None)]))
            .unwrap();
        match &response.results[0] {
            PushChangeResultV1::Conflict {
                reason,
                current_server_version,
                server_entity,
                server_deleted,
                ..
            } => {
                assert_eq!(reason.as_str(), ConflictReason::BASE_VERSION_MISMATCH);
                assert_eq!(current_server_version.to_u64(), Some(2));
                assert!(!server_deleted);
                let current: Transaction =
                    serde_json::from_value(server_entity.clone().unwrap().0).unwrap();
                assert_eq!(current.amount_cents, 200);
            }
            other => panic!("expected conflict, got {other:?}"),
        }
        // Nothing was written.
        let entity = server.entity(EntityType::FINANCE_TRANSACTION, "tx-1").unwrap();
        assert_eq!(entity.server_version, 2);
    }

    #[test]
    fn duplicate_change_id() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        let first = server
            .push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)]))
            .unwrap();
        let second = server
            .push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)]))
            .unwrap();
        assert!(matches!(second.results[0], PushChangeResultV1::Duplicate { .. }));
        assert_eq!(
            accepted_version(&second.results[0]),
            accepted_version(&first.results[0])
        );
        assert_eq!(server.change_count(), 1);
        assert_eq!(
            server.entity(EntityType::FINANCE_TRANSACTION, "tx-1").unwrap().server_version,
            1
        );
    }

    #[test]
    fn change_id_reuse_with_different_payload_is_rejected() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        let response = server
            .push(&push_request(vec![upsert_change("c1", "tx-1", 999, 0, None)]))
            .unwrap();
        match &response.results[0] {
            PushChangeResultV1::Rejected { code, .. } => {
                assert_eq!(*code, ErrorCode::ChangeIdReuse);
            }
            other => panic!("expected rejection, got {other:?}"),
        }
        assert_eq!(server.change_count(), 1);
    }

    #[test]
    fn delete_generates_tombstone() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        let response = server
            .push(&push_request(vec![delete_change("c2", "tx-1", 1)]))
            .unwrap();
        assert_eq!(accepted_version(&response.results[0]), 2);
        let entity = server.entity(EntityType::FINANCE_TRANSACTION, "tx-1").unwrap();
        assert!(entity.deleted);
        assert!(entity.deleted_at.is_some());
        let pull = server
            .pull(&PullRequestV1 {
                request_id: RequestId::new("req-pull"),
                client: client(),
                after_cursor: Some(Cursor::new("0")),
                limit: 100,
                entity_types: None,
            })
            .unwrap();
        assert_eq!(pull.changes.len(), 2);
        let delete = &pull.changes[1];
        assert_eq!(delete.operation.as_str(), ChangeOperation::DELETE);
        assert!(delete.tombstone.is_some());
        assert_eq!(delete.tombstone.as_ref().unwrap().server_version.to_u64(), Some(2));
    }

    #[test]
    fn both_delete_is_idempotent() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        server.push(&push_request(vec![delete_change("c2", "tx-1", 1)])).unwrap();
        // Second delete with the tombstone version: idempotent success.
        let second = server
            .push(&push_request(vec![delete_change("c3", "tx-1", 2)]))
            .unwrap();
        assert_eq!(accepted_version(&second.results[0]), 2);
        assert_eq!(server.change_count(), 2, "no new change log entry for both-delete");
        // Second delete with a stale version: explicit both_deleted conflict.
        let stale = server
            .push(&push_request(vec![delete_change("c4", "tx-1", 1)]))
            .unwrap();
        match &stale.results[0] {
            PushChangeResultV1::Conflict { reason, server_deleted, .. } => {
                assert_eq!(reason.as_str(), ConflictReason::BOTH_DELETED);
                assert!(*server_deleted);
            }
            other => panic!("expected conflict, got {other:?}"),
        }
    }

    #[test]
    fn pull_is_ordered_by_cursor() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        for (index, change_id) in ["a", "b", "c"].iter().enumerate() {
            server
                .push(&push_request(vec![upsert_change(
                    change_id,
                    &format!("tx-{index}"),
                    100,
                    0,
                    None,
                )]))
                .unwrap();
        }
        let response = server
            .pull(&PullRequestV1 {
                request_id: RequestId::new("req-pull"),
                client: client(),
                after_cursor: None,
                limit: 100,
                entity_types: None,
            })
            .unwrap();
        let cursors: Vec<u64> = response
            .changes
            .iter()
            .map(|change| change.cursor.as_str().parse().unwrap())
            .collect();
        assert_eq!(cursors, vec![1, 2, 3]);
        assert!(!response.has_more);
        assert_eq!(response.next_cursor.as_str(), "3");
    }

    #[test]
    fn pull_pagination_has_no_gaps_or_duplicates() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        for index in 0..7 {
            server
                .push(&push_request(vec![upsert_change(
                    &format!("c{index}"),
                    &format!("tx-{index}"),
                    100,
                    0,
                    None,
                )]))
                .unwrap();
        }
        let mut seen = Vec::new();
        let mut after = None;
        loop {
            let page = server
                .pull(&PullRequestV1 {
                    request_id: RequestId::new("req-pull"),
                    client: client(),
                    after_cursor: after,
                    limit: 3,
                    entity_types: None,
                })
                .unwrap();
            assert!(page.changes.len() <= 3);
            for change in &page.changes {
                assert!(!seen.contains(&change.cursor), "no duplicate cursors");
                seen.push(change.cursor.clone());
            }
            after = Some(page.next_cursor.clone());
            if !page.has_more {
                break;
            }
        }
        assert_eq!(seen.len(), 7);
        let cursors: Vec<u64> = seen.iter().map(|value| value.as_str().parse().unwrap()).collect();
        assert!(cursors.windows(2).all(|pair| pair[0] < pair[1]), "strictly ascending");
    }

    #[test]
    fn snapshot_is_consistent_across_pages() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        for index in 0..5 {
            server
                .push(&push_request(vec![upsert_change(
                    &format!("c{index}"),
                    &format!("tx-{index}"),
                    100,
                    0,
                    None,
                )]))
                .unwrap();
        }
        let first = server
            .snapshot(&SnapshotRequestV1 {
                request_id: RequestId::new("req-snap"),
                client: client(),
                snapshot_id: None,
                page_token: None,
                entity_types: None,
                page_size: 2,
            })
            .unwrap();
        let snapshot_id = first.snapshot_id.clone();
        let snapshot_cursor = first.snapshot_cursor.clone();
        assert_eq!(first.items.len(), 2);
        assert!(!first.completed);

        let mut page = first;
        let mut all_items = page.items.clone();
        while let Some(token) = page.next_page_token.clone() {
            page = server
                .snapshot(&SnapshotRequestV1 {
                    request_id: RequestId::new("req-snap"),
                    client: client(),
                    snapshot_id: Some(snapshot_id.clone()),
                    page_token: Some(token),
                    entity_types: None,
                    page_size: 2,
                })
                .unwrap();
            assert_eq!(page.snapshot_id, snapshot_id);
            assert_eq!(page.snapshot_cursor, snapshot_cursor);
            all_items.extend(page.items.clone());
        }
        assert!(page.completed);
        assert_eq!(all_items.len(), 5);
        assert_eq!(snapshot_cursor.as_str(), "5");
    }

    #[test]
    fn snapshot_then_pull_has_no_gaps_for_concurrent_changes() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        let snapshot = server
            .snapshot(&SnapshotRequestV1 {
                request_id: RequestId::new("req-snap"),
                client: client(),
                snapshot_id: None,
                page_token: None,
                entity_types: None,
                page_size: 100,
            })
            .unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert!(snapshot.completed);
        assert_eq!(snapshot.snapshot_cursor.as_str(), "1");

        // A concurrent change lands after the snapshot cursor.
        server.push(&push_request(vec![upsert_change("c2", "tx-2", 200, 0, None)])).unwrap();
        let pull = server
            .pull(&PullRequestV1 {
                request_id: RequestId::new("req-pull"),
                client: client(),
                after_cursor: Some(snapshot.snapshot_cursor.clone()),
                limit: 100,
                entity_types: None,
            })
            .unwrap();
        let changes: Vec<&str> = pull
            .changes
            .iter()
            .map(|change| change.entity_id.as_str())
            .collect();
        assert_eq!(changes, vec!["tx-2"], "no gap: concurrent change is pulled");
    }

    #[test]
    fn atomic_group_fails_together() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        server.push(&push_request(vec![upsert_change("c1", "tx-1", 100, 0, None)])).unwrap();
        // Group: tx-2 create + tx-1 update with a stale base -> whole group fails.
        let group = vec![
            upsert_change("c2", "tx-2", 200, 0, Some("group-1")),
            upsert_change("c3", "tx-1", 300, 0, Some("group-1")),
        ];
        let response = server.push(&push_request(group)).unwrap();
        for result in &response.results {
            match result {
                PushChangeResultV1::Rejected { code, .. } => {
                    assert_eq!(*code, ErrorCode::AtomicGroupFailed);
                }
                other => panic!("expected group failure, got {other:?}"),
            }
        }
        assert!(server.entity(EntityType::FINANCE_TRANSACTION, "tx-2").is_none());
        assert_eq!(
            server.entity(EntityType::FINANCE_TRANSACTION, "tx-1").unwrap().server_version,
            1
        );
    }

    #[test]
    fn atomic_group_succeeds_together() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        let group = vec![
            upsert_change("c1", "tx-1", 100, 0, Some("group-1")),
            upsert_change("c2", "tx-2", 200, 0, Some("group-1")),
        ];
        let response = server.push(&push_request(group)).unwrap();
        assert!(response.results.iter().all(|result| {
            matches!(result, PushChangeResultV1::Accepted { .. })
        }));
        assert!(server.entity(EntityType::FINANCE_TRANSACTION, "tx-1").is_some());
        assert!(server.entity(EntityType::FINANCE_TRANSACTION, "tx-2").is_some());
    }

    #[test]
    fn unknown_entity_type_is_rejected() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        let mut change = upsert_change("c1", "tx-1", 100, 0, None);
        change.entity_type = EntityType::new("future.thing");
        let response = server.push(&push_request(vec![change])).unwrap();
        match &response.results[0] {
            PushChangeResultV1::Rejected { code, .. } => {
                assert_eq!(*code, ErrorCode::UnknownEntityType);
            }
            other => panic!("expected rejection, got {other:?}"),
        }
        assert_eq!(server.change_count(), 0);
    }

    #[test]
    fn expired_cursor_requires_snapshot() {
        let mut server = SyncServer::new(UserId::new("test-user")).with_retention(3);
        for index in 0..5 {
            server
                .push(&push_request(vec![upsert_change(
                    &format!("c{index}"),
                    &format!("tx-{index}"),
                    100,
                    0,
                    None,
                )]))
                .unwrap();
        }
        let error = server
            .pull(&PullRequestV1 {
                request_id: RequestId::new("req-pull"),
                client: client(),
                after_cursor: Some(Cursor::new("0")),
                limit: 100,
                entity_types: None,
            })
            .unwrap_err();
        assert_eq!(error.error.code, ErrorCode::CursorExpired);
    }

    #[test]
    fn unsupported_protocol_is_rejected() {
        let mut server = SyncServer::new(UserId::new("test-user"));
        let mut request = push_request(vec![]);
        request.client.protocol_version = 99;
        let error = server.push(&request).unwrap_err();
        assert_eq!(error.error.code, ErrorCode::ProtocolUnsupported);
        assert_eq!(error.http_status, 426);
    }
}
