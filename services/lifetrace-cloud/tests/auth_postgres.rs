//! PostgreSQL-backed EPIC-04 authentication acceptance tests.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lifetrace_cloud::auth::security::RequestContext;
use lifetrace_cloud::auth::AuthCredential;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::auth::v1::*;
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
        cursor_signing_key: Some("auth-test-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("auth-test-page-token-key".to_owned()),
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
        user_agent: Some("LifeTrace auth integration test".to_owned()),
        origin: Some("http://localhost:3000".to_owned()),
    }
}

fn register(app_id: &str, requested_scopes: Vec<Scope>) -> RegisterRequestV1 {
    RegisterRequestV1 {
        email: format!("{}@example.test", Uuid::new_v4()),
        password: "正确 horse battery staple 密码".to_owned(),
        display_name: Some("Auth Test".to_owned()),
        invite_token: None,
        app_id: AppId::new(app_id),
        device_id: Uuid::new_v4().to_string(),
        device_name: "Integration Device".to_owned(),
        platform: "windows".to_owned(),
        client_version: Some("0.2.1".to_owned()),
        requested_scopes,
    }
}

#[tokio::test]
async fn native_login_refresh_rotation_and_reuse_revocation() {
    let Some(state) = state().await else {
        return;
    };
    let initial = state
        .auth_service
        .register(register(AppId::DESKTOP, vec![]), &context())
        .await
        .unwrap();
    let refresh = initial.refresh_token.clone().unwrap();
    let device_id = sqlx::query_scalar::<_, String>(
        "SELECT external_device_id FROM cloud_devices WHERE id=$1::uuid",
    )
    .bind(initial.session.device_id.as_str())
    .fetch_one(&state.pool)
    .await
    .unwrap();

    let rotated = state
        .auth_service
        .refresh(
            RefreshRequestV1 {
                refresh_token: refresh.clone(),
                app_id: AppId::new(AppId::DESKTOP),
                device_id,
            },
            &context(),
        )
        .await
        .unwrap();
    assert_ne!(rotated.refresh_token.as_deref(), Some(refresh.as_str()));

    let reused = state
        .auth_service
        .refresh(
            RefreshRequestV1 {
                refresh_token: refresh,
                app_id: AppId::new(AppId::DESKTOP),
                device_id: sqlx::query_scalar(
                    "SELECT external_device_id FROM cloud_devices WHERE id=$1::uuid",
                )
                .bind(initial.session.device_id.as_str())
                .fetch_one(&state.pool)
                .await
                .unwrap(),
            },
            &context(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        reused.body.code.wire_name(),
        "LIFETRACE_AUTH_REFRESH_TOKEN_REUSED"
    );

    let session_status: String =
        sqlx::query_scalar("SELECT status FROM auth_sessions WHERE id=$1::uuid")
            .bind(initial.session.id.as_str())
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(session_status, "revoked");
}

#[tokio::test]
async fn app_policy_and_sync_scope_prevent_cross_domain_access() {
    let Some(state) = state().await else {
        return;
    };
    let tokens = state
        .auth_service
        .register(
            register(
                AppId::FINANCE_ANDROID,
                vec![Scope::new("finance:read"), Scope::new("notes:write")],
            ),
            &context(),
        )
        .await
        .unwrap();
    assert!(tokens
        .scopes
        .iter()
        .any(|scope| scope.as_str() == "finance:read"));
    assert!(!tokens
        .scopes
        .iter()
        .any(|scope| scope.as_str() == "notes:write"));

    let bearer = format!("Bearer {}", tokens.access_token);
    let principal = state
        .auth
        .authenticate(AuthCredential::Bearer(Some(&bearer)))
        .await
        .unwrap();
    assert!(principal.require_scope("finance:read").is_ok());
    assert!(principal.require_scope("notes:write").is_err());

    let response = app(state)
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sync/push")
                .header("content-type", "application/json")
                .header("authorization", bearer)
                .body(Body::from(
                    json!({
                        "requestId": "auth-scope-request",
                        "client": {
                            "appId": AppId::FINANCE_ANDROID,
                            "clientVersion": "0.2.1",
                            "platform": "android",
                            "protocolVersion": 1,
                            "schemaVersion": 1,
                            "deviceId": "external-device"
                        },
                        "changes": [{
                            "changeId": "auth-scope-change",
                            "entityType": "note.note",
                            "entityId": "auth-note",
                            "operation": "upsert",
                            "baseServerVersion": "0",
                            "entitySchemaVersion": 1,
                            "clientModifiedAt": "2026-08-05T00:00:00Z",
                            "payload": {},
                            "dependencies": []
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn password_reset_is_single_use_and_revokes_sessions() {
    let Some(state) = state().await else {
        return;
    };
    let request = register(AppId::DESKTOP, vec![]);
    let email = request.email.clone();
    let tokens = state
        .auth_service
        .register(request, &context())
        .await
        .unwrap();
    state
        .auth_service
        .forgot_password(ForgotPasswordRequestV1 { email }, &context())
        .await
        .unwrap();
    let reset = state.auth_service.development_reset_token().unwrap();
    state
        .auth_service
        .reset_password(
            ResetPasswordRequestV1 {
                token: reset.clone(),
                new_password: "另一个 secure password phrase 2026".to_owned(),
            },
            &context(),
        )
        .await
        .unwrap();
    assert!(state
        .auth_service
        .reset_password(
            ResetPasswordRequestV1 {
                token: reset,
                new_password: "第三个 secure password phrase 2026".to_owned(),
            },
            &context(),
        )
        .await
        .is_err());
    assert!(state
        .auth
        .authenticate(AuthCredential::Bearer(Some(&format!(
            "Bearer {}",
            tokens.access_token
        ))))
        .await
        .is_err());
}

#[tokio::test]
async fn web_cookie_csrf_and_origin_are_enforced() {
    let Some(state) = state().await else {
        return;
    };
    let request = register(AppId::DESKTOP, vec![]);
    let email = request.email.clone();
    let password = request.password.clone();
    state
        .auth_service
        .register(request, &context())
        .await
        .unwrap();

    let router = app(state.clone());
    let response = router
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
                        "requestedScopes": ["account:read"],
                        "publicDevice": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let csrf = payload["csrfToken"].as_str().unwrap();

    let forbidden = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/web/session/rotate")
                .header("cookie", &cookie)
                .header("x-csrf-token", csrf)
                .header("origin", "https://attacker.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let rotated = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/web/session/rotate")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("origin", "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rotated.status(), StatusCode::OK);
    let set_cookie = rotated
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(!set_cookie.contains("Domain="));
}

#[tokio::test]
async fn audit_log_contains_events_but_not_raw_secrets() {
    let Some(state) = state().await else {
        return;
    };
    let password = "audit password phrase 2026 安全";
    let mut request = register(AppId::DESKTOP, vec![]);
    request.password = password.to_owned();
    let tokens = state
        .auth_service
        .register(request, &context())
        .await
        .unwrap();
    state
        .auth_service
        .logout(
            &state
                .auth
                .authenticate(AuthCredential::Bearer(Some(&format!(
                    "Bearer {}",
                    tokens.access_token
                ))))
                .await
                .unwrap(),
            &context(),
        )
        .await
        .unwrap();
    let audit: Vec<String> = sqlx::query_scalar("SELECT metadata::text FROM auth_audit_log")
        .fetch_all(&state.pool)
        .await
        .unwrap();
    assert!(!audit.join("\n").contains(password));
    assert!(!audit.join("\n").contains("lt_at_"));
    assert!(!audit.join("\n").contains("lt_rt_"));
}
