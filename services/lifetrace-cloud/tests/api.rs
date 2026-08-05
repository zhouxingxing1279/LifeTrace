//! HTTP integration tests for the EPIC-03 sync server.
//!
//! These mirror the protocol semantics validated by the reference testkit
//! (`lifetrace-contracts`), but through the real Axum HTTP layer.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::domain::finance::Transaction;
use lifetrace_contracts::domain::{TransactionStatus, TransactionType};
use lifetrace_contracts::time::{LocalDate, UtcTimestamp};
use lifetrace_contracts::{CurrencyCode, EntityId, EntityMeta, UserId};
use serde_json::{json, Value};
use tower::ServiceExt;

const TOKEN_A: &str = "token-a";
const TOKEN_B: &str = "token-b";

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

fn transaction_payload(id: &str, amount_cents: i64) -> Value {
    serde_json::to_value(Transaction {
        meta: meta(id),
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
    })
    .unwrap()
}

fn client() -> Value {
    json!({
        "appId": "lifetrace-desktop",
        "clientVersion": "0.2.1",
        "platform": "windows",
        "protocolVersion": 1,
        "schemaVersion": 1,
        "deviceId": "device-1"
    })
}

fn change(
    change_id: &str,
    entity_id: &str,
    amount_cents: i64,
    base: u64,
    operation: &str,
    group: Option<&str>,
) -> Value {
    json!({
        "changeId": change_id,
        "entityType": "finance.transaction",
        "entityId": entity_id,
        "operation": operation,
        "baseServerVersion": base.to_string(),
        "entitySchemaVersion": 1,
        "clientModifiedAt": "2026-08-04T15:30:00Z",
        "payload": if operation == "upsert" {
            transaction_payload(entity_id, amount_cents)
        } else {
            Value::Null
        },
        "atomicGroupId": group,
        "dependencies": []
    })
}

fn push_request(changes: Vec<Value>) -> Value {
    json!({
        "requestId": "req-1",
        "client": client(),
        "changes": changes
    })
}

async fn send(
    app: Router,
    method: Method,
    uri: &str,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn test_app() -> Router {
    test_app_for(TOKEN_A, "user-a", "device-a")
}

fn test_app_for(token: &str, user: &str, device: &str) -> Router {
    let config = Config {
        dev_auth_token: token.to_owned(),
        dev_auth_user_id: user.to_owned(),
        dev_auth_device_id: device.to_owned(),
        ..Config::default()
    };
    app(AppState::new(config))
}

#[tokio::test]
async fn health_live_ok() {
    let (status, body) = send(
        test_app(),
        Method::GET,
        "/health/live",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn capabilities_ok() {
    let (status, body) = send(
        test_app(),
        Method::GET,
        "/api/v1/sync/capabilities",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["protocolVersion"], 1);
    assert_eq!(body["maximumPushBatchSize"], 500);
    assert_eq!(body["maximumAtomicGroupSize"], 50);
    assert!(body["supportedEntityTypes"].as_array().unwrap().len() >= 10);
}

#[tokio::test]
async fn push_create_pull_and_delete() {
    let app = test_app();

    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["status"], "accepted");
    assert_eq!(body["results"][0]["serverVersion"], "1");
    let accepted_cursor = body["results"][0]["cursor"].as_str().unwrap().to_owned();

    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-2",
            "client": client(),
            "afterCursor": null,
            "limit": 10,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["changes"].as_array().unwrap().len(), 1);
    assert_eq!(body["changes"][0]["operation"], "upsert");
    assert_eq!(body["changes"][0]["entityId"], "tx-1");

    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c2", "tx-1", 0, 1, "delete", None)]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["status"], "accepted");

    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-3",
            "client": client(),
            "afterCursor": accepted_cursor,
            "limit": 10,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["changes"][0]["operation"], "delete");
    assert!(body["changes"][0]["tombstone"]["serverVersion"].is_string());
}

#[tokio::test]
async fn base_version_conflict() {
    let app = test_app();
    send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;
    send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c2", "tx-1", 200, 1, "upsert", None)]),
    )
    .await;

    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c3", "tx-1", 300, 1, "upsert", None)]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result = &body["results"][0];
    assert_eq!(result["status"], "conflict");
    assert_eq!(result["reason"], "base_version_mismatch");
    assert_eq!(result["currentServerVersion"], "2");
    assert_eq!(result["serverEntity"]["amountCents"], 200);
}

#[tokio::test]
async fn duplicate_change_id_is_idempotent() {
    let app = test_app();
    let request = push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]);
    send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        request.clone(),
    )
    .await;
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        request,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["status"], "duplicate");
    assert_eq!(body["results"][0]["serverVersion"], "1");
}

