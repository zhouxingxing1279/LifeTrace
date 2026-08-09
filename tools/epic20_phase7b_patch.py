from pathlib import Path


def patch_payload() -> None:
    payload = Path("crates/lifetrace-contracts/src/domain/payload.rs")
    text = payload.read_text(encoding="utf-8")
    if "RegisteredJson" in text:
        return
    text = text.replace(
        "    UserPreference(UserPreference),\n}",
        "    UserPreference(UserPreference),\n    RegisteredJson {\n        entity_type: &'static str,\n        entity_id: EntityId,\n        payload: JsonValue,\n    },\n}",
        1,
    )
    text = text.replace(
        "            EntityPayload::UserPreference(_) => EntityType::USER_PREFERENCE,\n        })",
        "            EntityPayload::UserPreference(_) => EntityType::USER_PREFERENCE,\n            EntityPayload::RegisteredJson { entity_type, .. } => *entity_type,\n        })",
        1,
    )
    text = text.replace(
        "            EntityPayload::UserPreference(value) => &value.meta.id,\n        }",
        "            EntityPayload::UserPreference(value) => &value.meta.id,\n            EntityPayload::RegisteredJson { entity_id, .. } => entity_id,\n        }",
        1,
    )
    text = text.replace(
        "            EntityPayload::UserPreference(value) => serde_json::to_value(value).unwrap().into(),\n        }",
        "            EntityPayload::UserPreference(value) => serde_json::to_value(value).unwrap().into(),\n            EntityPayload::RegisteredJson { payload, .. } => payload.clone(),\n        }",
        1,
    )
    old = '            other => Err(format!("unknown entity type: {other}")),\n'
    new = '''            other => {
                let registered_type = match other {
                    EntityType::EXECUTION_PROJECT => Some(EntityType::EXECUTION_PROJECT),
                    EntityType::EXECUTION_RECURRENCE_RULE => Some(EntityType::EXECUTION_RECURRENCE_RULE),
                    EntityType::EXECUTION_TASK => Some(EntityType::EXECUTION_TASK),
                    EntityType::EXECUTION_TASK_DEPENDENCY => Some(EntityType::EXECUTION_TASK_DEPENDENCY),
                    EntityType::EXECUTION_TASK_OCCURRENCE => Some(EntityType::EXECUTION_TASK_OCCURRENCE),
                    EntityType::EXECUTION_WAITING_ITEM => Some(EntityType::EXECUTION_WAITING_ITEM),
                    EntityType::EXECUTION_CALENDAR_EVENT => Some(EntityType::EXECUTION_CALENDAR_EVENT),
                    EntityType::EXECUTION_CALENDAR_OCCURRENCE => Some(EntityType::EXECUTION_CALENDAR_OCCURRENCE),
                    EntityType::EXECUTION_MEMO => Some(EntityType::EXECUTION_MEMO),
                    EntityType::EXECUTION_MEMO_TAG => Some(EntityType::EXECUTION_MEMO_TAG),
                    EntityType::EXECUTION_MEMO_TAG_RELATION => Some(EntityType::EXECUTION_MEMO_TAG_RELATION),
                    EntityType::EXECUTION_REMINDER => Some(EntityType::EXECUTION_REMINDER),
                    EntityType::EXECUTION_COMPLETION_RESULT => Some(EntityType::EXECUTION_COMPLETION_RESULT),
                    EntityType::EXECUTION_ENTITY_LINK => Some(EntityType::EXECUTION_ENTITY_LINK),
                    _ => None,
                };
                let Some(registered_type) = registered_type else {
                    return Err(format!("unknown entity type: {other}"));
                };
                if crate::registry::describe(registered_type).is_none() {
                    return Err(format!("unregistered entity type: {other}"));
                }
                let entity_id = value
                    .0
                    .get("meta")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|meta| meta.get("id"))
                    .and_then(serde_json::Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| format!("invalid {other} payload: meta.id is required"))?;
                Ok(EntityPayload::RegisteredJson {
                    entity_type: registered_type,
                    entity_id: EntityId::new(entity_id),
                    payload: value,
                })
            }
'''
    if old not in text:
        raise SystemExit("EntityPayload fallback anchor not found")
    text = text.replace(old, new, 1)
    text += r'''

#[cfg(test)]
mod execution_registered_json_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registered_execution_payload_preserves_json_and_id() {
        let entity_type = EntityType::new(EntityType::EXECUTION_TASK);
        let raw = JsonValue(json!({
            "meta": {"id": "task-1", "userId": "local"},
            "title": "Ship EPIC20",
            "status": "todo"
        }));
        let payload = EntityPayload::try_from((&entity_type, raw.clone())).unwrap();
        assert_eq!(payload.entity_type().as_str(), EntityType::EXECUTION_TASK);
        assert_eq!(payload.entity_id().as_str(), "task-1");
        assert_eq!(payload.to_json(), raw);
    }

    #[test]
    fn execution_payload_requires_meta_id_and_unknown_type_stays_rejected() {
        let entity_type = EntityType::new(EntityType::EXECUTION_MEMO);
        let missing_id = JsonValue(json!({"meta": {}, "content": "memo"}));
        assert!(EntityPayload::try_from((&entity_type, missing_id)).is_err());
        let unknown = EntityType::new("future.secret");
        let raw = JsonValue(json!({"meta": {"id": "x"}}));
        assert!(EntityPayload::try_from((&unknown, raw)).is_err());
    }
}
'''
    payload.write_text(text, encoding="utf-8")


