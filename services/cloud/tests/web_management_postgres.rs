//! PostgreSQL-backed browser device/session management acceptance tests.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lifetrace_cloud::auth::security::RequestContext;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::auth::v1::RegisterRequestV1;
use lifetrace_contracts::sync::v1::AppId;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn config(url: String) -> Config {
    Config {
        database_url: Some(url),
        migration_on_startup: true,
        dev_auth_enabled: false,
        auth_registration_mode: "open".to_owned(),
        auth_password_pepper: Some("test-password-pepper-01234567890123456789".to_owned()),
        auth_token_hash_pepper: Some("test-token-pepper-0123456789012345678901".to_owned()),
        cursor_signing_key: Some("web-management-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("web-management-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        cors_allowed_origins: vec!["http://localhost:3000".to_owned()],
        ..Config::default()
    }
}

async fn state() -> Option<AppState> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let state = AppState::new(config(url));
    state.initialize().await.unwrap();
    sqlx::query(
        "TRUNCATE TABLE auth_audit_log, auth_login_attempts, auth_registration_invites, \
         auth_password_reset_tokens, auth_web_sessions, auth_refresh_tokens, auth_access_tokens, \
         auth_sessions, auth_app_grants, sync_snapshot_items, sync_snapshots, \
         sync_processed_changes, sync_change_log, sync_entities, cloud_devices, cloud_users \
         RESTART IDENTITY CASCADE",
    )
    .execute(&state.pool)
    .await
    .unwrap();
    Some(state)
}

fn request_context() -> RequestContext {
    RequestContext {
        ip: Some("127.0.0.1".parse().unwrap()),
        user_agent: Some("LifeTrace Web management integration test".to_owned()),
        origin: Some("http://localhost:3000".to_owned()),
    }
}

async fn json_body(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap_or(Value::Null)
}

#[tokio::test]
async fn web_cookie_can_list_and_csrf_protect_device_management() {
    let Some(state) = state().await else {
        return;
    };
    let email = format!("{}@example.test", Uuid::new_v4());
    let password = "正确 horse battery staple 密码".to_owned();
    state
        .auth_service
        .register(
            RegisterRequestV1 {
                email: email.clone(),
                password: password.clone(),
                display_name: Some("Web Management Test".to_owned()),
                invite_token: None,
                app_id: AppId::new(AppId::DESKTOP),
                device_id: Uuid::new_v4().to_string(),
                device_name: "Registration Device".to_owned(),
                platform: "windows".to_owned(),
                client_version: Some("0.2.1".to_owned()),
                requested_scopes: vec![],
            },
            &request_context(),
        )
        .await
        .unwrap();

    let router = app(state);
    let login = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/web/session/login")
                .header("content-type", "application/json")
                .header("origin", "http://localhost:3000")
                .body(Body::from(
                    json!({
                        "email": email,
                        "password": password,
                        "requestedScopes": [
                            "devices:read", "devices:write",
                            "sessions:read", "sessions:write"
                        ],
                        "publicDevice": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let login_payload = json_body(login).await;
    let csrf = login_payload["csrfToken"].as_str().unwrap().to_owned();

    let devices = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/web/devices")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(devices.status(), StatusCode::OK);
    let devices_payload = json_body(devices).await;
    let current_device = devices_payload["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|device| device["current"] == Value::Bool(true))
        .unwrap();
    let device_id = current_device["id"].as_str().unwrap();

    let missing_csrf = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/web/devices/{device_id}"))
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .header("origin", "http://localhost:3000")
                .body(Body::from(json!({"deviceName":"Renamed Web"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let renamed = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/web/devices/{device_id}"))
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .header("origin", "http://localhost:3000")
                .header("x-csrf-token", &csrf)
                .body(Body::from(json!({"deviceName":"Renamed Web"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(renamed.status(), StatusCode::OK);
    assert_eq!(json_body(renamed).await["deviceName"], "Renamed Web");

    let sessions = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/web/sessions")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sessions.status(), StatusCode::OK);
    assert!(!json_body(sessions).await["sessions"].as_array().unwrap().is_empty());
}
