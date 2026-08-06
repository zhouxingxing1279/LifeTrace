use std::collections::VecDeque;

use lifetrace_contracts::sync::v1::{PushChangeResultV1, PushRequestV1, SyncClientInfo};
use lifetrace_contracts::RequestId;

use crate::{FailureClass, LeasedChange, PersistedConflict, SyncError, SyncStore, SyncTransport};

pub(crate) async fn run_push<T: SyncTransport, S: SyncStore>(
    transport: &T,
    store: &S,
    client: &SyncClientInfo,
    changes: &[LeasedChange],
) -> Result<usize, SyncError> {
    if changes.is_empty() {
        return Ok(0);
    }
    let mut queue = VecDeque::from([changes.to_vec()]);
    let mut confirmed = 0usize;
    while let Some(batch) = queue.pop_front() {
        let request = PushRequestV1 {
            request_id: RequestId::new(format!("push-{}", batch[0].change.change_id)),
            client: client.clone(),
            changes: batch.iter().map(|item| item.change.clone()).collect(),
        };
        let response = match transport.push(request).await {
            Ok(value) => value,
            Err(error) if error.class == FailureClass::PayloadTooLarge => {
                if let Some((left, right)) = split_without_breaking_atomic_group(&batch) {
                    queue.push_front(right);
                    queue.push_front(left);
                    continue;
                }
                for item in &batch {
                    store
                        .mark_dead_letter(
                            item.change.change_id.as_str(),
                            "SYNC_ATOMIC_GROUP_TOO_LARGE",
                            "single change or atomic group exceeds the server request limit",
                        )
                        .await?;
                }
                continue;
            }
            Err(error)
                if matches!(
                    error.class,
                    FailureClass::PermissionDenied | FailureClass::UpgradeRequired
                ) =>
            {
                for item in &batch {
                    store
                        .mark_blocked(item.change.change_id.as_str(), &error.code, &error.message)
                        .await?;
                }
                return Err(error);
            }
            Err(error) => {
                let ids: Vec<String> = batch
                    .iter()
                    .map(|item| item.change.change_id.to_string())
                    .collect();
                store.release_lease(&ids, Some(&error)).await?;
                return Err(error);
            }
        };
        for result in response.results {
            match result {
                PushChangeResultV1::Accepted {
                    change_id,
                    server_version,
                    cursor,
                    ..
                }
                | PushChangeResultV1::Duplicate {
                    change_id,
                    server_version,
                    cursor,
                    ..
                } => {
                    store
                        .mark_confirmed(
                            change_id.as_str(),
                            server_version.as_str(),
                            cursor.as_str(),
                        )
                        .await?;
                    confirmed += 1;
                }
                PushChangeResultV1::Conflict {
                    conflict_id,
                    change_id,
                    entity_type,
                    entity_id,
                    client_base_server_version,
                    current_server_version,
                    server_entity,
                    server_deleted,
                    reason,
                } => {
                    let local_payload = batch
                        .iter()
                        .find(|item| item.change.change_id == change_id)
                        .and_then(|item| item.local_payload_json.clone())
                        .map(lifetrace_contracts::json_value::JsonValue);
                    store
                        .persist_conflict(PersistedConflict {
                            conflict_id,
                            change_id: Some(change_id),
                            entity_type,
                            entity_id,
                            base_version: client_base_server_version,
                            server_version: current_server_version,
                            local_payload,
                            remote_payload: server_entity,
                            server_deleted,
                            kind: reason.as_str().to_owned(),
                        })
                        .await?;
                }
                PushChangeResultV1::Rejected {
                    change_id,
                    code,
                    message,
                    ..
                } => {
                    let code_string = code.wire_name().to_owned();
                    match code_string.as_str() {
                        "LIFETRACE_AUTH_SCOPE_DENIED" => {
                            store
                                .mark_blocked(change_id.as_str(), &code_string, &message)
                                .await?
                        }
                        "LIFETRACE_UNKNOWN_ENTITY_TYPE"
                        | "LIFETRACE_INVALID_ENTITY_PAYLOAD"
                        | "LIFETRACE_SCHEMA_UNSUPPORTED"
                        | "LIFETRACE_PROTOCOL_UNSUPPORTED" => {
                            store
                                .mark_dead_letter(change_id.as_str(), &code_string, &message)
                                .await?
                        }
                        _ => {
                            store
                                .mark_blocked(change_id.as_str(), &code_string, &message)
                                .await?
                        }
                    }
                }
            }
        }
    }
    Ok(confirmed)
}

fn split_without_breaking_atomic_group(
    batch: &[LeasedChange],
) -> Option<(Vec<LeasedChange>, Vec<LeasedChange>)> {
    if batch.len() < 2 {
        return None;
    }
    let midpoint = batch.len() / 2;
    let mut boundaries = (1..batch.len()).collect::<Vec<_>>();
    boundaries.sort_by_key(|index| index.abs_diff(midpoint));
    boundaries.into_iter().find_map(|index| {
        let left_group = batch[index - 1]
            .change
            .atomic_group_id
            .as_ref()
            .map(|value| value.as_str());
        let right_group = batch[index]
            .change
            .atomic_group_id
            .as_ref()
            .map(|value| value.as_str());
        if left_group.is_some() && left_group == right_group {
            None
        } else {
            Some((batch[..index].to_vec(), batch[index..].to_vec()))
        }
    })
}

pub fn classify_http_status(status: u16, retry_after_seconds: Option<u64>) -> FailureClass {
    match status {
        401 => FailureClass::AuthRequired,
        403 => FailureClass::PermissionDenied,
        413 => FailureClass::PayloadTooLarge,
        426 => FailureClass::UpgradeRequired,
        429 => FailureClass::RateLimited {
            retry_after_seconds,
        },
        500..=599 => FailureClass::Transient,
        _ => FailureClass::Permanent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifetrace_contracts::sync::v1::SyncChangeV1;
    use lifetrace_contracts::{AtomicGroupId, ChangeId};

    fn item(id: &str, group: Option<&str>) -> LeasedChange {
        let mut change: SyncChangeV1 = serde_json::from_value(serde_json::json!({
            "changeId": id,
            "entityType": "note.note",
            "entityId": id,
            "operation": "upsert",
            "baseServerVersion": "0",
            "entitySchemaVersion": 1,
            "clientModifiedAt": "2026-08-05T00:00:00Z",
            "payload": null,
            "dependencies": []
        }))
        .unwrap();
        change.change_id = ChangeId::new(id);
        change.atomic_group_id = group.map(AtomicGroupId::new);
        LeasedChange {
            change,
            local_payload_json: None,
        }
    }

    #[test]
    fn adaptive_split_never_breaks_an_atomic_group() {
        let batch = vec![item("a", Some("g")), item("b", Some("g")), item("c", None)];
        let (left, right) = split_without_breaking_atomic_group(&batch).unwrap();
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 1);
    }

    #[test]
    fn unsplittable_atomic_group_is_detected() {
        let batch = vec![item("a", Some("g")), item("b", Some("g"))];
        assert!(split_without_breaking_atomic_group(&batch).is_none());
    }
}
