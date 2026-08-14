use rusqlite::Transaction;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

/// Persists the execution-specific weekly review snapshot.
///
/// The review is intentionally separate from `review.daily`: it records
/// objective execution metrics derived from tasks, occurrences and completion
/// results, while daily review remains the subjective life-review surface.
pub struct M0015ExecutionWeeklyReviews;

impl Migration for M0015ExecutionWeeklyReviews {
    fn version(&self) -> i64 {
        15
    }

    fn name(&self) -> &'static str {
        "execution-weekly-reviews"
    }

    fn checksum(&self) -> &'static str {
        "m0015-execution-weekly-reviews-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        transaction
            .execute_batch(
                r#"
            CREATE TABLE execution_weekly_reviews (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
              week_start TEXT NOT NULL,
              week_end TEXT NOT NULL,
              planned_count INTEGER NOT NULL DEFAULT 0,
              completed_count INTEGER NOT NULL DEFAULT 0,
              completion_rate REAL NOT NULL DEFAULT 0,
              planned_minutes INTEGER NOT NULL DEFAULT 0,
              actual_minutes INTEGER NOT NULL DEFAULT 0,
              overdue_task_count INTEGER NOT NULL DEFAULT 0,
              overdue_occurrence_count INTEGER NOT NULL DEFAULT 0,
              note TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              deleted_at TEXT,
              version INTEGER NOT NULL DEFAULT 1,
              CHECK(week_start <= week_end),
              CHECK(completion_rate >= 0 AND completion_rate <= 100)
            );
            CREATE UNIQUE INDEX idx_execution_weekly_reviews_user_range
              ON execution_weekly_reviews(user_id,week_start,week_end)
              WHERE deleted_at IS NULL;

            CREATE TRIGGER trg_sync_execution_weekly_reviews_insert AFTER INSERT ON execution_weekly_reviews
            WHEN (SELECT origin FROM sync_context WHERE singleton=1)='local'
            BEGIN
              DELETE FROM sync_outbox
               WHERE profile_id=NEW.user_id AND entity_type='execution.weekly_review'
                 AND entity_id=NEW.id AND status='pending';
              INSERT INTO sync_outbox(
                change_id,profile_id,entity_type,entity_id,operation,base_server_version,
                entity_schema_version,payload_json,dependencies_json,status,retry_count,created_at,updated_at
              ) VALUES(
                lower(hex(randomblob(16))),NEW.user_id,'execution.weekly_review',NEW.id,
                CASE WHEN NEW.deleted_at IS NOT NULL THEN 'delete' ELSE 'upsert' END,
                COALESCE((SELECT server_version FROM sync_metadata WHERE profile_id=NEW.user_id
                  AND entity_type='execution.weekly_review' AND entity_id=NEW.id),'0'),
                1,NULL,'[]','pending',0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
              );
            END;

            CREATE TRIGGER trg_sync_execution_weekly_reviews_update AFTER UPDATE ON execution_weekly_reviews
            WHEN (SELECT origin FROM sync_context WHERE singleton=1)='local'
            BEGIN
              DELETE FROM sync_outbox
               WHERE profile_id=NEW.user_id AND entity_type='execution.weekly_review'
                 AND entity_id=NEW.id AND status='pending';
              INSERT INTO sync_outbox(
                change_id,profile_id,entity_type,entity_id,operation,base_server_version,
                entity_schema_version,payload_json,dependencies_json,status,retry_count,created_at,updated_at
              ) VALUES(
                lower(hex(randomblob(16))),NEW.user_id,'execution.weekly_review',NEW.id,
                CASE WHEN NEW.deleted_at IS NOT NULL THEN 'delete' ELSE 'upsert' END,
                COALESCE((SELECT server_version FROM sync_metadata WHERE profile_id=NEW.user_id
                  AND entity_type='execution.weekly_review' AND entity_id=NEW.id),'0'),
                1,NULL,'[]','pending',0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
              );
            END;

            CREATE TRIGGER trg_sync_execution_weekly_reviews_delete AFTER DELETE ON execution_weekly_reviews
            WHEN (SELECT origin FROM sync_context WHERE singleton=1)='local'
            BEGIN
              DELETE FROM sync_outbox
               WHERE profile_id=OLD.user_id AND entity_type='execution.weekly_review'
                 AND entity_id=OLD.id AND status='pending';
              INSERT INTO sync_outbox(
                change_id,profile_id,entity_type,entity_id,operation,base_server_version,
                entity_schema_version,payload_json,dependencies_json,status,retry_count,created_at,updated_at
              ) VALUES(
                lower(hex(randomblob(16))),OLD.user_id,'execution.weekly_review',OLD.id,'delete',
                COALESCE((SELECT server_version FROM sync_metadata WHERE profile_id=OLD.user_id
                  AND entity_type='execution.weekly_review' AND entity_id=OLD.id),'0'),
                1,NULL,'[]','pending',0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
              );
            END;
            "#,
            )
            .map_err(|error| MigrationError {
                version: 15,
                message: format!("create execution weekly reviews: {error}"),
            })?;

        let mut report = MigrationReport::default();
        report
            .metrics
            .insert("execution_weekly_review_tables".to_owned(), 1);
        report
            .metrics
            .insert("execution_weekly_review_sync_triggers".to_owned(), 3);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use rusqlite::Connection;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn weekly_review_is_syncable_locally() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-m0015-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let profile = crate::database::profile::active_profile_id(&connection).unwrap();

        connection.execute(
            "INSERT INTO execution_weekly_reviews(id,user_id,week_start,week_end,planned_count,completed_count,completion_rate,created_at,updated_at) VALUES('wr1',?1,'2026-08-08','2026-08-14',10,8,80,'2026-08-14T00:00:00Z','2026-08-14T00:00:00Z')",
            [&profile],
        ).unwrap();

        let changes: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='execution.weekly_review' AND entity_id='wr1' AND operation='upsert'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(changes, 1);
    }
}
