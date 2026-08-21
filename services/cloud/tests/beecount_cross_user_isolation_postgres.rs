//! Cross-account regression coverage for the LifeTrace BeeCount finance facade.
//!
//! These tests intentionally create data with colliding ledger references and a
//! shared-ledger membership. An authenticated account must never receive another
//! user's unrelated finance rows, and ledger statistics must always be scoped to
//! the authorized storage owner.

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
        auth_password_pepper: Some("beecount-isolation-password-pepper-0123456789".to_owned()),
        auth_token_hash_pepper: Some(
            "beecount-isolation-token-pepper-012345678901".to_owned(),
        ),
        cursor_signing_key: Some("beecount-isolation-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("beecount-isolation-page-token-key".to_owned()),
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

async fn register(router: axum::Router, label: &str) -> (String, String, String) {
    let device_id = format!("{label}-device-{}", Uuid::new_v4());
    let email = format!("{label}-{}@example.test", Uuid::new_v4());
    let (status, body) = send(
        router,
        Method::POST,
        "/api/v1/integrations/beecount/compat/auth/register",
        None,
        Some(json!({
            "email": email,
            "password": "正确 horse battery staple isolation 密码",
            "device_id": device_id,
            "device_name": format!("{label} Android"),
            "platform": "android",
            "app_version": "1.6.3"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    (
        body["access_token"].as_str().unwrap().to_owned(),
        body["user"]["id"].as_str().unwrap().to_owned(),
        device_id,
    )
}

async fn push(
    router: axum::Router,
    token: &str,
    device_id: &str,
    changes: Value,
) -> Value {
    let (status, body) = send(
        router,
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(token),
        Some(json!({
            "device_id": device_id,
            "changes": changes
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

#[tokio::test]
async fn ledger_stats_ignore_same_ledger_reference_from_another_user() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    state.initialize().await.unwrap();
    let router = app(state.clone());

    let (owner_token, _owner_id, owner_device) = register(router.clone(), "stats-owner").await;
    let (other_token, other_id, other_device) = register(router.clone(), "stats-other").await;
    let owner_ledger = format!("owner-ledger-{}", Uuid::new_v4());
    let other_ledger = format!("other-ledger-{}", Uuid::new_v4());
    let owner_tx = format!("owner-tx-{}", Uuid::new_v4());
    let other_tx = format!("other-tx-{}", Uuid::new_v4());
    let updated_at =
        (Utc::now() - Duration::seconds(10)).to_rfc3339_opts(SecondsFormat::Millis, true);

    let pushed = push(
        router.clone(),
        &owner_token,
        &owner_device,
        json!([
            {
                "ledger_id": owner_ledger,
                "entity_type": "ledger",
                "entity_sync_id": owner_ledger,
                "action": "upsert",
                "payload": {"syncId":owner_ledger,"ledgerName":"Owner ledger","currency":"CNY","monthStartDay":1},
                "updated_at": updated_at,
                "scope": "ledger"
            },
            {
                "ledger_id": owner_ledger,
                "entity_type": "transaction",
                "entity_sync_id": owner_tx,
                "action": "upsert",
                "payload": {"syncId":owner_tx,"type":"expense","amount":12.34,"currencyCode":"CNY","happenedAt":"2026-08-21T01:00:00Z"},
                "updated_at": updated_at,
                "scope": "ledger"
            }
        ]),
    )
    .await;
    assert_eq!(pushed["accepted"], 2);

    let pushed = push(
        router.clone(),
        &other_token,
        &other_device,
        json!([
            {
                "ledger_id": other_ledger,
                "entity_type": "ledger",
                "entity_sync_id": other_ledger,
                "action": "upsert",
                "payload": {"syncId":other_ledger,"ledgerName":"Other ledger","currency":"CNY","monthStartDay":1},
                "updated_at": updated_at,
                "scope": "ledger"
            },
            {
                "ledger_id": other_ledger,
                "entity_type": "transaction",
                "entity_sync_id": other_tx,
                "action": "upsert",
                "payload": {"syncId":other_tx,"type":"expense","amount":90.00,"currencyCode":"CNY","happenedAt":"2026-08-21T02:00:00Z"},
                "updated_at": updated_at,
                "scope": "ledger"
            }
        ]),
    )
    .await;
    assert_eq!(pushed["accepted"], 2);

    // Simulate a stale/malformed row under another account that happens to carry
    // the same external ledger id. The finance read surface must still scope the
    // aggregate by the authorized storage user, never by ledger id alone.
    let updated = sqlx::query(
        "UPDATE sync_entities \
         SET payload=jsonb_set(payload,'{beecountLedgerId}',to_jsonb($3::text),true) \
         WHERE user_id=$1::uuid AND entity_type='finance.transaction' AND entity_id=$2",
    )
    .bind(&other_id)
    .bind(format!("beecount:{other_tx}"))
    .bind(&owner_ledger)
    .execute(&state.pool)
    .await
    .unwrap();
    assert_eq!(updated.rows_affected(), 1);

    let (status, ledgers) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/ledgers",
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ledgers}");
    let owner_row = ledgers["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["sourceId"] == owner_ledger)
        .expect("owner ledger missing");
    assert_eq!(owner_row["transactionCount"], 1);
    assert_eq!(owner_row["expenseTotalCents"], 1234);
    assert_eq!(owner_row["balanceCents"], -1234);

    let (status, other_ledgers) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/ledgers",
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{other_ledgers}");
    assert!(!other_ledgers["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["sourceId"] == owner_ledger));

    let (status, denied) = send(
        router,
        Method::GET,
        &format!(
            "/api/v1/integrations/beecount/ledgers/{owner_ledger}/snapshot?limit=500&offset=0"
        ),
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{denied}");
}

#[tokio::test]
async fn shared_member_only_sees_owner_globals_referenced_by_shared_ledger() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    state.initialize().await.unwrap();
    let router = app(state.clone());

    let (owner_token, _owner_id, owner_device) = register(router.clone(), "share-owner").await;
    let (editor_token, editor_id, _editor_device) = register(router.clone(), "share-editor").await;
    let ledger_id = format!("shared-ledger-{}", Uuid::new_v4());
    let updated_at =
        (Utc::now() - Duration::seconds(10)).to_rfc3339_opts(SecondsFormat::Millis, true);

    let pushed = push(
        router.clone(),
        &owner_token,
        &owner_device,
        json!([
            {"ledger_id":ledger_id,"entity_type":"ledger","entity_sync_id":ledger_id,"action":"upsert","payload":{"syncId":ledger_id,"ledgerName":"Shared","currency":"CNY","monthStartDay":1},"updated_at":updated_at,"scope":"ledger"},
            {"ledger_id":null,"entity_type":"account","entity_sync_id":"account-shared","action":"upsert","payload":{"syncId":"account-shared","name":"Shared cash","type":"cash","currency":"CNY","initialBalance":0},"updated_at":updated_at,"scope":"user"},
            {"ledger_id":null,"entity_type":"account","entity_sync_id":"account-private","action":"upsert","payload":{"syncId":"account-private","name":"Private savings","type":"bank","currency":"CNY","initialBalance":9999},"updated_at":updated_at,"scope":"user"},
            {"ledger_id":null,"entity_type":"category","entity_sync_id":"category-shared","action":"upsert","payload":{"syncId":"category-shared","name":"Shared food","kind":"expense"},"updated_at":updated_at,"scope":"user"},
            {"ledger_id":null,"entity_type":"category","entity_sync_id":"category-private","action":"upsert","payload":{"syncId":"category-private","name":"Private category","kind":"expense"},"updated_at":updated_at,"scope":"user"},
            {"ledger_id":null,"entity_type":"tag","entity_sync_id":"tag-shared","action":"upsert","payload":{"syncId":"tag-shared","name":"Shared tag"},"updated_at":updated_at,"scope":"user"},
            {"ledger_id":null,"entity_type":"tag","entity_sync_id":"tag-private","action":"upsert","payload":{"syncId":"tag-private","name":"Private tag"},"updated_at":updated_at,"scope":"user"},
            {"ledger_id":ledger_id,"entity_type":"transaction","entity_sync_id":"shared-tx","action":"upsert","payload":{"syncId":"shared-tx","type":"expense","amount":12.34,"currencyCode":"CNY","accountId":"account-shared","categoryId":"category-shared","tagIds":["tag-shared"],"happenedAt":"2026-08-21T01:00:00Z"},"updated_at":updated_at,"scope":"ledger"}
        ]),
    )
    .await;
    assert_eq!(pushed["accepted"], 8);

    sqlx::query(
        "INSERT INTO beecount_ledger_members (ledger_id,user_id,role) \
         VALUES ($1,$2::uuid,'editor') \
         ON CONFLICT (ledger_id,user_id) DO UPDATE SET role='editor'",
    )
    .bind(&ledger_id)
    .bind(&editor_id)
    .execute(&state.pool)
    .await
    .unwrap();

    let (status, editor_snapshot) = send(
        router.clone(),
        Method::GET,
        &format!(
            "/api/v1/integrations/beecount/ledgers/{ledger_id}/snapshot?limit=500&offset=0"
        ),
        Some(&editor_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{editor_snapshot}");
    assert_eq!(editor_snapshot["ledger"]["role"], "editor");
    assert_eq!(editor_snapshot["accounts"].as_array().unwrap().len(), 1);
    assert_eq!(editor_snapshot["accounts"][0]["sourceId"], "account-shared");
    assert_eq!(editor_snapshot["categories"].as_array().unwrap().len(), 1);
    assert_eq!(
        editor_snapshot["categories"][0]["sourceId"],
        "category-shared"
    );
    assert_eq!(editor_snapshot["tags"].as_array().unwrap().len(), 1);
    assert_eq!(editor_snapshot["tags"][0]["sourceId"], "tag-shared");
    let serialized = editor_snapshot.to_string();
    assert!(!serialized.contains("Private savings"));
    assert!(!serialized.contains("Private category"));
    assert!(!serialized.contains("Private tag"));

    // The owner still sees all of their own user-global finance resources.
    let (status, owner_snapshot) = send(
        router,
        Method::GET,
        &format!(
            "/api/v1/integrations/beecount/ledgers/{ledger_id}/snapshot?limit=500&offset=0"
        ),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{owner_snapshot}");
    assert_eq!(owner_snapshot["accounts"].as_array().unwrap().len(), 2);
    assert_eq!(owner_snapshot["categories"].as_array().unwrap().len(), 2);
    assert_eq!(owner_snapshot["tags"].as_array().unwrap().len(), 2);
}
