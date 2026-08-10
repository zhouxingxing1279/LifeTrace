use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lifetrace_cloud::{app, AppState, Config};
use serde_json::Value;
use tower::ServiceExt;

const TOKEN: &str = "epic17-test-token";

fn test_app() -> axum::Router {
    app(AppState::new(Config {
        dev_auth_token: TOKEN.to_owned(),
        dev_auth_user_id: "00000000-0000-0000-0000-000000000017".to_owned(),
        dev_auth_device_id: "epic17-device".to_owned(),
        ..Config::default()
    }))
}

#[tokio::test]
async fn security_headers_are_present_on_api_responses() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["x-frame-options"], "DENY");
    assert_eq!(headers["referrer-policy"], "no-referrer");
    assert_eq!(headers["cache-control"], "no-store");
    assert!(headers["content-security-policy"]
        .to_str()
        .unwrap()
        .contains("default-src 'none'"));
    assert!(headers["strict-transport-security"]
        .to_str()
        .unwrap()
        .contains("max-age=31536000"));
}

#[tokio::test]
async fn privacy_endpoints_reject_anonymous_access() {
    for (method, uri, body) in [
        (Method::GET, "/api/v1/privacy/policy", Body::empty()),
        (Method::GET, "/api/v1/privacy/export", Body::empty()),
        (
            Method::DELETE,
            "/api/v1/privacy/account",
            Body::from(r#"{"confirmation":"DELETE MY ACCOUNT"}"#),
        ),
    ] {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::FORBIDDEN,
            "{uri} unexpectedly returned {}",
            response.status()
        );
    }
}

#[tokio::test]
async fn policy_is_authenticated_and_export_fails_closed_without_cloud_database() {
    let policy = test_app()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/privacy/policy")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(policy.status(), StatusCode::OK);
    let bytes = to_bytes(policy.into_body(), 128 * 1024).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["version"], 1);
    assert!(body["diagnosticLogs"]["content"]
        .as_str()
        .unwrap()
        .contains("tokens"));

    let export = test_app()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/privacy/export")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export.status(), StatusCode::SERVICE_UNAVAILABLE);
}
