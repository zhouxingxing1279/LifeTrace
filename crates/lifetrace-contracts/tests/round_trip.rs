//! Wire round-trip and forward-compatibility tests for sync v1 DTOs.

use lifetrace_contracts::domain::*;
use lifetrace_contracts::sync::v1::*;
use lifetrace_contracts::*;

fn stamp() -> UtcTimestamp {
    "2026-08-04T15:30:00Z".parse().unwrap()
}

fn meta(id: &str) -> EntityMeta {
    EntityMeta {
        id: EntityId::new(id),
        user_id: UserId::new("local-user"),
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

fn transaction_payload() -> Transaction {
    Transaction {
        meta: meta("tx-1"),
        transaction_type: TransactionType::new(TransactionType::EXPENSE),
        amount_cents: 12525,
        currency: CurrencyCode::cny(),
        account_id: Some(EntityId::new("wechat-wallet")),
        to_account_id: None,
        category_id: Some(EntityId::new("cat-food")),
        counterparty: Some("coffee shop".to_owned()),
        merchant: None,
        item: Some("latte".to_owned()),
        note: None,
        occurred_at: stamp(),
        local_date: LocalDate::new("2026-08-04").unwrap(),
        status: TransactionStatus::new(TransactionStatus::CONFIRMED),
        source_type: "manual".to_owned(),
        external_transaction_id: None,
    }
}

fn change() -> SyncChangeV1 {
    let payload = serde_json::to_value(transaction_payload()).unwrap();
    SyncChangeV1 {
        change_id: ChangeId::new("change-1"),
        entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
        entity_id: EntityId::new("tx-1"),
        operation: ChangeOperation::new(ChangeOperation::UPSERT),
        base_server_version: ServerVersion::zero(),
        entity_schema_version: 1,
        client_modified_at: stamp(),
        payload: Some(payload.into()),
        atomic_group_id: None,
        dependencies: vec![],
    }
}

#[test]
fn sync_change_v1_round_trips_with_camel_case() {
    let change = change();
    let json = serde_json::to_value(&change).unwrap();
    assert_eq!(json["changeId"], "change-1");
    assert_eq!(json["entityType"], "finance.transaction");
    assert_eq!(json["operation"], "upsert");
    assert_eq!(json["baseServerVersion"], "0");
    assert_eq!(json["entitySchemaVersion"], 1);
    assert!(json.get("atomicGroupId").unwrap().is_null());
    assert_eq!(json["dependencies"], serde_json::json!([]));
    let payload = &json["payload"];
    assert_eq!(payload["amountCents"], 12525);
    assert_eq!(payload["transactionType"], "expense");

    let back: SyncChangeV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, change);
}

#[test]
fn push_request_round_trips() {
    let request = PushRequestV1 {
        request_id: RequestId::new("req-1"),
        client: client(),
        changes: vec![change()],
    };
    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["requestId"], "req-1");
    assert_eq!(json["client"]["appId"], "lifetrace-desktop");
    assert_eq!(json["changes"][0]["entityId"], "tx-1");
    let back: PushRequestV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, request);
}

#[test]
fn push_response_accepted_round_trips() {
    let response = PushResponseV1 {
        request_id: RequestId::new("req-1"),
        server_time: stamp(),
        results: vec![PushChangeResultV1::Accepted {
            change_id: ChangeId::new("change-1"),
            entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
            entity_id: EntityId::new("tx-1"),
            server_version: ServerVersion::from_u64(42),
            cursor: Cursor::new("42"),
            server_modified_at: stamp(),
        }],
        latest_cursor: Cursor::new("42"),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["results"][0]["status"], "accepted");
    assert_eq!(json["results"][0]["serverVersion"], "42");
    let back: PushResponseV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, response);
}

#[test]
fn push_response_conflict_round_trips() {
    let response = PushResponseV1 {
        request_id: RequestId::new("req-1"),
        server_time: stamp(),
        results: vec![PushChangeResultV1::Conflict {
            conflict_id: ConflictId::new("conflict-1"),
            change_id: ChangeId::new("change-1"),
            entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
            entity_id: EntityId::new("tx-1"),
            client_base_server_version: ServerVersion::from_u64(7),
            current_server_version: ServerVersion::from_u64(8),
            server_entity: Some(serde_json::json!({"server": true}).into()),
            server_deleted: false,
            reason: ConflictReason::new(ConflictReason::BASE_VERSION_MISMATCH),
        }],
        latest_cursor: Cursor::new("42"),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["results"][0]["status"], "conflict");
    assert_eq!(json["results"][0]["clientBaseServerVersion"], "7");
    assert_eq!(json["results"][0]["currentServerVersion"], "8");
    assert_eq!(json["results"][0]["reason"], "base_version_mismatch");
    let back: PushResponseV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, response);
}

