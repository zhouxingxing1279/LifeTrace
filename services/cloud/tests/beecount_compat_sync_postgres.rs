//! End-to-end BeeCount protocol compatibility checks backed by PostgreSQL.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use chrono::{Duration, SecondsFormat, Utc};
use lifetrace_cloud::{app, AppState, Config};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn config(database_url: String) -> Config {
    Config {
        database_url: Some(database_url),
        migration_on_startup: true,
        dev_auth_enabled: false,
        auth_registration_mode: "open".to_owned(),
        auth_password_pepper: Some("beecount-test-password-pepper-0123456789".to_owned()),
        auth_token_hash_pepper: Some("beecount-test-token-pepper-012345678901".to_owned()),
        cursor_signing_key: Some("beecount-test-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("beecount-test-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        ..Config::default()
    }
}

async fn send(
    router: axum::Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = bearer {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let body = if let Some(value) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(value.to_string())
    } else {
        Body::empty()
    };
    let response = router.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

#[tokio::test]
async fn stock_client_sync_routes_share_the_lifetrace_entity_log() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    state.initialize().await.unwrap();
    let router = app(state.clone());
    let external_device_id = format!("beecount-device-{}", Uuid::new_v4());
    let email = format!("beecount-{}@example.test", Uuid::new_v4());

    let (status, registration) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/auth/register",
        None,
        Some(json!({
            "email": email,
            "password": "正确 horse battery staple 密码",
            "device_id": external_device_id,
            "device_name": "BeeCount Android",
            "platform": "android",
            "app_version": "1.0.0"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{registration}");
    assert_eq!(registration["device_id"], external_device_id);
    assert!(registration.get("accessToken").is_none());
    let token = registration["access_token"].as_str().unwrap().to_owned();

    let updated_at =
        (Utc::now() - Duration::seconds(30)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let changes = json!({
        "device_id": external_device_id,
        "changes": [
            {
                "ledger_id": "ledger-1",
                "entity_type": "ledger",
                "entity_sync_id": "ledger-1",
                "action": "upsert",
                "payload": {
                    "syncId": "ledger-1",
                    "ledgerName": "日常账本",
                    "currency": "CNY",
                    "monthStartDay": 1
                },
                "updated_at": updated_at,
                "scope": "ledger"
            },
            {
                "ledger_id": null,
                "entity_type": "account",
                "entity_sync_id": "account-1",
                "action": "upsert",
                "payload": {
                    "syncId": "account-1",
                    "name": "现金",
                    "type": "cash",
                    "currency": "CNY",
                    "initialBalance": 100
                },
                "updated_at": updated_at,
                "scope": "user"
            },
            {
                "ledger_id": "ledger-1",
                "entity_type": "transaction",
                "entity_sync_id": "transaction-1",
                "action": "upsert",
                "payload": {
                    "syncId": "transaction-1",
                    "type": "expense",
                    "amount": 12.345,
                    "currencyCode": "CNY",
                    "accountId": "account-1",
                    "happenedAt": "2026-08-13T10:00:00Z",
                    "note": "午餐"
                },
                "updated_at": updated_at,
                "scope": "ledger"
            }
        ]
    });
    let (status, pushed) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(&token),
        Some(changes.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pushed}");
    assert_eq!(pushed["accepted"], 3);
    assert_eq!(pushed["rejected"], 0);
    assert_eq!(pushed["conflict_count"], 0);
    assert!(pushed.get("serverCursor").is_none());
    let first_cursor = pushed["server_cursor"].as_i64().unwrap();

    let canonical_amount: i64 = sqlx::query_scalar(
        "SELECT (payload->>'amountCents')::BIGINT FROM sync_entities \
         WHERE entity_type='finance.transaction' AND entity_id='beecount:transaction-1' \
           AND user_id=$1::uuid",
    )
    .bind(registration["user"]["id"].as_str().unwrap())
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(canonical_amount, 1235);

    let (status, pulled) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/sync/pull?since=0&limit=20",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pulled}");
    assert_eq!(pulled["changes"].as_array().unwrap().len(), 3);
    let transaction = pulled["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|change| change["entity_type"] == "transaction")
        .unwrap();
    assert_eq!(transaction["entity_sync_id"], "transaction-1");
    assert_eq!(transaction["payload"]["amount"], 12.35);
    assert_eq!(transaction["payload"]["accountId"], "account-1");

    let (status, ledgers) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/sync/ledgers",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ledgers}");
    assert_eq!(ledgers.as_array().unwrap().len(), 1);
    assert_eq!(ledgers[0]["ledger_id"], "ledger-1");

    let (status, full) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/sync/full?ledger_id=ledger-1",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{full}");
    let content: Value =
        serde_json::from_str(full["snapshot"]["payload"]["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["ledgerName"], "日常账本");
    assert_eq!(content["items"].as_array().unwrap().len(), 1);
    assert_eq!(content["accounts"].as_array().unwrap().len(), 1);

    let (status, replayed) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(&token),
        Some(changes),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["accepted"], 3);
    assert_eq!(replayed["server_cursor"], first_cursor);

    let older_at = (Utc::now() - Duration::hours(1)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let (status, stale) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(&token),
        Some(json!({
            "device_id": external_device_id,
            "changes": [{
                "ledger_id": "ledger-1",
                "entity_type": "transaction",
                "entity_sync_id": "transaction-1",
                "action": "upsert",
                "payload": {
                    "type": "expense",
                    "amount": 99,
                    "currencyCode": "CNY",
                    "happenedAt": "2026-08-13T10:00:00Z"
                },
                "updated_at": older_at,
                "scope": "ledger"
            }]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{stale}");
    assert_eq!(stale["accepted"], 0);
    assert_eq!(stale["rejected"], 1);
    assert_eq!(stale["conflict_count"], 1);

    let native_updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let user_id = registration["user"]["id"].as_str().unwrap();
    let (status, native_push) = send(
        router.clone(),
        Method::POST,
        "/api/v1/sync/push",
        Some(&token),
        Some(json!({
            "requestId": format!("native-request-{}", Uuid::new_v4()),
            "client": {
                "appId": "beecount-mobile",
                "clientVersion": "1.0.0",
                "platform": "android",
                "protocolVersion": 1,
                "schemaVersion": 1,
                "deviceId": external_device_id
            },
            "changes": [{
                "changeId": format!("native-change-{}", Uuid::new_v4()),
                "entityType": "finance.ledger",
                "entityId": "native-ledger-1",
                "operation": "upsert",
                "baseServerVersion": "0",
                "entitySchemaVersion": 1,
                "clientModifiedAt": native_updated_at,
                "payload": {
                    "meta": {
                        "id": "native-ledger-1",
                        "userId": user_id,
                        "createdAt": native_updated_at,
                        "updatedAt": native_updated_at,
                        "deletedAt": null,
                        "localVersion": 1,
                        "serverVersion": null,
                        "modifiedByDevice": external_device_id
                    },
                    "name": "LifeTrace 原生账本",
                    "currency": "CNY",
                    "ledgerType": "personal",
                    "monthStartDay": 1,
                    "sortOrder": 0,
                    "isArchived": false
                },
                "atomicGroupId": null,
                "dependencies": []
            }]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{native_push}");
    assert_eq!(native_push["results"][0]["status"], "accepted");

    let (status, native_pull) = send(
        router.clone(),
        Method::GET,
        &format!("/api/v1/integrations/beecount/compat/sync/pull?since={first_cursor}&limit=20"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{native_pull}");
    let native_change = native_pull["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|change| change["entity_sync_id"] == "lifetrace:native-ledger-1")
        .unwrap();
    assert_eq!(native_change["entity_type"], "ledger");

    let (status, native_full) = send(
        router,
        Method::GET,
        "/api/v1/integrations/beecount/compat/sync/full?ledger_id=lifetrace:native-ledger-1",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{native_full}");
    let native_content: Value = serde_json::from_str(
        native_full["snapshot"]["payload"]["content"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(native_content["ledgerName"], "LifeTrace 原生账本");
}
