use rusqlite::{OptionalExtension, Transaction};

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

pub struct M0008SyncTriggers;

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

impl Migration for M0008SyncTriggers {
    fn version(&self) -> i64 {
        8
    }
    fn name(&self) -> &'static str {
        "sync-outbox-triggers"
    }
    fn checksum(&self) -> &'static str {
        "m0008-sync-triggers-v1"
    }

    fn up(
        &self,
        tx: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_context (
               singleton INTEGER PRIMARY KEY CHECK(singleton=1),
               origin TEXT NOT NULL CHECK(origin IN ('local','remote','migration'))
             );
             INSERT OR IGNORE INTO sync_context(singleton,origin) VALUES(1,'local');",
        )
        .map_err(|error| MigrationError {
            version: 8,
            message: error.to_string(),
        })?;

        let specs = [
            TriggerSpec { table:"finance_accounts", entity_type:"finance.account", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"transaction_categories", entity_type:"finance.category", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"transactions", entity_type:"finance.transaction", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"transaction_evidence", entity_type:"finance.transaction_evidence", id_new:"NEW.id", id_old:"OLD.id", profile_new:"(SELECT user_id FROM transactions WHERE id=NEW.transaction_id)", profile_old:"(SELECT user_id FROM transactions WHERE id=OLD.transaction_id)", delete_new:"0" },
            TriggerSpec { table:"activities", entity_type:"habit.activity", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"activity_logs", entity_type:"habit.log", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"daily_reviews", entity_type:"review.daily", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"note_folders", entity_type:"note.folder", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"note_tags", entity_type:"note.tag", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"notes", entity_type:"note.note", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"note_tag_relations", entity_type:"note.tag_relation", id_new:"NEW.note_id || ':' || NEW.tag_id", id_old:"OLD.note_id || ':' || OLD.tag_id", profile_new:"(SELECT user_id FROM notes WHERE id=NEW.note_id)", profile_old:"(SELECT user_id FROM notes WHERE id=OLD.note_id)", delete_new:"0" },
            TriggerSpec { table:"note_relations", entity_type:"note.relation", id_new:"NEW.id", id_old:"OLD.id", profile_new:"(SELECT user_id FROM notes WHERE id=NEW.note_id)", profile_old:"(SELECT user_id FROM notes WHERE id=OLD.note_id)", delete_new:"0" },
            TriggerSpec { table:"note_revisions", entity_type:"note.revision", id_new:"NEW.id", id_old:"OLD.id", profile_new:"(SELECT user_id FROM notes WHERE id=NEW.note_id)", profile_old:"(SELECT user_id FROM notes WHERE id=OLD.note_id)", delete_new:"0" },
            TriggerSpec { table:"english_learning_records", entity_type:"english.learning_record", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"english_highlights", entity_type:"english.highlight", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"english_notes", entity_type:"english.note", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"english_vocabulary", entity_type:"english.vocabulary", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"vocabulary_occurrences", entity_type:"english.vocabulary_occurrence", id_new:"NEW.id", id_old:"OLD.id", profile_new:"(SELECT user_id FROM english_vocabulary WHERE id=NEW.vocabulary_id)", profile_old:"(SELECT user_id FROM english_vocabulary WHERE id=OLD.vocabulary_id)", delete_new:"0" },
            TriggerSpec { table:"vocabulary_review_state", entity_type:"english.vocabulary_review_state", id_new:"NEW.vocabulary_id", id_old:"OLD.vocabulary_id", profile_new:"(SELECT user_id FROM english_vocabulary WHERE id=NEW.vocabulary_id)", profile_old:"(SELECT user_id FROM english_vocabulary WHERE id=OLD.vocabulary_id)", delete_new:"0" },
            TriggerSpec { table:"workout_imports", entity_type:"workout.import", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"workouts", entity_type:"workout.workout", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
            TriggerSpec { table:"workout_exercises", entity_type:"workout.exercise", id_new:"NEW.id", id_old:"OLD.id", profile_new:"(SELECT user_id FROM workouts WHERE id=NEW.workout_id)", profile_old:"(SELECT user_id FROM workouts WHERE id=OLD.workout_id)", delete_new:"0" },
            TriggerSpec { table:"workout_sets", entity_type:"workout.set", id_new:"NEW.id", id_old:"OLD.id", profile_new:"(SELECT w.user_id FROM workout_exercises e JOIN workouts w ON w.id=e.workout_id WHERE e.id=NEW.exercise_id)", profile_old:"(SELECT w.user_id FROM workout_exercises e JOIN workouts w ON w.id=e.workout_id WHERE e.id=OLD.exercise_id)", delete_new:"0" },
            TriggerSpec { table:"training_notes", entity_type:"workout.training_note", id_new:"NEW.id", id_old:"OLD.id", profile_new:"NEW.user_id", profile_old:"OLD.user_id", delete_new:"NEW.deleted_at IS NOT NULL" },
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
                    version: 8,
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
                    table=spec.table, entity_type=spec.entity_type,
                );
                tx.execute_batch(&sql).map_err(|error| MigrationError {
                    version: 8,
                    message: format!("create {trigger_name}: {error}"),
                })?;
                created += 1;
            }
        }
        let mut report = MigrationReport::default();
        report
            .metrics
            .insert("sync_triggers".to_owned(), created as i64);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use rusqlite::Connection;
    use serde_json::json;

    #[test]
    fn local_write_enqueues_and_remote_write_is_suppressed() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(
            &mut connection,
            &MigrationContext::new(std::env::temp_dir()),
            &all(),
        )
        .unwrap();
        let profile: String = connection
            .query_row(
                "SELECT active_profile_id FROM app_profile_state",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut value = json!({"id":"a1","userId":profile,"name":"现金","type":"cash","balance":0});
        crate::database::repositories::finance::save_account(&connection, &value).unwrap();
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='finance.account'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        connection
            .execute(
                "UPDATE sync_context SET origin='remote' WHERE singleton=1",
                [],
            )
            .unwrap();
        value["name"] = json!("远端现金");
        crate::database::repositories::finance::save_account(&connection, &value).unwrap();
        let count_after: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='finance.account'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 1);
    }
}