#[test]
fn pull_response_with_tombstone_round_trips() {
    let response = PullResponseV1 {
        request_id: RequestId::new("req-2"),
        server_time: stamp(),
        changes: vec![ServerChangeV1 {
            cursor: Cursor::new("11"),
            entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
            entity_id: EntityId::new("tx-1"),
            operation: ChangeOperation::new(ChangeOperation::DELETE),
            server_version: ServerVersion::from_u64(9),
            server_modified_at: stamp(),
            payload: None,
            tombstone: Some(TombstoneV1 {
                entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
                entity_id: EntityId::new("tx-1"),
                deleted_at: stamp(),
                server_version: ServerVersion::from_u64(9),
                deleted_by_device: Some(DeviceId::new("device-2")),
            }),
            origin_device_id: Some(DeviceId::new("device-2")),
        }],
        next_cursor: Cursor::new("12"),
        has_more: false,
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["changes"][0]["operation"], "delete");
    assert_eq!(json["changes"][0]["tombstone"]["serverVersion"], "9");
    assert_eq!(json["changes"][0]["tombstone"]["deletedByDevice"], "device-2");
    let back: PullResponseV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, response);
}

#[test]
fn snapshot_response_round_trips() {
    let response = SnapshotResponseV1 {
        request_id: RequestId::new("req-3"),
        snapshot_id: SnapshotId::new("snapshot-1"),
        snapshot_cursor: Cursor::new("100"),
        items: vec![EntitySnapshotV1 {
            entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
            entity_id: EntityId::new("tx-1"),
            server_version: ServerVersion::from_u64(42),
            payload: serde_json::to_value(transaction_payload()).unwrap().into(),
        }],
        next_page_token: Some("page-2".to_owned()),
        completed: false,
        server_time: stamp(),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["snapshotCursor"], "100");
    assert_eq!(json["nextPageToken"], "page-2");
    assert_eq!(json["completed"], false);
    let back: SnapshotResponseV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, response);
}

#[test]
fn capabilities_round_trip_with_defaults() {
    let capabilities = CapabilitiesResponseV1::default_v1(stamp());
    let json = serde_json::to_value(&capabilities).unwrap();
    assert_eq!(json["protocolVersion"], 1);
    assert_eq!(json["maximumPushBatchSize"], 500);
    assert_eq!(json["maximumAtomicGroupSize"], 50);
    assert_eq!(json["tombstoneRetentionDays"], 90);
    let supported = json["supportedEntityTypes"].as_array().unwrap();
    assert!(supported.iter().any(|value| value == "finance.transaction"));
    assert!(supported.iter().any(|value| value == "english.article"));
    let back: CapabilitiesResponseV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, capabilities);
}

#[test]
fn unknown_fields_are_ignored() {
    let mut json = serde_json::to_value(change()).unwrap();
    json.as_object_mut()
        .unwrap()
        .insert("futureField".to_owned(), serde_json::json!({"x": 1}));
    let back: SyncChangeV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back, change());
}

#[test]
fn unknown_enum_values_do_not_fail_batch_parsing() {
    let json = serde_json::json!({
        "changeId": "change-1",
        "entityType": "finance.transaction",
        "entityId": "tx-1",
        "operation": "future_operation",
        "baseServerVersion": "0",
        "entitySchemaVersion": 1,
        "clientModifiedAt": "2026-08-04T15:30:00Z",
        "payload": null,
        "atomicGroupId": null,
        "dependencies": []
    });
    let change: SyncChangeV1 = serde_json::from_value(json).unwrap();
    assert_eq!(change.operation.as_str(), "future_operation");
}

#[test]
fn entity_payload_dispatch_round_trips() {
    let entity_type = EntityType::new(EntityType::FINANCE_TRANSACTION);
    let json: JsonValue = serde_json::to_value(transaction_payload()).unwrap().into();
    let payload = EntityPayload::try_from((&entity_type, json)).unwrap();
    assert_eq!(payload.entity_type(), entity_type);
    assert_eq!(payload.entity_id().as_str(), "tx-1");
    let round_tripped = serde_json::from_value::<Transaction>(payload.to_json().0).unwrap();
    assert_eq!(round_tripped, transaction_payload());
}

#[test]
fn error_body_round_trips() {
    let error = ApiErrorV1::new(ErrorCode::SnapshotRequired, "cursor expired");
    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(json["code"], "LIFETRACE_SNAPSHOT_REQUIRED");
    let back: ApiErrorV1 = serde_json::from_value(json).unwrap();
    assert_eq!(back.code, ErrorCode::SnapshotRequired);
}