#[tokio::test]
async fn change_id_reuse_with_different_payload_is_rejected() {
    let app = test_app();
    send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c1", "tx-2", 200, 0, "upsert", None)]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["status"], "rejected");
    assert_eq!(body["results"][0]["code"], "LIFETRACE_CHANGE_ID_REUSE");
}

#[tokio::test]
async fn unknown_entity_type_is_rejected() {
    let app = test_app();
    let mut unknown = change("c1", "x-1", 100, 0, "upsert", None);
    unknown["entityType"] = json!("foo.bar");
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![unknown]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["status"], "rejected");
    assert_eq!(body["results"][0]["code"], "LIFETRACE_UNKNOWN_ENTITY_TYPE");
}

#[tokio::test]
async fn unsupported_protocol_returns_426() {
    let app = test_app();
    let mut request = push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]);
    request["client"]["protocolVersion"] = json!(2);
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        request,
    )
    .await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    assert_eq!(body["code"], "LIFETRACE_PROTOCOL_UNSUPPORTED");
}

#[tokio::test]
async fn atomic_group_is_all_or_nothing() {
    let app = test_app();
    send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;

    // Group: one valid update + one stale-base update -> the whole group fails.
    let group_request = push_request(vec![
        change("g1", "tx-1", 150, 1, "upsert", Some("group-1")),
        change("g2", "tx-1", 999, 0, "upsert", Some("group-1")),
    ]);
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        group_request,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for result in body["results"].as_array().unwrap() {
        assert_eq!(result["status"], "rejected");
        assert_eq!(result["code"], "LIFETRACE_ATOMIC_GROUP_FAILED");
    }

    // Entity must be unchanged (version still 1).
    let (_, pull) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-9",
            "client": client(),
            "afterCursor": null,
            "limit": 10,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(pull["changes"][0]["serverVersion"], "1");
    assert_eq!(pull["changes"][0]["payload"]["amountCents"], 100);
}

#[tokio::test]
async fn pull_pagination_has_no_gaps_or_duplicates() {
    let app = test_app();
    for (index, change_id) in ["c1", "c2", "c3"].iter().enumerate() {
        send(
            app.clone(),
            Method::POST,
            "/api/v1/sync/push",
            TOKEN_A,
            push_request(vec![change(
                change_id,
                &format!("tx-{index}"),
                100 * (index as i64 + 1),
                0,
                "upsert",
                None,
            )]),
        )
        .await;
    }

    let (status, page1) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-p1",
            "client": client(),
            "afterCursor": null,
            "limit": 2,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page1["changes"].as_array().unwrap().len(), 2);
    assert_eq!(page1["hasMore"], true);
    let next_cursor = page1["nextCursor"].as_str().unwrap().to_owned();

    let (_, page2) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-p2",
            "client": client(),
            "afterCursor": next_cursor,
            "limit": 2,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(page2["changes"].as_array().unwrap().len(), 1);
    assert_eq!(page2["hasMore"], false);
    let cursors: Vec<u64> = page1["changes"]
        .as_array()
        .unwrap()
        .iter()
        .chain(page2["changes"].as_array().unwrap().iter())
        .map(|change| change["cursor"].as_str().unwrap().parse().unwrap())
        .collect();
    assert!(
        cursors.windows(2).all(|pair| pair[0] < pair[1]),
        "strictly ordered"
    );
}

#[tokio::test]
async fn snapshot_is_consistent_and_follow_up_pull_has_no_gaps() {
    let app = test_app();
    for (index, change_id) in ["c1", "c2"].iter().enumerate() {
        send(
            app.clone(),
            Method::POST,
            "/api/v1/sync/push",
            TOKEN_A,
            push_request(vec![change(
                change_id,
                &format!("tx-{index}"),
                100,
                0,
                "upsert",
                None,
            )]),
        )
        .await;
    }

    let (status, page1) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/snapshot",
        TOKEN_A,
        json!({
            "requestId": "req-s1",
            "client": client(),
            "snapshotId": null,
            "pageToken": null,
            "entityTypes": null,
            "pageSize": 1
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page1["items"].as_array().unwrap().len(), 1);
    assert_eq!(page1["completed"], false);
    let snapshot_id = page1["snapshotId"].as_str().unwrap().to_owned();
    let page_token = page1["nextPageToken"].as_str().unwrap().to_owned();
    let snapshot_cursor = page1["snapshotCursor"].as_str().unwrap().to_owned();

    // A concurrent change lands between snapshot pages; it must not be lost.
    send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c3", "tx-9", 500, 0, "upsert", None)]),
    )
    .await;

    let (status, page2) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/snapshot",
        TOKEN_A,
        json!({
            "requestId": "req-s2",
            "client": client(),
            "snapshotId": snapshot_id,
            "pageToken": page_token,
            "entityTypes": null,
            "pageSize": 1
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);
    assert_eq!(page2["completed"], true);
    assert_eq!(page2["snapshotCursor"].as_str().unwrap(), snapshot_cursor);

    // Pull from the snapshot cursor must contain only the concurrent change.
    let (_, pull) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-s3",
            "client": client(),
            "afterCursor": snapshot_cursor,
            "limit": 10,
            "entityTypes": null
        }),
    )
    .await;
    let changes = pull["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["entityId"], "tx-9");
}

