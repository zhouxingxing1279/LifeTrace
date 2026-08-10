use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lifetrace_cloud::auth::security::RequestContext;
use lifetrace_cloud::auth::AuthCredential;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::auth::v1::{RegisterRequestV1, Scope};
use lifetrace_contracts::sync::v1::AppId;
use serde_json::Value;
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
        auth_password_pepper: Some("epic17-password-pepper-0123456789012345".to_owned()),
        auth_token_hash_pepper: Some("epic17-token-pepper-012345678901234567890".to_owned()),
        cursor_signing_key: Some("epic17-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("epic17-page-token-signing-key".to_owned()),
        ..Config::default()
    }
}

fn context() -> RequestContext {
    RequestContext {
        ip: Some("127.0.0.1".parse().unwrap()),
        user_agent: Some("LifeTrace EPIC-17 integration test".to_owned()),
        origin: None,
    }
}

fn register_request() -> RegisterRequestV1 {
    RegisterRequestV1 {
        email: format!("epic17-{}@example.test", Uuid::new_v4()),
        password: "EPIC 17 secure integration password 2026".to_owned(),
        display_name: Some("EPIC-17 Test".to_owned()),
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
async fn export_is_readable_secret_free_and_account_deletion_cascades() {
    let Some(url) = database_url() else {
        return;
    };
    let state = AppState::new(config(url));
    state.initialize().await.unwrap();
    sqlx::query("TRUNCATE TABLE cloud_users RESTART IDENTITY CASCADE")
        .execute(&state.pool)
        .await
        .unwrap();

    let tokens = state
        .auth_service
        .register(register_request(), &context())
        .await
        .unwrap();
    let bearer = format!("Bearer {}", tokens.access_token);
    let principal = state
        .auth
        .authenticate(AuthCredential::Bearer(Some(&bearer)))
        .await
        .unwrap();
    let user_id = Uuid::parse_str(principal.user_id.as_str()).unwrap();

    sqlx::query(
        r#"
        INSERT INTO sync_entities (
            user_id, entity_type, entity_id, entity_schema_version,
            server_version, payload, payload_hash, is_deleted, deleted_at,
            origin_device_id, created_at, server_modified_at,
            client_modified_at, last_cursor
        ) VALUES (
            $1, 'finance.transaction', 'epic17-tx', 1,
            1, '{"amountCents":1700,"note":"export-visible"}'::jsonb,
            decode('17', 'hex'), FALSE, NULL,
            NULL, now(), now(), now(), 1
        )
        "#,
    )
    .bind(user_id)
    .execute(&state.pool)
    .await
    .unwrap();

    let router = app(state.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/privacy/export?modules=account,devices,grants,sync")
                .header("authorization", &bearer)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let export: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(export["schema"], "lifetrace.user-data-export");
    assert_eq!(export["data"]["sync"]["entities"][0]["entityId"], "epic17-tx");
    assert_eq!(
        export["data"]["sync"]["entities"][0]["payload"]["note"],
        "export-visible"
    );

    // The response intentionally names excluded field classes in
    // `secretsExcluded`; verify that sensitive fields are absent from the
    // exported user-data payload itself, while actual token values are absent
    // from the entire response.
    let serialized = String::from_utf8(body.to_vec()).unwrap();
    let data_serialized = export["data"].to_string();
    for forbidden_field in [
        "password_hash",
        "passwordHash",
        "credentialCiphertext",
        "credentialNonce",
        "sessionSecret",
    ] {
        assert!(!data_serialized.contains(forbidden_field));
    }
    assert!(!serialized.contains(&tokens.access_token));
    if let Some(refresh) = tokens.refresh_token.as_deref() {
        assert!(!serialized.contains(refresh));
    }
    assert!(export["secretsExcluded"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "credentialCiphertext"));

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/v1/privacy/account")
                .header("authorization", &bearer)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"confirmation":"DELETE MY ACCOUNT"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cloud_users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let entities: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_entities WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_sessions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!((users, entities, sessions), (0, 0, 0));
    assert!(state
        .auth
        .authenticate(AuthCredential::Bearer(Some(&bearer)))
        .await
        .is_err());
}
