use chrono::Utc;
use lifetrace_cloud::{AppState, Config};
use lifetrace_contracts::sync::v1::{
    AppId, ChangeOperation, ClientPlatform, PushChangeResultV1, PushRequestV1, SyncChangeV1,
    SyncClientInfo,
};
use lifetrace_contracts::{
    AtomicGroupId, ChangeId, DeviceId, EntityId, EntityType, ErrorCode, JsonValue, RequestId,
    ServerVersion, UserId,
};
use serde_json::{json, Value};

fn database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn client() -> SyncClientInfo {
    SyncClientInfo {
        app_id: AppId::new(AppId::DESKTOP),
        client_version: "0.2.1".to_owned(),
        platform: ClientPlatform::new(ClientPlatform::WINDOWS),
        protocol_version: 1,
        schema_version: 1,
        device_id: DeviceId::new("execution-p4-device"),
    }
}

fn meta(id: &str) -> Value {
    json!({
        "id": id,
        "userId": "execution-p4-user",
        "createdAt": "2026-08-14T00:00:00Z",
        "updatedAt": "2026-08-14T00:00:00Z",
        "deletedAt": null,
        "localVersion": 1,
        "serverVersion": null,
        "modifiedByDevice": "execution-p4-device"
    })
}

fn task_payload(id: &str, title: &str, status: &str) -> JsonValue {
    JsonValue(json!({
        "meta": meta(id),
        "projectId": null,
        "parentTaskId": null,
        "title": title,
        "description": null,
        "status": status,
        "priority": "normal",
        "dueAt": null,
        "scheduledStartAt": null,
        "scheduledEndAt": null,
        "timezone": "UTC",
        "estimatedMinutes": null,
        "context": null,
        "recurrenceRuleId": null,
        "sortOrder": 0,
        "completedAt": if status == "done" { Some("2026-08-14T01:00:00Z") } else { None::<&str> }
    }))
}

fn dependency_payload(id: &str, task_id: &str, predecessor_id: &str) -> JsonValue {
    JsonValue(json!({
        "meta": meta(id),
        "taskId": task_id,
        "dependsOnTaskId": predecessor_id,
        "dependencyType": "finish_before_start"
    }))
}

fn change(
    change_id: &str,
    entity_type: &str,
    entity_id: &str,
    base: u64,
    payload: JsonValue,
    atomic_group_id: Option<&str>,
) -> SyncChangeV1 {
    SyncChangeV1 {
        change_id: ChangeId::new(change_id),
        entity_type: EntityType::new(entity_type),
        entity_id: EntityId::new(entity_id),
        operation: ChangeOperation::new(ChangeOperation::UPSERT),
        base_server_version: ServerVersion::from_u64(base),
        entity_schema_version: 1,
        client_modified_at: Utc::now(),
        payload: Some(payload),
        atomic_group_id: atomic_group_id.map(AtomicGroupId::new),
        dependencies: vec![],
    }
}

fn request(id: &str, changes: Vec<SyncChangeV1>) -> PushRequestV1 {
    PushRequestV1 {
        request_id: RequestId::new(id),
        client: client(),
        changes,
    }
}

#[tokio::test]
async fn finish_before_start_is_server_authoritative_and_atomic_group_aware() {
    let Some(url) = database_url() else {
        eprintln!("TEST_DATABASE_URL not set; execution P4 PostgreSQL test skipped");
        return;
    };
    let config = Config {
        database_url: Some(url),
        migration_on_startup: true,
        dev_auth_token: "execution-p4-token".to_owned(),
        dev_auth_user_id: "execution-p4-user".to_owned(),
        dev_auth_device_id: "execution-p4-device".to_owned(),
        cursor_signing_key: Some("execution-p4-cursor-key".to_owned()),
        page_token_signing_key: Some("execution-p4-page-key".to_owned()),
        ..Config::default()
    };
    let state = AppState::new(config);
    state.initialize().await.unwrap();
    sqlx::query(
        "TRUNCATE TABLE sync_snapshot_items, sync_snapshots, sync_processed_changes, sync_change_log, sync_entities, cloud_devices, cloud_users RESTART IDENTITY CASCADE",
    )
    .execute(&state.pool)
    .await
    .unwrap();

    let user = UserId::new("execution-p4-user");
    let seed = request(
        "execution-p4-seed",
        vec![
            change(
                "seed-pre",
                "execution.task",
                "pre",
                0,
                task_payload("pre", "前置", "todo"),
                None,
            ),
            change(
                "seed-next",
                "execution.task",
                "next",
                0,
                task_payload("next", "后继", "todo"),
                None,
            ),
            change(
                "seed-dep",
                "execution.task_dependency",
                "dep",
                0,
                dependency_payload("dep", "next", "pre"),
                None,
            ),
        ],
    );
    let seeded = state.store.push(&user, &seed).await.unwrap();
    assert!(seeded
        .results
        .iter()
        .all(|result| matches!(result, PushChangeResultV1::Accepted { .. })));

    let blocked = state
        .store
        .push(
            &user,
            &request(
                "execution-p4-blocked",
                vec![change(
                    "blocked-next",
                    "execution.task",
                    "next",
                    1,
                    task_payload("next", "后继", "in_progress"),
                    None,
                )],
            ),
        )
        .await
        .unwrap();
    assert!(matches!(
        &blocked.results[0],
        PushChangeResultV1::Rejected { code, .. } if code == &ErrorCode::DependencyMissing
    ));

    let atomic = state
        .store
        .push(
            &user,
            &request(
                "execution-p4-atomic",
                vec![
                    change(
                        "atomic-pre",
                        "execution.task",
                        "pre",
                        1,
                        task_payload("pre", "前置", "done"),
                        Some("finish-and-start"),
                    ),
                    change(
                        "atomic-next",
                        "execution.task",
                        "next",
                        1,
                        task_payload("next", "后继", "in_progress"),
                        Some("finish-and-start"),
                    ),
                ],
            ),
        )
        .await
        .unwrap();
    assert!(atomic
        .results
        .iter()
        .all(|result| matches!(result, PushChangeResultV1::Accepted { .. })));

    let next = state
        .store
        .list_entities(&user, "execution.task")
        .await
        .unwrap()
        .into_iter()
        .find(|entity| entity.entity_id.as_str() == "next")
        .unwrap();
    assert_eq!(next.payload.0["status"], "in_progress");
}
