//! Proves that stock BeeCount writes and LifeTrace Web finance reads use one
//! PostgreSQL authoritative store. No external BeeCount adapter is configured.

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
        auth_password_pepper: Some("beecount-web-test-password-pepper-0123456789".to_owned()),
        auth_token_hash_pepper: Some("beecount-web-test-token-pepper-012345678901".to_owned()),
        cursor_signing_key: Some("beecount-web-test-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("beecount-web-test-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        // Deliberately leave BEECOUNT_ADAPTER_ENABLED at its default false.
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
async fn web_finance_reads_stock_beecount_writes_without_external_adapter() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    assert!(state.beecount_adapter.is_none());
    state.initialize().await.unwrap();
    let router = app(state.clone());

    let external_device_id = format!("beecount-web-device-{}", Uuid::new_v4());
    let email = format!("beecount-web-{}@example.test", Uuid::new_v4());
    let (status, registration) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/auth/register",
        None,
        Some(json!({
            "email": email,
            "password": "正确 horse battery staple web 密码",
            "device_id": external_device_id,
            "device_name": "BeeCount Android",
            "platform": "android",
            "app_version": "1.0.0"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{registration}");
    let token = registration["access_token"].as_str().unwrap().to_owned();
    let user_id = registration["user"]["id"].as_str().unwrap().to_owned();

    let updated_at =
        (Utc::now() - Duration::seconds(10)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let (status, pushed) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(&token),
        Some(json!({
            "device_id": external_device_id,
            "changes": [
                {
                    "ledger_id": "ledger-web-1",
                    "entity_type": "ledger",
                    "entity_sync_id": "ledger-web-1",
                    "action": "upsert",
                    "payload": {"syncId":"ledger-web-1","ledgerName":"共同账本","currency":"CNY","monthStartDay":1},
                    "updated_at": updated_at,
                    "scope": "ledger"
                },
                {
                    "ledger_id": null,
                    "entity_type": "account",
                    "entity_sync_id": "account-web-1",
                    "action": "upsert",
                    "payload": {"syncId":"account-web-1","name":"现金","type":"cash","currency":"CNY","initialBalance":100},
                    "updated_at": updated_at,
                    "scope": "user"
                },
                {
                    "ledger_id": null,
                    "entity_type": "category",
                    "entity_sync_id": "category-web-1",
                    "action": "upsert",
                    "payload": {"syncId":"category-web-1","name":"餐饮","kind":"expense"},
                    "updated_at": updated_at,
                    "scope": "user"
                },
                {
                    "ledger_id": null,
                    "entity_type": "tag",
                    "entity_sync_id": "tag-web-1",
                    "action": "upsert",
                    "payload": {"syncId":"tag-web-1","name":"早餐","color":"#ffaa00"},
                    "updated_at": updated_at,
                    "scope": "user"
                },
                {
                    "ledger_id": "ledger-web-1",
                    "entity_type": "budget",
                    "entity_sync_id": "budget-web-1",
                    "action": "upsert",
                    "payload": {"syncId":"budget-web-1","type":"category","categoryId":"category-web-1","amount":500,"period":"monthly","startDay":1,"enabled":true},
                    "updated_at": updated_at,
                    "scope": "ledger"
                },
                {
                    "ledger_id": "ledger-web-1",
                    "entity_type": "transaction",
                    "entity_sync_id": "tx-web-1",
                    "action": "upsert",
                    "payload": {
                        "syncId":"tx-web-1",
                        "type":"expense",
                        "amount":12.34,
                        "currencyCode":"CNY",
                        "accountId":"account-web-1",
                        "categoryId":"category-web-1",
                        "tagIds":["tag-web-1"],
                        "happenedAt":"2026-08-20T01:00:00Z",
                        "note":"早餐"
                    },
                    "updated_at": updated_at,
                    "scope": "ledger"
                }
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pushed}");
    assert_eq!(pushed["accepted"], 6);

    let stored: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM sync_entities WHERE user_id=$1::uuid AND entity_type LIKE 'finance.%' AND is_deleted=FALSE",
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(stored, 6);

    let (status, integration) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/status",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{integration}");
    assert_eq!(integration["enabled"], true);
    assert_eq!(integration["storage"], "lifetrace-postgresql");
    assert_eq!(integration["upstreamReachable"], true);

    let (status, ledgers) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/ledgers",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ledgers}");
    assert_eq!(ledgers["storage"], "lifetrace-postgresql");
    assert_eq!(ledgers["items"][0]["sourceId"], "ledger-web-1");
    assert_eq!(ledgers["items"][0]["transactionCount"], 1);
    assert_eq!(ledgers["items"][0]["expenseTotalCents"], 1234);

    let (status, snapshot) = send(
        router,
        Method::GET,
        "/api/v1/integrations/beecount/ledgers/ledger-web-1/snapshot?limit=500&offset=0",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{snapshot}");
    assert_eq!(snapshot["storage"], "lifetrace-postgresql");
    assert_eq!(snapshot["ledger"]["name"], "共同账本");
    assert_eq!(snapshot["transactions"]["items"][0]["amountCents"], 1234);
    assert_eq!(snapshot["transactions"]["items"][0]["accountName"], "现金");
    assert_eq!(snapshot["transactions"]["items"][0]["categoryName"], "餐饮");
    assert_eq!(snapshot["transactions"]["items"][0]["tags"][0], "早餐");
    assert_eq!(snapshot["accounts"][0]["transactionCount"], 1);
    assert_eq!(snapshot["categories"][0]["transactionCount"], 1);
    assert_eq!(snapshot["tags"][0]["expenseTotalCents"], 1234);
    assert_eq!(snapshot["budgets"][0]["amountCents"], 50000);
}