def patch_cloud_api() -> None:
    api = Path("services/cloud/tests/api.rs")
    text = api.read_text(encoding="utf-8")
    if "fn execution_client(device_id: &str) -> Value" in text:
        return
    text += r'''

fn execution_client(device_id: &str) -> Value {
    json!({
        "appId": "lifetrace-desktop",
        "clientVersion": "0.2.1",
        "platform": "windows",
        "protocolVersion": 1,
        "schemaVersion": 1,
        "deviceId": device_id
    })
}

fn execution_payload(entity_id: &str, user_id: &str, fields: Value) -> Value {
    let mut object = fields.as_object().cloned().expect("execution fields object");
    object.insert(
        "meta".to_owned(),
        json!({
            "id": entity_id,
            "userId": user_id,
            "createdAt": "2026-08-09T00:00:00Z",
            "updatedAt": "2026-08-09T00:00:00Z",
            "deletedAt": null,
            "localVersion": 1,
            "serverVersion": null,
            "modifiedByDevice": null
        }),
    );
    Value::Object(object)
}

fn execution_change(
    user_id: &str,
    change_id: &str,
    entity_type: &str,
    entity_id: &str,
    base: u64,
    operation: &str,
    fields: Value,
) -> Value {
    json!({
        "changeId": change_id,
        "entityType": entity_type,
        "entityId": entity_id,
        "operation": operation,
        "baseServerVersion": base.to_string(),
        "entitySchemaVersion": 1,
        "clientModifiedAt": "2026-08-09T00:00:00Z",
        "payload": if operation == "upsert" {
            execution_payload(entity_id, user_id, fields)
        } else {
            Value::Null
        },
        "atomicGroupId": null,
        "dependencies": []
    })
}

fn execution_push_request(device_id: &str, changes: Vec<Value>) -> Value {
    json!({
        "requestId": format!("req-{device_id}"),
        "client": execution_client(device_id),
        "changes": changes
    })
}

async fn execution_pull(
    app: Router,
    token: &str,
    device_id: &str,
    after_cursor: Option<&str>,
) -> (StatusCode, Value) {
    send(
        app,
        Method::POST,
        "/api/v1/sync/pull",
        token,
        json!({
            "requestId": format!("pull-{device_id}"),
            "client": execution_client(device_id),
            "afterCursor": after_cursor,
            "limit": 100,
            "entityTypes": null
        }),
    )
    .await
}

#[tokio::test]
async fn execution_two_devices_create_update_delete_and_tombstone() {
    let user = "execution-e2e-user-core";
    let app_a = test_app_for(TOKEN_A, user, "execution-device-a");
    let app_b = test_app_for(TOKEN_B, user, "execution-device-b");
    let initial = vec![
        execution_change(user, "exec-core-task-c1", "execution.task", "task-core", 0, "upsert", json!({"title":"Offline task","status":"todo","priority":"normal"})),
        execution_change(user, "exec-core-calendar-c1", "execution.calendar_event", "event-core", 0, "upsert", json!({"title":"Focus block","isAllDay":false,"startAt":"2026-08-09T02:00:00Z","endAt":"2026-08-09T03:00:00Z","status":"scheduled"})),
        execution_change(user, "exec-core-waiting-c1", "execution.waiting_item", "waiting-core", 0, "upsert", json!({"title":"Wait for reply","status":"open","waitingFor":"Alice"})),
        execution_change(user, "exec-core-memo-c1", "execution.memo", "memo-core", 0, "upsert", json!({"content":"Remember sync","plainText":"Remember sync","isPinned":false,"status":"active"})),
        execution_change(user, "exec-core-reminder-c1", "execution.reminder", "reminder-core", 0, "upsert", json!({"subjectType":"task","subjectId":"task-core","triggerAt":"2026-08-10T00:00:00Z","status":"scheduled","fireKey":"task-core@2026-08-10"})),
    ];
    let (status, push) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-device-a", initial)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(push["results"].as_array().unwrap().iter().all(|result| result["status"] == "accepted"));

    let (status, first_pull) = execution_pull(app_b.clone(), TOKEN_B, "execution-device-b", None).await;
    assert_eq!(status, StatusCode::OK);
    let changes = first_pull["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 5);
    for expected in ["execution.task", "execution.calendar_event", "execution.waiting_item", "execution.memo", "execution.reminder"] {
        assert!(changes.iter().any(|change| change["entityType"] == expected));
    }
    let cursor_after_create = first_pull["nextCursor"].as_str().unwrap().to_owned();

    let (status, update) = send(
        app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A,
        execution_push_request("execution-device-a", vec![execution_change(user, "exec-core-task-c2", "execution.task", "task-core", 1, "upsert", json!({"title":"Task updated on A","status":"in_progress","priority":"high"}))]),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(update["results"][0]["status"], "accepted");
    assert_eq!(update["results"][0]["serverVersion"], "2");

    let (status, update_pull) = execution_pull(app_b.clone(), TOKEN_B, "execution-device-b", Some(&cursor_after_create)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(update_pull["changes"].as_array().unwrap().len(), 1);
    assert_eq!(update_pull["changes"][0]["entityType"], "execution.task");
    assert_eq!(update_pull["changes"][0]["payload"]["title"], "Task updated on A");
    let cursor_after_update = update_pull["nextCursor"].as_str().unwrap().to_owned();

    let (status, delete) = send(
        app_a, Method::POST, "/api/v1/sync/push", TOKEN_A,
        execution_push_request("execution-device-a", vec![execution_change(user, "exec-core-memo-c2", "execution.memo", "memo-core", 1, "delete", Value::Null)]),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(delete["results"][0]["status"], "accepted");
    let (status, delete_pull) = execution_pull(app_b, TOKEN_B, "execution-device-b", Some(&cursor_after_update)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(delete_pull["changes"].as_array().unwrap().len(), 1);
    assert_eq!(delete_pull["changes"][0]["entityType"], "execution.memo");
    assert_eq!(delete_pull["changes"][0]["operation"], "delete");
    assert_eq!(delete_pull["changes"][0]["tombstone"]["serverVersion"], "2");
}

#[tokio::test]
async fn execution_duplicate_and_domain_conflicts_are_not_silent() {
    let user = "execution-e2e-user-conflicts";
    let app_a = test_app_for(TOKEN_A, user, "execution-conflict-a");
    let app_b = test_app_for(TOKEN_B, user, "execution-conflict-b");

    let memo_create = execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-conflict-memo-c1", "execution.memo", "memo-conflict", 0, "upsert", json!({"content":"Base memo","plainText":"Base memo","isPinned":false,"status":"active"}))]);
    let (_, first) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, memo_create.clone()).await;
    assert_eq!(first["results"][0]["status"], "accepted");
    let (_, duplicate) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, memo_create).await;
    assert_eq!(duplicate["results"][0]["status"], "duplicate");
    assert_eq!(duplicate["results"][0]["serverVersion"], "1");

    let (_, archive) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-conflict-memo-c2", "execution.memo", "memo-conflict", 1, "upsert", json!({"content":"Base memo","plainText":"Base memo","isPinned":false,"status":"archived","archivedAt":"2026-08-09T01:00:00Z"}))])).await;
    assert_eq!(archive["results"][0]["status"], "accepted");
    let (_, stale_edit) = send(app_b.clone(), Method::POST, "/api/v1/sync/push", TOKEN_B, execution_push_request("execution-conflict-b", vec![execution_change(user, "exec-conflict-memo-c3", "execution.memo", "memo-conflict", 1, "upsert", json!({"content":"Edited on B","plainText":"Edited on B","isPinned":false,"status":"active"}))])).await;
    assert_eq!(stale_edit["results"][0]["status"], "conflict");
    assert_eq!(stale_edit["results"][0]["reason"], "base_version_mismatch");
    assert_eq!(stale_edit["results"][0]["serverEntity"]["status"], "archived");

    let (_, memo_text_create) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-text-c1", "execution.memo", "memo-text", 0, "upsert", json!({"content":"Start","plainText":"Start","isPinned":false,"status":"active"}))])).await;
    assert_eq!(memo_text_create["results"][0]["status"], "accepted");
    let (_, memo_a) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-text-c2", "execution.memo", "memo-text", 1, "upsert", json!({"content":"A text","plainText":"A text","isPinned":false,"status":"active"}))])).await;
    assert_eq!(memo_a["results"][0]["status"], "accepted");
    let (_, memo_b) = send(app_b.clone(), Method::POST, "/api/v1/sync/push", TOKEN_B, execution_push_request("execution-conflict-b", vec![execution_change(user, "exec-text-c3", "execution.memo", "memo-text", 1, "upsert", json!({"content":"B text","plainText":"B text","isPinned":false,"status":"active"}))])).await;
    assert_eq!(memo_b["results"][0]["status"], "conflict");
    assert_eq!(memo_b["results"][0]["serverEntity"]["content"], "A text");

    let (_, task_create) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-task-conflict-c1", "execution.task", "task-conflict", 0, "upsert", json!({"title":"Competing state","status":"todo","priority":"normal"}))])).await;
    assert_eq!(task_create["results"][0]["status"], "accepted");
    let (_, completed) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-task-conflict-c2", "execution.task", "task-conflict", 1, "upsert", json!({"title":"Competing state","status":"done","priority":"normal","completedAt":"2026-08-09T01:00:00Z"}))])).await;
    assert_eq!(completed["results"][0]["status"], "accepted");
    let (_, cancelled) = send(app_b.clone(), Method::POST, "/api/v1/sync/push", TOKEN_B, execution_push_request("execution-conflict-b", vec![execution_change(user, "exec-task-conflict-c3", "execution.task", "task-conflict", 1, "upsert", json!({"title":"Competing state","status":"cancelled","priority":"normal","cancelledAt":"2026-08-09T01:00:00Z"}))])).await;
    assert_eq!(cancelled["results"][0]["status"], "conflict");
    assert_eq!(cancelled["results"][0]["serverEntity"]["status"], "done");

    let (_, recurrence_create) = send(app_a.clone(), Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-recur-c1", "execution.recurrence_rule", "rule-conflict", 0, "upsert", json!({"frequency":"daily","intervalValue":1}))])).await;
    assert_eq!(recurrence_create["results"][0]["status"], "accepted");
    let (_, recurrence_a) = send(app_a, Method::POST, "/api/v1/sync/push", TOKEN_A, execution_push_request("execution-conflict-a", vec![execution_change(user, "exec-recur-c2", "execution.recurrence_rule", "rule-conflict", 1, "upsert", json!({"frequency":"daily","intervalValue":2}))])).await;
    assert_eq!(recurrence_a["results"][0]["status"], "accepted");
    let (_, recurrence_b) = send(app_b, Method::POST, "/api/v1/sync/push", TOKEN_B, execution_push_request("execution-conflict-b", vec![execution_change(user, "exec-recur-c3", "execution.recurrence_rule", "rule-conflict", 1, "upsert", json!({"frequency":"weekly","intervalValue":1,"weekdaysJson":"[1,3,5]"}))])).await;
    assert_eq!(recurrence_b["results"][0]["status"], "conflict");
    assert_eq!(recurrence_b["results"][0]["serverEntity"]["intervalValue"], 2);
}
'''
    api.write_text(text, encoding="utf-8")


