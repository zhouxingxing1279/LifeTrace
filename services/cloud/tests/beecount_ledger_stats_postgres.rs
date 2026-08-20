//! Regression for the stock BeeCount `GET /api/v1/read/ledgers/{id}/stats` call.

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
        auth_password_pepper: Some("beecount-stats-password-pepper-0123456789".to_owned()),
        auth_token_hash_pepper: Some("beecount-stats-token-pepper-012345678901".to_owned()),
        cursor_signing_key: Some("beecount-stats-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("beecount-stats-page-token-key".to_owned()),
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

async fn register(router: axum::Router) -> (String, String, String) {
    let external_device_id = format!("beecount-device-{}", Uuid::new_v4());
    let email = format!("beecount-stats-{}@example.test", Uuid::new_v4());
    let (status, registration) = send(
        router,
        Method::POST,
        "/api/v1/integrations/beecount/compat/auth/register",
        None,
        Some(json!({
            "email": email,
            "password": "正确 horse battery staple stats 密码",
            "device_id": external_device_id,
            "device_name": "BeeCount Android",
            "platform": "android",
            "app_version": "1.0.0"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{registration}");
    (
        registration["access_token"].as_str().unwrap().to_owned(),
        external_device_id,
        registration["user"]["id"].as_str().unwrap().to_owned(),
    )
}

#[tokio::test]
async fn stock_ledger_stats_match_current_ledger_and_user_totals() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    state.initialize().await.unwrap();
    let router = app(state.clone());
    let (token, external_device_id, user_id) = register(router.clone()).await;
    let updated_at =
        (Utc::now() - Duration::seconds(30)).to_rfc3339_opts(SecondsFormat::Millis, true);

    let changes = json!({
        "device_id": external_device_id,
        "changes": [
            {
                "ledger_id": "ledger-a", "entity_type": "ledger", "entity_sync_id": "ledger-a",
                "action": "upsert", "payload": {"syncId":"ledger-a","ledgerName":"A","currency":"CNY","monthStartDay":1},
                "updated_at": updated_at, "scope": "ledger"
            },
            {
                "ledger_id": "ledger-b", "entity_type": "ledger", "entity_sync_id": "ledger-b",
                "action": "upsert", "payload": {"syncId":"ledger-b","ledgerName":"B","currency":"CNY","monthStartDay":1},
                "updated_at": updated_at, "scope": "ledger"
            },
            {
                "ledger_id": null, "entity_type": "account", "entity_sync_id": "account-1",
                "action": "upsert", "payload": {"syncId":"account-1","name":"现金","type":"cash","currency":"CNY","initialBalance":0},
                "updated_at": updated_at, "scope": "user"
            },
            {
                "ledger_id": null, "entity_type": "category", "entity_sync_id": "category-1",
                "action": "upsert", "payload": {"syncId":"category-1","name":"餐饮","kind":"expense","level":1,"sortOrder":1},
                "updated_at": updated_at, "scope": "user"
            },
            {
                "ledger_id": null, "entity_type": "tag", "entity_sync_id": "tag-1",
                "action": "upsert", "payload": {"syncId":"tag-1","name":"测试","color":"#123456","sortOrder":1},
                "updated_at": updated_at, "scope": "user"
            },
            {
                "ledger_id": "ledger-a", "entity_type": "transaction", "entity_sync_id": "tx-a-1",
                "action": "upsert", "payload": {"syncId":"tx-a-1","type":"expense","amount":10,"currencyCode":"CNY","accountId":"account-1","categoryId":"category-1","tagIds":["tag-1"],"happenedAt":"2026-08-20T01:00:00Z"},
                "updated_at": updated_at, "scope": "ledger"
            },
            {
                "ledger_id": "ledger-a", "entity_type": "transaction", "entity_sync_id": "tx-a-2",
                "action": "upsert", "payload": {"syncId":"tx-a-2","type":"expense","amount":20,"currencyCode":"CNY","accountId":"account-1","happenedAt":"2026-08-20T02:00:00Z"},
                "updated_at": updated_at, "scope": "ledger"
            },
            {
                "ledger_id": "ledger-b", "entity_type": "transaction", "entity_sync_id": "tx-b-1",
                "action": "upsert", "payload": {"syncId":"tx-b-1","type":"income","amount":30,"currencyCode":"CNY","accountId":"account-1","happenedAt":"2026-08-20T03:00:00Z"},
                "updated_at": updated_at, "scope": "ledger"
            },
            {
                "ledger_id": "ledger-a", "entity_type": "budget", "entity_sync_id": "budget-a",
                "action": "upsert", "payload": {"syncId":"budget-a","type":"total","amount":1000,"period":"monthly","startDay":1,"enabled":true},
                "updated_at": updated_at, "scope": "ledger"
            },
            {
                "ledger_id": "ledger-b", "entity_type": "budget", "entity_sync_id": "budget-b",
                "action": "upsert", "payload": {"syncId":"budget-b","type":"total","amount":2000,"period":"monthly","startDay":1,"enabled":true},
                "updated_at": updated_at, "scope": "ledger"
            }
        ]
    });
    let (status, pushed) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(&token),
        Some(changes),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pushed}");
    assert_eq!(pushed["accepted"], 10, "{pushed}");
    assert_eq!(pushed["rejected"], 0, "{pushed}");

    let user_uuid = Uuid::parse_str(&user_id).unwrap();
    for (ledger_id, kind, sha_byte) in [
        (Some("ledger-a"), "transaction_attachment", 'a'),
        (Some("ledger-b"), "transaction_attachment", 'b'),
        (None, "category_icon", 'c'),
    ] {
        let id = Uuid::new_v4();
        let sha256 = sha_byte.to_string().repeat(64);
        sqlx::query(
            "INSERT INTO cloud_file_blobs \
             (id,user_id,file_entity_id,ledger_id,attachment_kind,sha256,size_bytes,file_name,content) \
             VALUES ($1,$2,$3,$4,$5,$6,1,$7,$8)",
        )
        .bind(id)
        .bind(user_uuid)
        .bind(format!("beecount-file:{id}"))
        .bind(ledger_id)
        .bind(kind)
        .bind(sha256)
        .bind(format!("{kind}.bin"))
        .bind(vec![1_u8])
        .execute(&state.pool)
        .await
        .unwrap();
    }

    let (status, stats) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/read/ledgers/ledger-a/stats",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{stats}");
    assert_eq!(stats["transaction_count"], 2);
    assert_eq!(stats["transaction_total"], 3);
    assert_eq!(stats["transaction_attachment_count"], 1);
    assert_eq!(stats["transaction_attachment_total"], 2);
    assert_eq!(stats["budget_count"], 1);
    assert_eq!(stats["budget_total"], 2);
    assert_eq!(stats["account_count"], 1);
    assert_eq!(stats["account_total"], 1);
    assert_eq!(stats["category_count"], 1);
    assert_eq!(stats["category_total"], 1);
    assert_eq!(stats["category_attachment_count"], 1);
    assert_eq!(stats["category_attachment_total"], 1);
    assert_eq!(stats["tag_count"], 1);
    assert_eq!(stats["tag_total"], 1);

    let (other_token, _, _) = register(router.clone()).await;
    let (status, _) = send(
        router,
        Method::GET,
        "/api/v1/integrations/beecount/compat/read/ledgers/ledger-a/stats",
        Some(&other_token),
        None,
    )
    .await;
    assert_ne!(status, StatusCode::OK);
}
