//! BeeCount attachment protocol checks backed by PostgreSQL.

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use lifetrace_cloud::{app, AppState, Config};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

fn config(database_url: String) -> Config {
    Config {
        database_url: Some(database_url),
        migration_on_startup: true,
        dev_auth_enabled: false,
        auth_registration_mode: "open".to_owned(),
        auth_password_pepper: Some("beecount-file-password-pepper-0123456789".to_owned()),
        auth_token_hash_pepper: Some("beecount-file-token-pepper-012345678901".to_owned()),
        cursor_signing_key: Some("beecount-file-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("beecount-file-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        ..Config::default()
    }
}

async fn json_request(
    router: axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    value: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .oneshot(builder.body(Body::from(value.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn multipart_body(
    boundary: &str,
    ledger_id: Option<&str>,
    file_name: &str,
    mime_type: &str,
    content: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    if let Some(ledger_id) = ledger_id {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"ledger_id\"\r\n\r\n{ledger_id}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: {mime_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

async fn upload(
    router: axum::Router,
    uri: &str,
    token: &str,
    ledger_id: Option<&str>,
    content: &[u8],
) -> (StatusCode, Value) {
    let boundary = format!("lifetrace-{}", Uuid::new_v4());
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(multipart_body(
                    &boundary,
                    ledger_id,
                    "午餐-收据.txt",
                    "text/plain",
                    content,
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn stock_attachment_routes_deduplicate_and_write_file_metadata() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    state.initialize().await.unwrap();
    let router = app(state.clone());
    let external_device_id = format!("beecount-file-device-{}", Uuid::new_v4());

    let (status, registration) = json_request(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/auth/register",
        None,
        json!({
            "email": format!("beecount-file-{}@example.test", Uuid::new_v4()),
            "password": "正确 horse battery staple 文件密码",
            "device_id": external_device_id,
            "device_name": "BeeCount Android",
            "platform": "android",
            "app_version": "1.0.0"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{registration}");
    let token = registration["access_token"].as_str().unwrap();
    let user_id = registration["user"]["id"].as_str().unwrap();

    let (status, pushed) = json_request(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/sync/push",
        Some(token),
        json!({
            "device_id": external_device_id,
            "changes": [{
                "ledger_id": "ledger-files",
                "entity_type": "ledger",
                "entity_sync_id": "ledger-files",
                "action": "upsert",
                "payload": {"syncId": "ledger-files", "ledgerName": "附件账本", "currency": "CNY"},
                "updated_at": chrono::Utc::now(),
                "scope": "ledger"
            }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pushed}");

    let content = b"LifeTrace BeeCount attachment bytes";
    let attachment_path = "/api/v1/integrations/beecount/compat/attachments/upload";
    let (status, first) = upload(
        router.clone(),
        attachment_path,
        token,
        Some("ledger-files"),
        content,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["ledger_id"], "ledger-files");
    assert_eq!(first["sha256"], hex::encode(Sha256::digest(content)));
    assert_eq!(first["file_name"], "午餐-收据.txt");

    let (_, duplicate) = upload(
        router.clone(),
        attachment_path,
        token,
        Some("ledger-files"),
        content,
    )
    .await;
    assert_eq!(duplicate["file_id"], first["file_id"]);

    let (status, exists) = json_request(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/attachments/batch-exists",
        Some(token),
        json!({
            "ledger_id": "ledger-files",
            "sha256_list": [first["sha256"], "0".repeat(64)]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{exists}");
    assert_eq!(exists["items"][0]["exists"], true);
    assert_eq!(exists["items"][1]["exists"], false);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/integrations/beecount/compat/attachments/{}",
                    first["file_id"].as_str().unwrap()
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "text/plain");
    let downloaded = to_bytes(response.into_body(), content.len()).await.unwrap();
    assert_eq!(&downloaded[..], content);

    let icon_path = "/api/v1/integrations/beecount/compat/attachments/category-icons/upload";
    let (status, icon) = upload(router.clone(), icon_path, token, None, b"icon-bytes").await;
    assert_eq!(status, StatusCode::OK, "{icon}");
    assert_eq!(icon["ledger_id"], "");
    let (_, icon_duplicate) = upload(router, icon_path, token, None, b"icon-bytes").await;
    assert_eq!(icon_duplicate["file_id"], icon["file_id"]);

    let metadata_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM sync_entities WHERE user_id=$1 AND entity_type='file.metadata'",
    )
    .bind(Uuid::parse_str(user_id).unwrap())
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(metadata_count, 2);
}