def patch_desktop_execution_tests() -> None:
    execution = Path("apps/desktop/src-tauri/src/sync/execution.rs")
    text = execution.read_text(encoding="utf-8")
    if "fn offline_core_entities_sync_into_second_device_real_tables()" in text:
        return
    insert = r'''

    #[test]
    fn offline_core_entities_sync_into_second_device_real_tables() {
        let (source, source_profile) = db("offline-source");
        let (target, target_profile) = db("offline-target");
        let stamp = "2026-08-09T00:00:00Z";

        source.execute("INSERT INTO execution_tasks(id,user_id,title,status,priority,created_at,updated_at) VALUES('task-sync',?1,'Offline Task','todo','normal',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_calendar_events(id,user_id,title,is_all_day,start_at,end_at,status,created_at,updated_at) VALUES('event-sync',?1,'Focus',0,'2026-08-09T02:00:00Z','2026-08-09T03:00:00Z','scheduled',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_waiting_items(id,user_id,title,status,waiting_for,source_task_id,created_at,updated_at) VALUES('waiting-sync',?1,'Waiting','open','Alice','task-sync',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_memos(id,user_id,content,plain_text,is_pinned,status,created_at,updated_at) VALUES('memo-sync',?1,'Remember','Remember',0,'active',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_reminders(id,user_id,subject_type,subject_id,trigger_at,status,fire_key,created_at,updated_at) VALUES('reminder-sync',?1,'task','task-sync','2026-08-10T00:00:00Z','scheduled','task-sync@2026-08-10',?2,?2)", params![source_profile, stamp]).unwrap();

        let queued: i64 = source.query_row("SELECT COUNT(*) FROM sync_outbox WHERE profile_id=?1 AND entity_type LIKE 'execution.%' AND status='pending'", [&source_profile], |row| row.get(0)).unwrap();
        assert_eq!(queued, 5, "offline writes must be captured before reconnect");

        target.execute("UPDATE sync_context SET origin='remote' WHERE singleton=1", []).unwrap();
        for (entity_type, entity_id) in [
            ("execution.task", "task-sync"),
            ("execution.calendar_event", "event-sync"),
            ("execution.waiting_item", "waiting-sync"),
            ("execution.memo", "memo-sync"),
            ("execution.reminder", "reminder-sync"),
        ] {
            let local = load_local_entity(&source, &source_profile, entity_type, entity_id).unwrap().unwrap();
            let wire = crate::sync::payload::legacy_to_wire(entity_type, &local, &source_profile, Some("1")).unwrap();
            let legacy = crate::sync::payload::wire_to_legacy(&wire).unwrap();
            apply_upsert(&target, &target_profile, entity_type, &legacy).unwrap();
        }
        let target_outbox: i64 = target.query_row("SELECT COUNT(*) FROM sync_outbox WHERE entity_type LIKE 'execution.%'", [], |row| row.get(0)).unwrap();
        assert_eq!(target_outbox, 0, "remote pull must not echo into outbox");
        let task_status: String = target.query_row("SELECT status FROM execution_tasks WHERE id='task-sync' AND user_id=?1", [&target_profile], |row| row.get(0)).unwrap();
        assert_eq!(task_status, "todo");
        let event_title: String = target.query_row("SELECT title FROM execution_calendar_events WHERE id='event-sync' AND user_id=?1", [&target_profile], |row| row.get(0)).unwrap();
        assert_eq!(event_title, "Focus");
        let waiting_for: String = target.query_row("SELECT waiting_for FROM execution_waiting_items WHERE id='waiting-sync' AND user_id=?1", [&target_profile], |row| row.get(0)).unwrap();
        assert_eq!(waiting_for, "Alice");
        let memo: String = target.query_row("SELECT content FROM execution_memos WHERE id='memo-sync' AND user_id=?1", [&target_profile], |row| row.get(0)).unwrap();
        assert_eq!(memo, "Remember");
        let reminder_status: String = target.query_row("SELECT status FROM execution_reminders WHERE id='reminder-sync' AND user_id=?1", [&target_profile], |row| row.get(0)).unwrap();
        assert_eq!(reminder_status, "scheduled");

        source.execute("UPDATE execution_tasks SET status='done',completed_at='2026-08-09T01:00:00Z',updated_at='2026-08-09T01:00:00Z',version=version+1 WHERE id='task-sync'", []).unwrap();
        let task_local = load_local_entity(&source, &source_profile, "execution.task", "task-sync").unwrap().unwrap();
        let task_wire = crate::sync::payload::legacy_to_wire("execution.task", &task_local, &source_profile, Some("2")).unwrap();
        let task_legacy = crate::sync::payload::wire_to_legacy(&task_wire).unwrap();
        apply_upsert(&target, &target_profile, "execution.task", &task_legacy).unwrap();
        let task_status: String = target.query_row("SELECT status FROM execution_tasks WHERE id='task-sync'", [], |row| row.get(0)).unwrap();
        assert_eq!(task_status, "done");

        source.execute("UPDATE execution_memos SET deleted_at='2026-08-09T02:00:00Z',updated_at='2026-08-09T02:00:00Z',version=version+1 WHERE id='memo-sync'", []).unwrap();
        let memo_operation: String = source.query_row("SELECT operation FROM sync_outbox WHERE profile_id=?1 AND entity_type='execution.memo' AND entity_id='memo-sync' AND status='pending'", [&source_profile], |row| row.get(0)).unwrap();
        assert_eq!(memo_operation, "delete");
        apply_delete(&target, &target_profile, "execution.memo", "memo-sync").unwrap();
        let deleted: Option<String> = target.query_row("SELECT deleted_at FROM execution_memos WHERE id='memo-sync'", [], |row| row.get(0)).unwrap();
        assert!(deleted.is_some());
        target.execute("UPDATE sync_context SET origin='local' WHERE singleton=1", []).unwrap();
    }
'''
    pos = text.rfind("\n}")
    if pos < 0:
        raise SystemExit("execution test module closing brace not found")
    execution.write_text(text[:pos] + insert + text[pos:], encoding="utf-8")


patch_payload()
patch_cloud_api()
patch_desktop_execution_tests()