#[tokio::test]
async fn expired_cursor_requires_snapshot() {
    let config = Config {
        dev_auth_token: TOKEN_A.to_owned(),
        retention_entries: 1,
        ..Config::default()
    };
    let app = app(AppState::new(config));
    let (_, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;
    let old_cursor = first["results"][0]["cursor"].as_str().unwrap().to_owned();
    for (index, change_id) in ["c2", "c3"].iter().enumerate() {
        send(
            app.clone(),
            Method::POST,
            "/api/v1/sync/push",
            TOKEN_A,
            push_request(vec![change(
                change_id,
                &format!("tx-{}", index + 2),
                100,
                0,
                "upsert",
                None,
            )]),
        )
        .await;
    }
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-e1",
            "client": client(),
            "afterCursor": old_cursor,
            "limit": 10,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(body["code"], "LIFETRACE_CURSOR_EXPIRED");
}

#[tokio::test]
async fn users_are_isolated() {
    let app_a = test_app_for(TOKEN_A, "user-a", "device-a");
    let app_b = test_app_for(TOKEN_B, "user-b", "device-b");
    send(
        app_a.clone(),
        Method::POST,
        "/api/v1/sync/push",
        TOKEN_A,
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;

    let (_, pull) = send(
        app_b.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_B,
        json!({
            "requestId": "req-i1",
            "client": client(),
            "afterCursor": null,
            "limit": 10,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(pull["changes"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn sync_endpoints_require_auth() {
    let app = test_app();
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        "",
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "LIFETRACE_AUTH_REQUIRED");

    let (status, _) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/push",
        "wrong-token",
        push_request(vec![change("c1", "tx-1", 100, 0, "upsert", None)]),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn meta_version_endpoint() {
    let (status, body) = send(
        test_app(),
        Method::GET,
        "/api/v1/meta/version",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["protocolVersion"], 1);
    assert_eq!(body["schemaVersion"], 1);
}

#[tokio::test]
async fn finance_crud_create_list_get_delete() {
    let app = test_app();

    // Create via the business CRUD endpoint.
    let (status, body) = send(
        app.clone(),
        Method::POST,
        "/api/v1/finance/transactions",
        TOKEN_A,
        transaction_payload("tx-crud-1", 888),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["serverVersion"], "1");

    // List.
    let (status, body) = send(
        app.clone(),
        Method::GET,
        "/api/v1/finance/transactions",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["id"], "tx-crud-1");
    assert_eq!(body["items"][0]["data"]["amountCents"], 888);

    // The same entity is visible through the sync protocol.
    let (_, pull) = send(
        app.clone(),
        Method::POST,
        "/api/v1/sync/pull",
        TOKEN_A,
        json!({
            "requestId": "req-crud-1",
            "client": client(),
            "afterCursor": null,
            "limit": 10,
            "entityTypes": null
        }),
    )
    .await;
    assert_eq!(pull["changes"][0]["entityId"], "tx-crud-1");

    // Get one.
    let (status, body) = send(
        app.clone(),
        Method::GET,
        "/api/v1/finance/transactions/tx-crud-1",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["serverVersion"], "1");

    // Delete.
    let (status, body) = send(
        app.clone(),
        Method::DELETE,
        "/api/v1/finance/transactions/tx-crud-1",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["deleted"], true);

    // Get after delete -> 404; list is empty.
    let (status, _) = send(
        app.clone(),
        Method::GET,
        "/api/v1/finance/transactions/tx-crud-1",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = send(
        app.clone(),
        Method::GET,
        "/api/v1/finance/transactions",
        TOKEN_A,
        Value::Null,
    )
    .await;
    assert_eq!(list["items"].as_array().unwrap().len(), 0);
}
