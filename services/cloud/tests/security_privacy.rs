//! EPIC-17 security, privacy and data-lifecycle acceptance tests.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lifetrace_cloud::auth::security::RequestContext;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::auth::v1::{RegisterRequestV1, Scope};
use lifetrace_contracts::sync::v1::AppId;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

const DEV_TOKEN: &str = "epic17-dev-token";

fn memory_app() -> Router {
    app(AppState::new(Config {
        dev_auth_token: DEV_TOKEN.to_owned(),
        dev_auth_user_id: "epic17-user".to_owned(),
        dev_auth_device_id: "epic17-device".to_owned(),
        ..Config::default()
    }))
}

async fn request(app: Router, method: Method, uri: &str, token: Option<&str>) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    app.oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn all_api_responses_receive_security_headers() {
    let response = request(memory_app(), Method::GET, "/health/live", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["referrer-policy"], "no-referrer");
    assert_eq!(headers["x-frame-options"], "DENY");
    assert_eq!(headers["cache-control"], "no-store");
    assert!(headers["content-security-policy"]
        .to_str()
        .unwrap()
        .contains("default-src 'none'"));
    assert!(headers["content-security-policy"]
        .to_str()
        .unwrap()
        .contains("frame-ancestors 'none'"));
}

#[tokio::test]
async fn production_responses_include_hsts() {
    let app = app(AppState::new(Config {
        environment: "production".to_owned(),
        ..Config::default()
    }));
    let response = request(app, Method::GET, "/health/live", None).await;
    assert_eq!(
        response.headers()["strict-transport-security"],
        "max-age=63072000; includeSubDomains"
    );
}

#[tokio::test]
async fn privacy_endpoints_reject_anonymous_access() {
    let response = request(memory_app(), Method::GET, "/api/v1/privacy/export", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn account_export_is_readable_and_contains_no_authentication_secret() {
    let response = request(
        memory_app(),
        Method::GET,
        "/api/v1/privacy/export/account",
        Some(DEV_TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["format"], "lifetrace-privacy-export-v1");
    assert_eq!(body["requestedModule"], "account");
    assert_eq!(body["sections"]["account"]["userId"], "epic17-user");
    let serialized = body.to_string().to_ascii_lowercase();
    assert!(!serialized.contains("password_hash"));
    assert!(!serialized.contains("access_token"));
    assert!(!serialized.contains("refresh_token"));
    assert!(!serialized.contains(DEV_TOKEN));
}

#[tokio::test]
async fn unknown_export_module_fails_closed() {
    let response = request(
        memory_app(),
        Method::GET,
        "/api/v1/privacy/export/future-secret",
        Some(DEV_TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn policy_is_authenticated_and_documents_object_cleanup_boundary() {
    let response = request(
        memory_app(),
        Method::GET,
        "/api/v1/privacy/policy",
        Some(DEV_TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert!(body["fileObjects"]
        .as_str()
        .unwrap()
        .contains("blocks account deletion"));
}

fn database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn postgres_config(url: String) -> Config {
    Config {
        database_url: Some(url),
        migration_on_startup: true,
        dev_auth_enabled: false,
        auth_registration_mode: "open".to_owned(),
        auth_password_pepper: Some("test-password-pepper-01234567890123456789".to_owned()),
        auth_token_hash_pepper: Some("test-token-pepper-0123456789012345678901".to_owned()),
        cursor_signing_key: Some("epic17-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("epic17-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        cors_allowed_origins: vec!["http://localhost:3000".to_owned()],
        ..Config::default()
    }
}

async fn postgres_state() -> Option<AppState> {
    let url = database_url()?;
    let state = AppState::new(postgres_config(url));
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

fn auth_context() -> RequestContext {
    RequestContext {
        ip: Some("127.0.0.1".parse().unwrap()),
        user_agent: Some("LifeTrace EPIC-17 integration test".to_owned()),
        origin: Some("http://localhost:3000".to_owned()),
    }
}

fn registration() -> RegisterRequestV1 {
    RegisterRequestV1 {
        email: format!("epic17-{}@example.test", Uuid::new_v4()),
        password: "正确 horse battery staple 密码".to_owned(),
        display_name: Some("Privacy Test".to_owned()),
        invite_token: None,
        app_id: AppId::new(AppId::DESKTOP),
        device_id: Uuid::new_v4().to_string(),
        device_name: "EPIC-17 Device".to_owned(),
        platform: "windows".to_owned(),
        client_version: Some("0.2.1".to_owned()),
        requested_scopes: Vec::<Scope>::new(),
    }
}

#[tokio::test]
async fn postgres_account_deletion_removes_user_sessions_and_tokens() {
    let Some(state) = postgres_state().await else {
        return;
    };
    let issued = state
        .auth_service
        .register(registration(), &auth_context())
        .await
        .unwrap();
    let user_id = Uuid::parse_str(issued.user.id.as_str()).unwrap();

    let response = request(
        app(state.clone()),
        Method::DELETE,
        "/api/v1/privacy/account",
        Some(&issued.access_token),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cloud_users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_sessions WHERE user_id=$1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let access_tokens: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_access_tokens t JOIN auth_sessions s ON s.id=t.session_id WHERE s.user_id=$1",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(users, 0);
    assert_eq!(sessions, 0);
    assert_eq!(access_tokens, 0);
}

#[tokio::test]
async fn postgres_full_export_excludes_password_and_token_hashes() {
    let Some(state) = postgres_state().await else {
        return;
    };
    let issued = state
        .auth_service
        .register(registration(), &auth_context())
        .await
        .unwrap();
    let response = request(
        app(state),
        Method::GET,
        "/api/v1/privacy/export",
        Some(&issued.access_token),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let serialized = body.to_string().to_ascii_lowercase();
    assert!(!serialized.contains("password_hash"));
    assert!(!serialized.contains("token_hash"));
    assert!(!serialized.contains("credential_ciphertext"));
    assert!(!serialized.contains(&issued.access_token.to_ascii_lowercase()));
    if let Some(refresh_token) = issued.refresh_token {
        assert!(!serialized.contains(&refresh_token.to_ascii_lowercase()));
    }
}
