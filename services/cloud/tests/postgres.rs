//! PostgreSQL integration coverage for EPIC-03 runtime persistence.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chrono::Utc;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::domain::finance::Transaction;
use lifetrace_contracts::domain::{TransactionStatus, TransactionType};
use lifetrace_contracts::sync::v1::{
    AppId, ChangeOperation, ClientPlatform, PullRequestV1, PushChangeResultV1, PushRequestV1,
    SnapshotRequestV1, SyncChangeV1, SyncClientInfo,
};
use lifetrace_contracts::time::{LocalDate, UtcTimestamp};
use lifetrace_contracts::{
    ChangeId, CurrencyCode, EntityId, EntityMeta, EntityType, RequestId, ServerVersion, UserId,
};
use serde_json::Value;
use tower::ServiceExt;

fn database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn stamp() -> UtcTimestamp {
    "2026-08-05T03:00:00Z".parse().unwrap()
}

fn client() -> SyncClientInfo {
    SyncClientInfo {
        app_id: AppId::new(AppId::DESKTOP),
        client_version: "0.2.1".to_owned(),
        platform: ClientPlatform::new(ClientPlatform::WINDOWS),
        protocol_version: 1,
        schema_version: 1,
        device_id: lifetrace_contracts::DeviceId::new("postgres-device"),
    }
}

fn transaction_payload(id: &str, amount_cents: i64) -> lifetrace_contracts::JsonValue {
    let transaction = Transaction {
        meta: EntityMeta {
            id: EntityId::new(id),
            user_id: UserId::new("local-user"),
            created_at: stamp(),
            updated_at: stamp(),
            deleted_at: None,
            local_version: 1,
            server_version: None,
            modified_by_device: None,
        },
        transaction_type: TransactionType::new(TransactionType::EXPENSE),
        amount_cents,
        currency: CurrencyCode::cny(),
        account_id: None,
        to_account_id: None,
        category_id: None,
        counterparty: None,
        merchant: None,
        item: None,
        note: None,
        occurred_at: stamp(),
        local_date: LocalDate::new("2026-08-05").unwrap(),
        status: TransactionStatus::new(TransactionStatus::CONFIRMED),
        source_type: "manual".to_owned(),
        external_transaction_id: None,
    };
    lifetrace_contracts::JsonValue(serde_json::to_value(transaction).unwrap())
}

fn push_request() -> PushRequestV1 {
    PushRequestV1 {
        request_id: RequestId::new("postgres-request-1"),
        client: client(),
        changes: vec![SyncChangeV1 {
            change_id: ChangeId::new("postgres-change-1"),
            entity_type: EntityType::new("finance.transaction"),
            entity_id: EntityId::new("postgres-entity-1"),
            operation: ChangeOperation::new(ChangeOperation::UPSERT),
            base_server_version: ServerVersion::zero(),
            entity_schema_version: 1,
            client_modified_at: Utc::now(),
            payload: Some(transaction_payload("postgres-entity-1", 1234)),
            atomic_group_id: None,
            dependencies: vec![],
        }],
    }
}

#[tokio::test]
async fn postgres_runtime_migrates_persists_and_replays_idempotently() {
    let Some(url) = database_url() else {
        eprintln!("TEST_DATABASE_URL not set; PostgreSQL integration test skipped");
        return;
    };

    let config = Config {
        database_url: Some(url),
        migration_on_startup: true,
        dev_auth_token: "postgres-token".to_owned(),
        dev_auth_user_id: "postgres-user".to_owned(),
        dev_auth_device_id: "postgres-device".to_owned(),
        cursor_signing_key: Some("postgres-cursor-key".to_owned()),
        page_token_signing_key: Some("postgres-page-key".to_owned()),
        ..Config::default()
    };

    let state = AppState::new(config.clone());
    state.initialize().await.unwrap();
    sqlx::query(
        "TRUNCATE TABLE sync_snapshot_items, sync_snapshots, sync_processed_changes, \
         sync_change_log, sync_entities, cloud_devices, cloud_users RESTART IDENTITY CASCADE",
    )
    .execute(&state.pool)
    .await
    .unwrap();

    let user = UserId::new("postgres-user");
    let request = push_request();
    let first = state.store.push(&user, &request).await.unwrap();
    assert!(matches!(
        first.results[0],
        PushChangeResultV1::Accepted { .. }
    ));
    assert_eq!(state.store.change_count(&user).await.unwrap(), 1);

    let migrated: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM _sqlx_migrations")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert!(migrated >= 7);

    let ready_response = app(state.clone())
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ready_response.status(), StatusCode::OK);
    let ready_body = to_bytes(ready_response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let ready_json: Value = serde_json::from_slice(&ready_body).unwrap();
    assert_eq!(ready_json["checks"]["storage"], "postgresql");
    assert_eq!(ready_json["checks"]["postgresql"], true);

    drop(state);

    let restarted = AppState::new(config);
    restarted.initialize().await.unwrap();
    let items = restarted
        .store
        .list_entities(&user, "finance.transaction")
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].entity_id.as_str(), "postgres-entity-1");

    let duplicate = restarted.store.push(&user, &request).await.unwrap();
    assert!(matches!(
        duplicate.results[0],
        PushChangeResultV1::Duplicate { .. }
    ));
    assert_eq!(restarted.store.change_count(&user).await.unwrap(), 1);

    let pulled = restarted
        .store
        .pull(
            &user,
            &PullRequestV1 {
                request_id: RequestId::new("postgres-pull-1"),
                client: client(),
                after_cursor: None,
                limit: 50,
                entity_types: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(pulled.changes.len(), 1);
    assert_eq!(pulled.changes[0].entity_id.as_str(), "postgres-entity-1");

    let snapshot = restarted
        .store
        .snapshot(
            &user,
            &SnapshotRequestV1 {
                request_id: RequestId::new("postgres-snapshot-1"),
                client: client(),
                snapshot_id: None,
                page_token: None,
                entity_types: None,
                page_size: 20,
            },
        )
        .await
        .unwrap();
    assert!(snapshot.completed);
    assert_eq!(snapshot.items.len(), 1);

    let filtered_types = vec![EntityType::new("finance.transaction")];
    let filtered_snapshot = restarted
        .store
        .snapshot(
            &user,
            &SnapshotRequestV1 {
                request_id: RequestId::new("postgres-filtered-snapshot"),
                client: client(),
                snapshot_id: None,
                page_token: None,
                entity_types: Some(filtered_types.clone()),
                page_size: 20,
            },
        )
        .await
        .unwrap();
    assert!(filtered_snapshot.completed);
    assert_eq!(filtered_snapshot.items.len(), 1);

    let after_filtered_snapshot = restarted
        .store
        .pull(
            &user,
            &PullRequestV1 {
                request_id: RequestId::new("postgres-filtered-pull"),
                client: client(),
                after_cursor: Some(filtered_snapshot.snapshot_cursor),
                limit: 50,
                entity_types: Some(filtered_types),
            },
        )
        .await
        .unwrap();
    assert!(after_filtered_snapshot.changes.is_empty());
}

#[tokio::test]
async fn readiness_fails_when_postgres_is_unavailable() {
    let config = Config {
        database_url: Some(
            "postgres://lifetrace:invalid@127.0.0.1:1/lifetrace_unavailable".to_owned(),
        ),
        ..Config::default()
    };
    let state = AppState::new(config);
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
