//! PostgreSQL-backed Web/PWA sync authentication acceptance tests.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lifetrace_cloud::auth::security::RequestContext;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::auth::v1::RegisterRequestV1;
use lifetrace_contracts::sync::v1::AppId;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn config(url: String) -> Config {
    Config {
        database_url: Some(url),
        migration_on_startup: true,
        dev_auth_enabled: false,
        auth_registration_mode: "open".to_owned(),
        auth_password_pepper: Some("test-password-pepper-01234567890123456789".to_owned()),
        auth_token_hash_pepper: Some("test-token-pepper-0123456789012345678901".to_owned()),
        cursor_signing_key: Some("web-sync-test-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("web-sync-test-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        cors_allowed_origins: vec!["http://localhost:3000".to_owned()],
        ..Config::default()
    }
}

async fn state() -> Option<AppState> {
    let url = database_url()?;
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

fn context() -> RequestContext {
    RequestContext {
        ip: Some("127.0.0.1".parse().unwrap()),
        user_agent: Some("LifeTrace Web sync integration test".to_owned()),
        origin: Some("http://localhost:3000".to_owned()),
    }
}

fn register() -> RegisterRequestV1 {
    RegisterRequestV1 {
        email: format!("{}@example.test", Uuid::new_v4()),
        password: "正确 horse battery staple 密码".to_owned(),
        display_name: Some("Web Sync Test".to_owned()),
        invite_token: None,
        app_id: AppId::new(AppId::DESKTOP),
        device_id: Uuid::new_v4().to_string(),
        device_name: "Registration Device".to_owned(),
        platform: "windows".to_owned(),
        client_version: Some("0.2.1".to_owned()),
        requested_scopes: vec![],
    }
}

async fn body_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap_or(Value::Null)
}

fn pull_body() -> Value {
    json!({
        "requestId": "web-sync-pull",
        "client": {
            "appId": AppId::WEB,
            "clientVersion": "0.2.1",
            "platform": "web",
            "protocolVersion": 1,
            "schemaVersion": 1,
            "deviceId": "web-device"
        },
        "afterCursor": null,
        "limit": 10,
        "entityTypes": ["english.article"]
    })
}

fn sync_request(cookie: &str, csrf: Option<&str>, origin: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sync/pull")
        .header("content-type", "application/json")
        .header("cookie", cookie)
        .header("origin", origin);
    if let Some(csrf) = csrf {
        builder = builder.header("x-csrf-token", csrf);
    }
    builder
        .body(Body::from(pull_body().to_string()))
        .unwrap()
}

#[tokio::test]
async fn web_cookie_sync_requires_valid_csrf_and_origin() {
    let Some(state) = state().await else {
        return;
    };
    let registration = register();
    let email = registration.email.clone();
    let password = registration.password.clone();
    state
        .auth_service
        .register(registration, &context())
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
                        "requestedScopes": ["sync:read", "english:read"],
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
    let login_payload = body_json(login).await;
    let csrf = login_payload["csrfToken"].as_str().unwrap().to_owned();

    let allowed = router
        .clone()
        .oneshot(sync_request(
            &cookie,
            Some(&csrf),
            "http://localhost:3000",
        ))
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);

    let missing_csrf = router
        .clone()
        .oneshot(sync_request(&cookie, None, "http://localhost:3000"))
        .await
        .unwrap();
    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let attacker_origin = router
        .oneshot(sync_request(
            &cookie,
            Some(&csrf),
            "https://attacker.example",
        ))
        .await
        .unwrap();
    assert_eq!(attacker_origin.status(), StatusCode::FORBIDDEN);
}
