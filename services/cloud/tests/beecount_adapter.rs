use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Method, Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use lifetrace_cloud::config::SecretString;
use lifetrace_cloud::{app, AppState, Config};
use serde_json::{json, Value};
use tower::ServiceExt;

#[derive(Clone)]
struct MockState {
    logins: Arc<AtomicUsize>,
    reject_first_ledger: Arc<AtomicBool>,
}

async fn spawn_upstream(reject_first_ledger: bool) -> (String, Arc<AtomicUsize>) {
    let logins = Arc::new(AtomicUsize::new(0));
    let state = MockState {
        logins: logins.clone(),
        reject_first_ledger: Arc::new(AtomicBool::new(reject_first_ledger)),
    };
    let upstream = Router::new()
        .route(
            "/api/v1/auth/login",
            post(|State(state): State<MockState>| async move {
                state.logins.fetch_add(1, Ordering::SeqCst);
                Json(json!({
                    "requires_2fa": false,
                    "access_token": "upstream-access",
                    "expires_in": 3600
                }))
            }),
        )
        .route(
            "/api/v1/version",
            get(|| async { Json(json!({"name": "BeeCount Cloud", "version": "test"})) }),
        )
        .route(
            "/api/v1/read/ledgers",
            get(|State(state): State<MockState>| async move {
                if state.reject_first_ledger.swap(false, Ordering::SeqCst) {
                    return (StatusCode::UNAUTHORIZED, Json(json!({"detail": "expired"})))
                        .into_response();
                }
                Json(json!([{
                    "ledger_id": "ledger-a",
                    "ledger_name": "日常账本",
                    "currency": "CNY",
                    "month_start_day": 1,
                    "transaction_count": 1,
                    "income_total": 0,
                    "expense_total": 12.34,
                    "balance": -12.34,
                    "updated_at": "2026-08-13T00:00:00Z",
                    "role": "owner",
                    "is_shared": false,
                    "member_count": 1
                }]))
                .into_response()
            }),
        )
        .route(
            "/api/v1/read/ledgers/ledger-a",
            get(|| async {
                Json(json!({
                    "ledger_id": "ledger-a",
                    "ledger_name": "日常账本",
                    "currency": "CNY",
                    "month_start_day": 1,
                    "transaction_count": 1,
                    "income_total": 0,
                    "expense_total": 12.34,
                    "balance": -12.34,
                    "source_change_id": 9,
                    "updated_at": "2026-08-13T00:00:00Z",
                    "role": "owner",
                    "is_shared": false,
                    "member_count": 1
                }))
            }),
        )
        .route(
            "/api/v1/read/ledgers/ledger-a/transactions",
            get(|| async {
                Json(json!([{
                    "id": "tx-a",
                    "tx_index": 1,
                    "tx_type": "expense",
                    "amount": 12.34,
                    "happened_at": "2026-08-13T04:05:06Z",
                    "note": "午餐",
                    "category_name": "餐饮",
                    "category_kind": "expense",
                    "account_name": "微信",
                    "from_account_name": null,
                    "to_account_name": null,
                    "category_id": "cat-a",
                    "account_id": "account-a",
                    "from_account_id": null,
                    "to_account_id": null,
                    "tags": "工作日",
                    "tags_list": ["工作日"],
                    "tag_ids": ["tag-a"],
                    "attachments": [],
                    "exclude_from_stats": false,
                    "exclude_from_budget": false,
                    "currency_code": null,
                    "native_amount": 12.34,
                    "last_change_id": 9,
                    "ledger_id": "ledger-a",
                    "ledger_name": "日常账本"
                }]))
            }),
        )
        .route(
            "/api/v1/read/workspace/accounts",
            get(|| async {
                Json(json!([{
                    "id": "account-a",
                    "name": "微信",
                    "account_type": "asset",
                    "currency": "CNY",
                    "initial_balance": 100.00,
                    "balance": 87.66,
                    "income_total": 0,
                    "expense_total": 12.34,
                    "tx_count": 1,
                    "hidden": false
                }]))
            }),
        )
        .route(
            "/api/v1/read/workspace/categories",
            get(|| async {
                Json(json!([{
                    "id": "cat-a",
                    "name": "餐饮",
                    "kind": "expense",
                    "level": 1,
                    "sort_order": 1,
                    "tx_count": 1
                }]))
            }),
        )
        .route(
            "/api/v1/read/workspace/tags",
            get(|| async {
                Json(json!([{
                    "id": "tag-a",
                    "name": "工作日",
                    "color": "#123456",
                    "tx_count": 1,
                    "income_total": 0,
                    "expense_total": 12.34
                }]))
            }),
        )
        .route(
            "/api/v1/read/ledgers/ledger-a/budgets",
            get(|| async {
                Json(json!([{
                    "id": "budget-a",
                    "type": "category",
                    "category_id": "cat-a",
                    "category_name": "餐饮",
                    "amount": 500.00,
                    "period": "monthly",
                    "start_day": 1,
                    "enabled": true
                }]))
            }),
        )
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });
    (format!("http://{address}/"), logins)
}

