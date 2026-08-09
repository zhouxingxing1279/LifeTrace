use rusqlite::{OptionalExtension, Transaction};

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

pub struct M0010ExecutionSync;

#[derive(Clone, Copy)]
struct TriggerSpec {
    table: &'static str,
    entity_type: &'static str,
    id_new: &'static str,
    id_old: &'static str,
    profile_new: &'static str,
    profile_old: &'static str,
    delete_new: &'static str,
}

impl Migration for M0010ExecutionSync {
    fn version(&self) -> i64 {
        10
    }

    fn name(&self) -> &'static str {
        "execution-sync-outbox"
    }

    fn checksum(&self) -> &'static str {
        "m0010-execution-sync-outbox-v1"
    }

    fn up(
        &self,
        tx: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        let specs = [
            TriggerSpec { table:"execution_projects", entity_type:"execution.project", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_recurrence_rules", entity_type:"execution.recurrence_rule", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_tasks", entity_type:"execution.task", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_task_dependencies", entity_type:"execution.task_dependency", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"0" },
            TriggerSpec { table:"execution_task_occurrences", entity_type:"execution.task_occurrence", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_waiting_items", entity_type:"execution.waiting_item", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_calendar_events", entity_type:"execution.calendar_event", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_calendar_occurrences", entity_type:"execution.calendar_occurrence", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_memos", entity_type:"execution.memo", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_memo_tags", entity_type:"execution.memo_tag", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_memo_tag_relations", entity_type:"execution.memo_tag_relation", id_new:"NEW.memo_id || ':' || NEW.tag_id", id_old:"OLD.memo_id || ':' || OLD.tag_id", profile_new:"(SELECT user_id FROM execution_memos WHERE id=NEW.memo_id)", profile_old:"(SELECT user_id FROM execution_memos WHERE id=OLD.memo_id)", delete_new:"0" },
            TriggerSpec { table:"execution_reminders", entity_type:"execution.reminder", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_completion_results", entity_type:"execution.completion_result", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"execution_entity_links", entity_type:"execution.entity_link", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
        ];

        let mut created = 0usize;
        for spec in specs {
            let exists: Option<String> = tx
                .query_row(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
                    [spec.table],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| MigrationError {
                    version: 10,
                    message: error.to_string(),
                })?;
            if exists.is_none() {
                continue;
            }

            for (suffix, timing, raw_profile_expr, id_expr, operation_expr) in [
                (
                    "insert",
                    "AFTER INSERT",
                    spec.profile_new,
                    spec.id_new,
                    format!(
                        "CASE WHEN {} THEN 'delete' ELSE 'upsert' END",
                        spec.delete_new
                    ),
                ),
                (
                    "update",
                    "AFTER UPDATE",
                    spec.profile_new,
                    spec.id_new,
                    format!(
                        "CASE WHEN {} THEN 'delete' ELSE 'upsert' END",
                        spec.delete_new
                    ),
                ),
                (
                    "delete",
                    "AFTER DELETE",
                    spec.profile_old,
                    spec.id_old,
                    "'delete'".to_owned(),
                ),
            ] {
                let trigger_name = format!("trg_sync_{}_{}", spec.table, suffix);
                let profile_expr = format!(
                    "COALESCE((SELECT id FROM local_profiles WHERE id=({raw_profile_expr})), (SELECT active_profile_id FROM app_profile_state WHERE singleton=1))"
                );
                let sql = format!(
                    "CREATE TRIGGER IF NOT EXISTS {trigger_name} {timing} ON {table}
                     WHEN (SELECT origin FROM sync_context WHERE singleton=1)='local'
                       AND ({profile_expr}) IS NOT NULL
                     BEGIN
                       DELETE FROM sync_outbox
                        WHERE profile_id=({profile_expr}) AND entity_type='{entity_type}'
                          AND entity_id=({id_expr}) AND status='pending';
                       INSERT INTO sync_outbox(
                         change_id,profile_id,entity_type,entity_id,operation,base_server_version,
                         entity_schema_version,payload_json,dependencies_json,status,retry_count,created_at,updated_at
                       ) VALUES(
                         lower(hex(randomblob(16))),({profile_expr}),'{entity_type}',({id_expr}),{operation_expr},
                         COALESCE((SELECT server_version FROM sync_metadata WHERE profile_id=({profile_expr})
                           AND entity_type='{entity_type}' AND entity_id=({id_expr})),'0'),
                         1,NULL,'[]','pending',0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
                       );
                     END;",
                    table = spec.table,
                    entity_type = spec.entity_type,
                );
                tx.execute_batch(&sql).map_err(|error| MigrationError {
                    version: 10,
                    message: format!("create {trigger_name}: {error}"),
                })?;
                created += 1;
            }
        }

        let mut report = MigrationReport::default();
        report
            .metrics
            .insert("execution_sync_triggers".to_owned(), created as i64);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use rusqlite::Connection;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn db() -> (Connection, String) {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-m0010-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let profile: String = connection
            .query_row(
                "SELECT active_profile_id FROM app_profile_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        (connection, profile)
    }

    #[test]
    fn local_execution_writes_enqueue_and_remote_writes_are_suppressed() {
        let (connection, profile) = db();
        connection.execute(
            "INSERT INTO execution_tasks(id,user_id,title,status,priority,created_at,updated_at)
             VALUES('t1',?1,'Task','todo','normal','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&profile],
        ).unwrap();
        connection.execute(
            "INSERT INTO execution_memos(id,user_id,content,plain_text,status,created_at,updated_at)
             VALUES('m1',?1,'Memo','Memo','active','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&profile],
        ).unwrap();
        let task_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='execution.task' AND entity_id='t1' AND operation='upsert'",
            [], |row| row.get(0)
        ).unwrap();
        let memo_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='execution.memo' AND entity_id='m1' AND operation='upsert'",
            [], |row| row.get(0)
        ).unwrap();
        assert_eq!(task_count, 1);
        assert_eq!(memo_count, 1);

        connection.execute(
            "UPDATE sync_context SET origin='remote' WHERE singleton=1",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO execution_memos(id,user_id,content,plain_text,status,created_at,updated_at)
             VALUES('m2',?1,'Remote','Remote','active','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&profile],
        ).unwrap();
        let remote_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='execution.memo' AND entity_id='m2'",
            [], |row| row.get(0)
        ).unwrap();
        assert_eq!(remote_count, 0);
    }

    #[test]
    fn soft_delete_and_memo_tag_relation_use_expected_operations() {
        let (connection, profile) = db();
        connection.execute(
            "INSERT INTO execution_memos(id,user_id,content,plain_text,status,created_at,updated_at)
             VALUES('m1',?1,'Memo','Memo','active','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&profile],
        ).unwrap();
        connection.execute(
            "INSERT INTO execution_memo_tags(id,user_id,name,normalized_name,created_at,updated_at)
             VALUES('tag1',?1,'Work','work','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&profile],
        ).unwrap();
        connection.execute(
            "INSERT INTO execution_memo_tag_relations(memo_id,tag_id,created_at)
             VALUES('m1','tag1','2026-08-09T00:00:00Z')",
            [],
        ).unwrap();
        let relation_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='execution.memo_tag_relation' AND entity_id='m1:tag1' AND operation='upsert'",
            [], |row| row.get(0)
        ).unwrap();
        assert_eq!(relation_count, 1);

        connection.execute(
            "UPDATE execution_memos SET deleted_at='2026-08-09T01:00:00Z',updated_at='2026-08-09T01:00:00Z' WHERE id='m1'",
            [],
        ).unwrap();
        let operation: String = connection.query_row(
            "SELECT operation FROM sync_outbox WHERE entity_type='execution.memo' AND entity_id='m1' AND status='pending' ORDER BY created_at DESC LIMIT 1",
            [], |row| row.get(0)
        ).unwrap();
        assert_eq!(operation, "delete");
    }
}
