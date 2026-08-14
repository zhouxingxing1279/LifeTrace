//! Regression coverage for a single LifeTrace identity being used concurrently
//! by native LifeTrace clients and the stock BeeCount client.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lifetrace_cloud::auth::security::RequestContext;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::auth::v1::RegisterRequestV1;
use lifetrace_contracts::sync::v1::AppId;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn config(database_url: String) -> Config {
    Config {
        database_url: Some(database_url),
        migration_on_startup: true,
        dev_auth_enabled: false,
        auth_registration_mode: "open".to_owned(),
        auth_password_pepper: Some("shared-account-password-pepper-0123456789".to_owned()),
        auth_token_hash_pepper: Some("shared-account-token-pepper-012345678901".to_owned()),
        cursor_signing_key: Some("shared-account-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("shared-account-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        ..Config::default()
    }
}

fn context() -> RequestContext {
    RequestContext {
        ip: Some("127.0.0.1".parse().unwrap()),
        user_agent: Some("LifeTrace/BeeCount shared account test".to_owned()),
        origin: Some("http://localhost:3000".to_owned()),
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
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
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
async fn one_account_can_keep_lifetrace_and_beecount_sessions_active_together() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let state = AppState::new(config(database_url));
    state.initialize().await.unwrap();

    let email = format!("shared-account-{}@example.test", Uuid::new_v4());
    let password = "正确 horse battery staple 密码".to_owned();
    let lifetrace_device_id = format!("lifetrace-device-{}", Uuid::new_v4());
    let beecount_device_id = format!("beecount-device-{}", Uuid::new_v4());

    // Create the canonical LifeTrace account and keep its native session alive.
    let native = state
        .auth_service
        .register(
            RegisterRequestV1 {
                email: email.clone(),
                password: password.clone(),
                display_name: Some("Shared Account".to_owned()),
                invite_token: None,
                app_id: AppId::new(AppId::DESKTOP),
                device_id: lifetrace_device_id,
                device_name: "LifeTrace Desktop".to_owned(),
                platform: "windows".to_owned(),
                client_version: Some("0.2.1".to_owned()),
                requested_scopes: vec![],
            },
            &context(),
        )
        .await
        .unwrap();
    let canonical_user_id = native.user.id.as_str().to_owned();
    let native_access_token = native.access_token.clone();

    // Log into the BeeCount compatibility surface with the exact same email
    // and password. This must resolve to the existing cloud_users row rather
    // than creating or requiring a second BeeCount account.
    let router = app(state.clone());
    let (status, bee_login) = send(
        router.clone(),
        Method::POST,
        "/api/v1/integrations/beecount/compat/auth/login",
        None,
        Some(json!({
            "email": email,
            "password": password,
            "device_id": beecount_device_id,
            "device_name": "BeeCount Android",
            "platform": "android",
            "app_version": "1.6.3"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{bee_login}");
    assert_eq!(bee_login["user"]["id"], canonical_user_id);
    let bee_access_token = bee_login["access_token"].as_str().unwrap().to_owned();

    // BeeCount login must not invalidate the already-active LifeTrace session.
    let (status, native_me) = send(
        router.clone(),
        Method::GET,
        "/api/v1/auth/me",
        Some(&native_access_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{native_me}");
    assert_eq!(native_me["id"], canonical_user_id);

    // The BeeCount session must independently remain valid and expose the same
    // canonical identity through the BeeCount-compatible profile surface.
    let (status, bee_profile) = send(
        router,
        Method::GET,
        "/api/v1/integrations/beecount/compat/profile/me",
        Some(&bee_access_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{bee_profile}");
    assert_eq!(bee_profile["user_id"], canonical_user_id);

    let user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM cloud_users WHERE id=$1::uuid",
    )
    .bind(&canonical_user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(user_count, 1);

    let sessions: Vec<(String, String)> = sqlx::query_as(
        "SELECT app_id,status FROM auth_sessions \
         WHERE user_id=$1::uuid AND status='active' ORDER BY app_id",
    )
    .bind(&canonical_user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap();
    assert!(
        sessions
            .iter()
            .any(|(app_id, status)| app_id == AppId::DESKTOP && status == "active"),
        "native LifeTrace session missing: {sessions:?}"
    );
    assert!(
        sessions
            .iter()
            .any(|(app_id, status)| app_id == AppId::BEECOUNT && status == "active"),
        "BeeCount session missing: {sessions:?}"
    );
}
