use rusqlite::Transaction;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

/// Adds the shared Goal layer above execution projects.
///
/// Goal is a first-class sync entity. Projects keep their existing identity and
/// gain an optional `goal_id`, so Goal → Project → Task does not require a
/// second project/task database.
pub struct M0014ExecutionGoals;

impl Migration for M0014ExecutionGoals {
    fn version(&self) -> i64 {
        14
    }

    fn name(&self) -> &'static str {
        "execution-goals"
    }

    fn checksum(&self) -> &'static str {
        "m0014-execution-goals-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        transaction
            .execute_batch(
                r#"
            CREATE TABLE execution_goals (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
              name TEXT NOT NULL,
              description TEXT,
              status TEXT NOT NULL DEFAULT 'active'
                CHECK(status IN ('active','paused','completed','cancelled')),
              target_at TEXT,
              color TEXT,
              icon TEXT,
              sort_order INTEGER NOT NULL DEFAULT 0,
              completed_at TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              deleted_at TEXT,
              version INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX idx_execution_goals_user_status_order
              ON execution_goals(user_id,status,sort_order,updated_at DESC);

            ALTER TABLE execution_projects
              ADD COLUMN goal_id TEXT REFERENCES execution_goals(id) ON DELETE SET NULL;
            CREATE INDEX idx_execution_projects_goal
              ON execution_projects(user_id,goal_id,sort_order);

            CREATE TRIGGER trg_sync_execution_goals_insert AFTER INSERT ON execution_goals
            WHEN (SELECT origin FROM sync_context WHERE singleton=1)='local'
            BEGIN
              DELETE FROM sync_outbox
               WHERE profile_id=NEW.user_id AND entity_type='execution.goal'
                 AND entity_id=NEW.id AND status='pending';
              INSERT INTO sync_outbox(
                change_id,profile_id,entity_type,entity_id,operation,base_server_version,
                entity_schema_version,payload_json,dependencies_json,status,retry_count,created_at,updated_at
              ) VALUES(
                lower(hex(randomblob(16))),NEW.user_id,'execution.goal',NEW.id,
                CASE WHEN NEW.deleted_at IS NOT NULL THEN 'delete' ELSE 'upsert' END,
                COALESCE((SELECT server_version FROM sync_metadata WHERE profile_id=NEW.user_id
                  AND entity_type='execution.goal' AND entity_id=NEW.id),'0'),
                1,NULL,'[]','pending',0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
              );
            END;

            CREATE TRIGGER trg_sync_execution_goals_update AFTER UPDATE ON execution_goals
            WHEN (SELECT origin FROM sync_context WHERE singleton=1)='local'
            BEGIN
              DELETE FROM sync_outbox
               WHERE profile_id=NEW.user_id AND entity_type='execution.goal'
                 AND entity_id=NEW.id AND status='pending';
              INSERT INTO sync_outbox(
                change_id,profile_id,entity_type,entity_id,operation,base_server_version,
                entity_schema_version,payload_json,dependencies_json,status,retry_count,created_at,updated_at
              ) VALUES(
                lower(hex(randomblob(16))),NEW.user_id,'execution.goal',NEW.id,
                CASE WHEN NEW.deleted_at IS NOT NULL THEN 'delete' ELSE 'upsert' END,
                COALESCE((SELECT server_version FROM sync_metadata WHERE profile_id=NEW.user_id
                  AND entity_type='execution.goal' AND entity_id=NEW.id),'0'),
                1,NULL,'[]','pending',0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
              );
            END;

            CREATE TRIGGER trg_sync_execution_goals_delete AFTER DELETE ON execution_goals
            WHEN (SELECT origin FROM sync_context WHERE singleton=1)='local'
            BEGIN
              DELETE FROM sync_outbox
               WHERE profile_id=OLD.user_id AND entity_type='execution.goal'
                 AND entity_id=OLD.id AND status='pending';
              INSERT INTO sync_outbox(
                change_id,profile_id,entity_type,entity_id,operation,base_server_version,
                entity_schema_version,payload_json,dependencies_json,status,retry_count,created_at,updated_at
              ) VALUES(
                lower(hex(randomblob(16))),OLD.user_id,'execution.goal',OLD.id,'delete',
                COALESCE((SELECT server_version FROM sync_metadata WHERE profile_id=OLD.user_id
                  AND entity_type='execution.goal' AND entity_id=OLD.id),'0'),
                1,NULL,'[]','pending',0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
              );
            END;
            "#,
            )
            .map_err(|error| MigrationError {
                version: 14,
                message: format!("create execution goal layer: {error}"),
            })?;

        let mut report = MigrationReport::default();
        report.metrics.insert("execution_goal_tables".to_owned(), 1);
        report
            .metrics
            .insert("execution_goal_sync_triggers".to_owned(), 3);
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
    fn goal_and_project_link_are_syncable_locally() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-m0014-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let profile = crate::database::profile::active_profile_id(&connection).unwrap();

        connection
            .execute(
                "INSERT INTO execution_goals(id,user_id,name,status,created_at,updated_at) VALUES('g1',?1,'Graduate','active','2026-08-14T00:00:00Z','2026-08-14T00:00:00Z')",
                [&profile],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_projects(id,user_id,name,status,sort_order,goal_id,created_at,updated_at) VALUES('p1',?1,'Thesis','active',0,'g1','2026-08-14T00:00:00Z','2026-08-14T00:00:00Z')",
                [&profile],
            )
            .unwrap();

        let goal_changes: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE entity_type='execution.goal' AND entity_id='g1' AND operation='upsert'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let project_goal: String = connection
            .query_row(
                "SELECT goal_id FROM execution_projects WHERE id='p1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(goal_changes, 1);
        assert_eq!(project_goal, "g1");
    }
}
