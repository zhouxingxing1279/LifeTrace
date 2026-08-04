//! Golden fixture tests.
//!
//! Fixtures under `tests/fixtures/sync-v1/` are frozen v1 compatibility
//! samples. They MUST keep parsing with the current types; a future schema
//! change that breaks them requires a protocol/schema version bump.

use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::*;

fn fixture(name: &str) -> serde_json::Value {
    let path = format!(
        "{}/tests/fixtures/sync-v1/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn golden_push_request_parses() {
    let request: PushRequestV1 = serde_json::from_value(fixture("push-request.json")).unwrap();
    assert_eq!(request.client.app_id.as_str(), AppId::DESKTOP);
    assert_eq!(request.changes.len(), 1);
    let change = &request.changes[0];
    assert_eq!(change.entity_type.as_str(), EntityType::FINANCE_TRANSACTION);
    assert_eq!(change.operation.as_str(), ChangeOperation::UPSERT);
    assert_eq!(change.base_server_version.as_str(), "0");
    assert_eq!(change.entity_schema_version, 1);
    assert!(change.payload.is_some());
    let payload = change.payload.as_ref().unwrap();
    assert_eq!(payload.0["amountCents"], 12525);
    assert_eq!(payload.0["currency"], "CNY");
}

#[test]
fn golden_push_success_parses() {
    let response: PushResponseV1 = serde_json::from_value(fixture("push-success.json")).unwrap();
    assert_eq!(response.latest_cursor.as_str(), "42");
    match &response.results[0] {
        PushChangeResultV1::Accepted { server_version, .. } => {
            assert_eq!(server_version.as_str(), "42");
        }
        other => panic!("expected accepted, got {other:?}"),
    }
}

#[test]
fn golden_push_conflict_parses() {
    let response: PushResponseV1 = serde_json::from_value(fixture("push-conflict.json")).unwrap();
    match &response.results[0] {
        PushChangeResultV1::Conflict {
            client_base_server_version,
            current_server_version,
            server_deleted,
            reason,
            ..
        } => {
            assert_eq!(client_base_server_version.as_str(), "41");
            assert_eq!(current_server_version.as_str(), "42");
            assert!(!server_deleted);
            assert_eq!(reason.as_str(), ConflictReason::BASE_VERSION_MISMATCH);
        }
        other => panic!("expected conflict, got {other:?}"),
    }
}

#[test]
fn golden_pull_response_parses() {
    let response: PullResponseV1 = serde_json::from_value(fixture("pull-response.json")).unwrap();
    assert_eq!(response.changes.len(), 2);
    assert_eq!(response.next_cursor.as_str(), "42");
    assert!(!response.has_more);
    let delete = &response.changes[1];
    assert_eq!(delete.operation.as_str(), ChangeOperation::DELETE);
    assert!(delete.tombstone.is_some());
    assert_eq!(
        delete.tombstone.as_ref().unwrap().server_version.as_str(),
        "12"
    );
}

#[test]
fn golden_tombstone_parses() {
    let tombstone: TombstoneV1 = serde_json::from_value(fixture("delete-tombstone.json")).unwrap();
    assert_eq!(tombstone.entity_id.as_str(), "tx-2026-08-04-0002");
    assert_eq!(tombstone.server_version.as_str(), "12");
}

#[test]
fn golden_snapshot_page_parses() {
    let response: SnapshotResponseV1 =
        serde_json::from_value(fixture("snapshot-page.json")).unwrap();
    assert_eq!(response.snapshot_cursor.as_str(), "100");
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.next_page_token.as_deref(), Some("page-1"));
    assert!(!response.completed);
}

#[test]
fn golden_error_response_parses() {
    let error: ApiErrorV1 = serde_json::from_value(fixture("error-response.json")).unwrap();
    assert_eq!(error.code, ErrorCode::SnapshotRequired);
    assert!(!error.retryable);
    assert!(error.field_errors.is_empty());
}

#[test]
fn golden_capabilities_parses() {
    let capabilities: CapabilitiesResponseV1 =
        serde_json::from_value(fixture("capabilities.json")).unwrap();
    assert_eq!(capabilities.protocol_version, 1);
    assert_eq!(capabilities.maximum_push_batch_size, 500);
    assert_eq!(capabilities.maximum_atomic_group_size, 50);
    assert_eq!(capabilities.tombstone_retention_days, 90);
    assert_eq!(capabilities.supported_entity_types.len(), 30);
}

#[test]
fn golden_capabilities_matches_registry() {
    let capabilities: CapabilitiesResponseV1 =
        serde_json::from_value(fixture("capabilities.json")).unwrap();
    let registered: Vec<String> = lifetrace_contracts::registry::REGISTRY
        .iter()
        .map(|descriptor| descriptor.entity_type.to_owned())
        .collect();
    assert_eq!(capabilities.supported_entity_types, registered);
}