fn test_app(base_url: String, user_id: &str) -> Router {
    let config = Config {
        dev_auth_token: "token-a".to_owned(),
        dev_auth_user_id: user_id.to_owned(),
        beecount_adapter_enabled: true,
        beecount_adapter_base_url: base_url,
        beecount_adapter_email: Some("bridge@example.com".to_owned()),
        beecount_adapter_password: Some(SecretString::new("not-returned-to-callers")),
        beecount_adapter_lifetrace_user_id: Some("user-a".to_owned()),
        ..Config::default()
    };
    app(AppState::new(config))
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("authorization", "Bearer token-a")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn adapter_normalizes_snapshot_and_reuses_one_login() {
    let (base_url, logins) = spawn_upstream(false).await;
    let app = test_app(base_url, "user-a");

    let (status, ledgers) = get_json(app.clone(), "/api/v1/integrations/beecount/ledgers").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ledgers["readOnly"], true);
    assert_eq!(ledgers["items"][0]["id"], "beecount:ledger-a");
    assert_eq!(ledgers["items"][0]["expenseTotalCents"], 1234);

    let (status, snapshot) = get_json(
        app,
        "/api/v1/integrations/beecount/ledgers/ledger-a/snapshot?limit=50&offset=0",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(snapshot["transactions"]["items"][0]["id"], "beecount:tx-a");
    assert_eq!(snapshot["transactions"]["items"][0]["amountCents"], 1234);
    assert_eq!(snapshot["accounts"][0]["balanceCents"], 8766);
    assert_eq!(snapshot["budgets"][0]["amountCents"], 50_000);
    assert_eq!(logins.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn adapter_rejects_a_different_lifetrace_user_before_upstream_login() {
    let (base_url, logins) = spawn_upstream(false).await;
    let (status, body) = get_json(
        test_app(base_url, "user-b"),
        "/api/v1/integrations/beecount/ledgers",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "LIFETRACE_AUTH_SCOPE_DENIED");
    assert_eq!(logins.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn adapter_refreshes_once_after_an_upstream_unauthorized_response() {
    let (base_url, logins) = spawn_upstream(true).await;
    let (status, body) = get_json(
        test_app(base_url, "user-a"),
        "/api/v1/integrations/beecount/ledgers",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"][0]["sourceId"], "ledger-a");
    assert_eq!(logins.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn adapter_returns_a_sanitized_error_when_upstream_is_unavailable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    let (status, body) = get_json(
        test_app(format!("http://{address}/"), "user-a"),
        "/api/v1/integrations/beecount/ledgers",
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "LIFETRACE_TEMPORARILY_UNAVAILABLE");
    let serialized = body.to_string();
    assert!(!serialized.contains("bridge@example.com"));
    assert!(!serialized.contains("not-returned-to-callers"));
}
