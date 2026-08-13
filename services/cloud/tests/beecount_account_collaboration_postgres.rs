//! PostgreSQL acceptance coverage for BeeCount profile/device/shared-ledger compatibility.

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
        auth_password_pepper: Some("beecount-phase5-password-pepper-0123456789".to_owned()),
        auth_token_hash_pepper: Some("beecount-phase5-token-pepper-012345678901".to_owned()),
        cursor_signing_key: Some("beecount-phase5-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("beecount-phase5-page-token-key".to_owned()),
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
    response_json(router, builder.body(body).unwrap()).await
}

async fn response_json(router: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.oneshot(request).await.unwrap();
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
            "email":email,"password":"正确 horse battery staple 密码",
            "device_id":device_id,"device_name":format!("{label} Android"),
            "platform":"android","app_version":"1.2.3","os_version":"15",
            "device_model":"Test Phone"
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

#[tokio::test]
async fn stock_profile_devices_and_shared_ledger_flow_use_lifetrace_state() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    state.initialize().await.unwrap();
    let router = app(state.clone());
    let (owner_token, owner_id, owner_device) = register(router.clone(), "owner").await;
    let (editor_token, editor_id, editor_device) = register(router.clone(), "editor").await;

    let (status, profile) = send(
        router.clone(),
        Method::PATCH,
        "/api/v1/integrations/beecount/compat/profile/me",
        Some(&owner_token),
        Some(json!({
            "display_name":"家庭账本 Owner","income_is_red":true,
            "theme_primary_color":"#12abef","appearance":{"compact_amount":true},
            "ai_config":{"use_vision":true},"primary_currency":"cny"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["display_name"], "家庭账本 Owner");
    assert_eq!(profile["theme_primary_color"], "#12ABEF");
    assert_eq!(profile["primary_currency"], "CNY");

    let boundary = format!("phase5-{}", Uuid::new_v4());
    let avatar = b"not-decoded-by-server";
    let multipart = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"avatar.png\"\r\nContent-Type: image/png\r\n\r\n{}\r\n--{boundary}--\r\n",
        String::from_utf8_lossy(avatar)
    );
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integrations/beecount/compat/profile/avatar")
        .header("authorization", format!("Bearer {owner_token}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(multipart))
        .unwrap();
    let (status, uploaded) = response_json(router.clone(), request).await;
    assert_eq!(status, StatusCode::OK, "{uploaded}");
    assert_eq!(uploaded["avatar_version"], 1);

    let (status, devices) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/devices?view=deduped&active_within_days=30",
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{devices}");
    assert_eq!(devices[0]["id"], owner_device);
    assert_eq!(devices[0]["os_version"], "15");
    assert_eq!(devices[0]["device_model"], "Test Phone");

    let updated_at =
        (Utc::now() - Duration::seconds(10)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let (status, pushed) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(&owner_token),
        Some(json!({
            "device_id":owner_device,
            "changes":[
                {"ledger_id":"shared-ledger-1","entity_type":"ledger","entity_sync_id":"shared-ledger-1","action":"upsert","payload":{"syncId":"shared-ledger-1","ledgerName":"家庭共享","currency":"CNY","monthStartDay":1},"updated_at":updated_at,"scope":"ledger"},
                {"ledger_id":null,"entity_type":"category","entity_sync_id":"category-food","action":"upsert","payload":{"syncId":"category-food","name":"餐饮","kind":"expense","sortOrder":1},"updated_at":updated_at,"scope":"user"},
                {"ledger_id":null,"entity_type":"account","entity_sync_id":"account-cash","action":"upsert","payload":{"syncId":"account-cash","name":"现金","type":"cash","currency":"CNY","initialBalance":0},"updated_at":updated_at,"scope":"user"},
                {"ledger_id":"shared-ledger-1","entity_type":"transaction","entity_sync_id":"owner-tx","action":"upsert","payload":{"syncId":"owner-tx","type":"expense","amount":12.3,"currencyCode":"CNY","accountId":"account-cash","categoryId":"category-food","happenedAt":"2026-08-13T10:00:00Z"},"updated_at":updated_at,"scope":"ledger"}
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pushed}");
    assert_eq!(pushed["accepted"], 4);

    let (status, invite) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/ledgers/shared-ledger-1/invites",
        Some(&owner_token),
        Some(json!({"role":"editor","expires_in_hours":24})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{invite}");
    let code = invite["code"].as_str().unwrap();

    let (status, preview) = send(
        router.clone(),
        Method::POST,
        &format!("/api/v1/integrations/beecount/compat/invites/{code}/preview"),
        Some(&editor_token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["ledger_name"], "家庭共享");

    let (status, accepted) = send(
        router.clone(),
        Method::POST,
        &format!("/api/v1/integrations/beecount/compat/invites/{code}/accept"),
        Some(&editor_token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{accepted}");
    assert_eq!(accepted["role"], "editor");
    assert_eq!(accepted["member_count"], 2);

    let (status, members) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/ledgers/shared-ledger-1/members",
        Some(&editor_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{members}");
    assert_eq!(members.as_array().unwrap().len(), 2);
    assert!(members
        .as_array()
        .unwrap()
        .iter()
        .any(|member| member["user_id"] == editor_id && member["is_self"] == true));

    let (status, ledgers) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/sync/ledgers",
        Some(&editor_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ledgers}");
    assert_eq!(ledgers[0]["ledger_id"], "shared-ledger-1");
    assert_eq!(ledgers[0]["role"], "editor");

    let (status, resources) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/ledgers/shared-ledger-1/shared-resources",
        Some(&editor_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{resources}");
    assert_eq!(resources["owner_user_id"], owner_id);
    assert_eq!(resources["categories"][0]["sync_id"], "category-food");
    assert_eq!(resources["accounts"][0]["sync_id"], "account-cash");

    let editor_updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let (status, editor_push) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(&editor_token),
        Some(json!({
            "device_id":editor_device,
            "changes":[{"ledger_id":"shared-ledger-1","entity_type":"transaction","entity_sync_id":"editor-tx","action":"upsert","payload":{"syncId":"editor-tx","type":"expense","amount":8.8,"currencyCode":"CNY","happenedAt":"2026-08-13T11:00:00Z"},"updated_at":editor_updated_at,"scope":"ledger"}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{editor_push}");
    assert_eq!(editor_push["accepted"], 1);

    let owner_internal = Uuid::parse_str(&owner_id).unwrap();
    let editor_internal = Uuid::parse_str(&editor_id).unwrap();
    let owner_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM sync_entities WHERE user_id=$1 \
         AND entity_type='finance.transaction' AND entity_id='beecount:editor-tx'",
    )
    .bind(owner_internal)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let editor_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM sync_entities WHERE user_id=$1 \
         AND entity_type='finance.transaction' AND entity_id='beecount:editor-tx'",
    )
    .bind(editor_internal)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(owner_rows, 1);
    assert_eq!(editor_rows, 0, "shared finance data must not be duplicated");

    let (status, full) = send(
        router.clone(),
        Method::GET,
        "/api/v1/integrations/beecount/compat/sync/full?ledger_id=shared-ledger-1",
        Some(&editor_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{full}");
    let snapshot: Value =
        serde_json::from_str(full["snapshot"]["payload"]["content"].as_str().unwrap()).unwrap();
    assert_eq!(snapshot["items"].as_array().unwrap().len(), 2);

    let (status, transferred) = send(
        router,
        Method::POST,
        "/api/v1/integrations/beecount/compat/ledgers/shared-ledger-1/transfer",
        Some(&owner_token),
        Some(json!({"new_owner_user_id":editor_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{transferred}");
    assert!(transferred
        .as_array()
        .unwrap()
        .iter()
        .any(|member| member["user_id"] == editor_id && member["role"] == "owner"));
}
