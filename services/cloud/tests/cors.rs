use axum::body::Body;
use axum::http::{
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS,
        ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
    },
    Method, Request,
};
use lifetrace_cloud::{app, AppState, Config};
use tower::ServiceExt;

const WEB_ORIGIN: &str = "http://127.0.0.1:4173";

fn cors_app() -> axum::Router {
    let config = Config {
        cors_allowed_origins: vec![WEB_ORIGIN.to_owned()],
        ..Config::default()
    };
    app(AppState::new(config))
}

#[tokio::test]
async fn configured_web_origin_can_read_credentialed_response() {
    let response = cors_app()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/auth/capabilities")
                .header(ORIGIN, WEB_ORIGIN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
        WEB_ORIGIN
    );
    assert_eq!(
        response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_CREDENTIALS)
            .unwrap(),
        "true"
    );
}

#[tokio::test]
async fn registration_preflight_allows_json_and_csrf_headers() {
    let response = cors_app()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/web/session/register")
                .header(ORIGIN, WEB_ORIGIN)
                .header(ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(ACCESS_CONTROL_REQUEST_HEADERS, "content-type,x-csrf-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
        WEB_ORIGIN
    );
    assert_eq!(
        response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_CREDENTIALS)
            .unwrap(),
        "true"
    );

    let methods = response
        .headers()
        .get(ACCESS_CONTROL_ALLOW_METHODS)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(methods.split(',').any(|value| value.trim() == "POST"));

    let headers = response
        .headers()
        .get(ACCESS_CONTROL_ALLOW_HEADERS)
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(headers
        .split(',')
        .any(|value| value.trim() == "content-type"));
    assert!(headers
        .split(',')
        .any(|value| value.trim() == "x-csrf-token"));
}
