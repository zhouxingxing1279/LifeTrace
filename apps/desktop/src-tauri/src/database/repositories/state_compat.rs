//! `/api/state` 兼容层。
//!
//! 读取：数据库真实列 → Rust Model → 旧 camelCase DTO。
//! 写入：旧 DTO → Validation → Repository → 新表。
//!
//! 阶段 1 尚无规范化业务表，此模块为空壳；财务（阶段 2）与习惯/复盘
//! （阶段 3）落地后在此实现转换，禁止把整个 DTO 写回 `data_json`。

#[cfg(test)]
mod tests {
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use crate::database::repositories::{finance, habits, notes, workouts};
    use rusqlite::Connection;
    use serde_json::{json, Value};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn migrated_connection() -> (Connection, std::path::PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-compat-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        let context = MigrationContext::new(directory.clone());
        run(&mut connection, &context, &all()).unwrap();
        (connection, directory)
    }

    #[test]
    fn state_payload_shape_matches_legacy_dto() {
        let (connection, directory) = migrated_connection();
        let stamp = "2026-07-01T00:00:00Z";
        finance::save_account(
            &connection,
            &json!({
                "id": "a1", "userId": "local-user", "name": "现金", "type": "cash",
                "balance": 100, "balanceAt": stamp, "color": "#fff", "icon": "cash",
                "isArchived": false, "createdAt": stamp, "updatedAt": stamp
            }),
        )
        .unwrap();
        finance::save_transaction(
            &connection,
            &json!({
                "id": "t1", "userId": "local-user", "type": "expense", "amount": 12.34,
                "category": "餐饮", "account": "现金", "accountId": "a1",
                "occurredAt": stamp, "createdAt": stamp, "updatedAt": stamp
            }),
        )
        .unwrap();
        habits::save_activity(
            &connection,
            &json!({
                "id": "h1", "userId": "local-user", "name": "阅读", "type": "count",
                "unit": "次", "targetPeriod": "daily", "isArchived": false,
                "createdAt": stamp, "updatedAt": stamp
            }),
        )
        .unwrap();

        let accounts = finance::list_accounts(&connection).unwrap();
        let transactions = finance::list_transactions(&connection).unwrap();
        let activities = habits::list_activities(&connection).unwrap();
        let logs = habits::list_activity_logs(&connection).unwrap();
        let reviews = habits::list_daily_reviews(&connection).unwrap();
        let workout_history = workouts::list_workouts(&connection).unwrap();

        assert_eq!(accounts.len(), 1);
        assert!(accounts[0].get("balance").is_some());
        assert_eq!(transactions[0]["amount"], json!(12.34));
        assert_eq!(transactions[0]["category"], json!("餐饮"));
        assert_eq!(transactions[0]["account"], json!("现金"));
        assert_eq!(activities[0]["name"], json!("阅读"));
        assert_eq!(logs.len(), 0);
        assert_eq!(reviews.len(), 0);
        assert_eq!(workout_history.len(), 0);
        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn json_backup_roundtrip_restores_normalized_tables() {
        let (connection, directory) = migrated_connection();
        let stamp = "2026-07-01T00:00:00Z";
        finance::save_account(
            &connection,
            &json!({
                "id": "a1", "userId": "local-user", "name": "现金", "type": "cash",
                "balance": 100, "color": "#fff", "icon": "cash", "isArchived": false,
                "createdAt": stamp, "updatedAt": stamp
            }),
        )
        .unwrap();
        finance::save_transaction(
            &connection,
            &json!({
                "id": "t1", "userId": "local-user", "type": "expense", "amount": 12.34,
                "category": "餐饮", "account": "现金", "accountId": "a1",
                "occurredAt": stamp, "createdAt": stamp, "updatedAt": stamp
            }),
        )
        .unwrap();
        habits::save_activity_log(
            &connection,
            &json!({
                "id": "l1", "userId": "local-user", "activityId": "missing-habit",
                "value": 1, "status": "completed", "createdAt": stamp, "updatedAt": stamp
            }),
        )
        .unwrap();
        notes::save_note(
            &connection,
            &json!({
                "title": "备份笔记", "noteType": "document", "contentJson": {"type": "doc"},
                "contentHtml": "", "contentText": "", "contentMarkdown": "", "summary": ""
            }),
            false,
            false,
        )
        .unwrap();
        let notes_backup = notes::backup(&connection).unwrap();

        // 导出（旧 UI 结构 + 版本字段）。
        let accounts = finance::list_accounts(&connection).unwrap();
        let transactions = finance::list_transactions(&connection).unwrap();
        let activities = habits::list_activities(&connection).unwrap();
        let logs = habits::list_activity_logs(&connection).unwrap();
        let reviews = habits::list_daily_reviews(&connection).unwrap();
        let workout_history = workouts::list_workouts(&connection).unwrap();
        let backup = json!({
            "format": "lifetrace-backup",
            "schemaVersion": 2,
            "createdAt": stamp,
            "activities": activities,
            "logs": logs,
            "transactions": transactions,
            "reviews": reviews,
            "accounts": accounts,
            "workoutHistory": workout_history,
            "notesBackup": notes_backup
        });

        // 恢复到全新数据库。
        let (mut target, target_dir) = migrated_connection();
        let data = backup.as_object().unwrap().clone();
        let transaction = target.transaction().unwrap();
        finance::replace_all(
            &transaction,
            data.get("accounts").and_then(Value::as_array).unwrap(),
            data.get("transactions").and_then(Value::as_array).unwrap(),
        )
        .unwrap();
        habits::replace_all(
            &transaction,
            data.get("activities").and_then(Value::as_array).unwrap(),
            data.get("logs").and_then(Value::as_array).unwrap(),
            data.get("reviews").and_then(Value::as_array).unwrap(),
        )
        .unwrap();
        transaction.commit().unwrap();
        notes::restore_backup(
            &mut Connection::open_in_memory().unwrap(),
            &json!({"format": "other"}),
        )
        .unwrap_err();

        assert_eq!(finance::list_transactions(&target).unwrap().len(), 1);
        assert_eq!(
            finance::list_transactions(&target).unwrap()[0]["amount"],
            json!(12.34)
        );
        assert_eq!(habits::list_activity_logs(&target).unwrap().len(), 1);
        assert_eq!(
            habits::list_activity_logs(&target).unwrap()[0]["activityId"],
            Value::Null
        );
        // 笔记备份恢复到另一个全新库。
        let (mut notes_target, notes_target_dir) = migrated_connection();
        notes::restore_backup(&mut notes_target, &notes_backup).unwrap();
        let restored_notes =
            notes::list_notes(&notes_target, None, Some("all"), None, None, None, None, 20)
                .unwrap();
        assert_eq!(restored_notes.len(), 1);
        assert_eq!(restored_notes[0]["title"], json!("备份笔记"));
        fs::remove_dir_all(&directory).ok();
        fs::remove_dir_all(&target_dir).ok();
        fs::remove_dir_all(&notes_target_dir).ok();
    }
}
