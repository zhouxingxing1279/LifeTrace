use rusqlite::Transaction;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

pub struct M0011ExecutionCompletionBackfill;

impl Migration for M0011ExecutionCompletionBackfill {
    fn version(&self) -> i64 {
        11
    }

    fn name(&self) -> &'static str {
        "execution-completion-backfill"
    }

    fn checksum(&self) -> &'static str {
        "m0011-execution-completion-backfill-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        let migrated = transaction
            .execute(
                "INSERT INTO execution_completion_results(
                   id,user_id,task_id,summary,completed_at,actual_minutes,created_at,updated_at,version
                 )
                 SELECT lower(hex(randomblob(16))),t.user_id,t.id,'',
                        COALESCE(t.completed_at,t.updated_at),t.actual_minutes,
                        COALESCE(t.completed_at,t.updated_at),COALESCE(t.completed_at,t.updated_at),1
                   FROM execution_tasks t
                  WHERE t.status='done' AND t.deleted_at IS NULL
                    AND NOT EXISTS(
                      SELECT 1 FROM execution_completion_results r
                       WHERE r.user_id=t.user_id AND r.task_id=t.id AND r.deleted_at IS NULL
                    )",
                [],
            )
            .map_err(|error| MigrationError {
                version: 11,
                message: format!("backfill execution completion results: {error}"),
            })?;

        let mut report = MigrationReport {
            migrated,
            ..MigrationReport::default()
        };
        report
            .metrics
            .insert("completion_results_backfilled".to_owned(), migrated as i64);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{
        migration_runner::{run, MigrationContext},
        migrations::all,
    };
    use rusqlite::Connection;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn backfill_creates_missing_completion_result_once() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-m0011-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let context = MigrationContext::new(directory);
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &context, &all()).unwrap();
        let user_id: String = connection
            .query_row(
                "SELECT active_profile_id FROM app_profile_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_tasks(
                   id,user_id,title,status,priority,actual_minutes,completed_at,created_at,updated_at,version
                 ) VALUES('done-task',?1,'Done','done','normal',42,'2026-08-09T01:00:00Z','2026-08-09T00:00:00Z','2026-08-09T01:00:00Z',1)",
                [&user_id],
            )
            .unwrap();

        {
            let transaction = connection.transaction().unwrap();
            let report = M0011ExecutionCompletionBackfill
                .up(&transaction, &context)
                .unwrap();
            assert_eq!(report.migrated, 1);
            transaction.commit().unwrap();
        }
        {
            let transaction = connection.transaction().unwrap();
            let report = M0011ExecutionCompletionBackfill
                .up(&transaction, &context)
                .unwrap();
            assert_eq!(report.migrated, 0);
            transaction.commit().unwrap();
        }
        let value: (String, String, Option<i64>) = connection
            .query_row(
                "SELECT summary,completed_at,actual_minutes FROM execution_completion_results
                 WHERE task_id='done-task' AND deleted_at IS NULL",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(value.0, "");
        assert_eq!(value.1, "2026-08-09T01:00:00Z");
        assert_eq!(value.2, Some(42));
    }
}
