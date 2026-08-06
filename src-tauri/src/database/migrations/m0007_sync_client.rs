use chrono::Utc;
use rusqlite::{params, OptionalExtension, Transaction};
use uuid::Uuid;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

/// EPIC-05 local profile ownership and durable sync protocol state.
pub struct M0007SyncClient;

impl Migration for M0007SyncClient {
    fn version(&self) -> i64 {
        7
    }
    fn name(&self) -> &'static str {
        "windows-client-sync-core"
    }
    fn checksum(&self) -> &'static str {
        "m0007-sync-client-v1"
    }

    fn up(
        &self,
        tx: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS local_profiles (
               id TEXT PRIMARY KEY,
               display_name TEXT NOT NULL,
               cloud_user_id TEXT UNIQUE,
               cloud_binding_state TEXT NOT NULL DEFAULT 'local_only'
                 CHECK(cloud_binding_state IN ('local_only','pending_choice','bound','detached')),
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS app_profile_state (
               singleton INTEGER PRIMARY KEY CHECK(singleton=1),
               active_profile_id TEXT NOT NULL REFERENCES local_profiles(id),
               updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sync_outbox (
               change_id TEXT PRIMARY KEY,
               profile_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
               entity_type TEXT NOT NULL,
               entity_id TEXT NOT NULL,
               operation TEXT NOT NULL CHECK(operation IN ('upsert','delete')),
               base_server_version TEXT NOT NULL DEFAULT '0',
               entity_schema_version INTEGER NOT NULL DEFAULT 1,
               payload_json TEXT,
               dependencies_json TEXT NOT NULL DEFAULT '[]',
               atomic_group_id TEXT,
               status TEXT NOT NULL DEFAULT 'pending'
                 CHECK(status IN ('pending','leased','confirmed','conflict','blocked','dead_letter')),
               retry_count INTEGER NOT NULL DEFAULT 0,
               next_attempt_at TEXT,
               lease_owner TEXT,
               lease_expires_at TEXT,
               last_error_code TEXT,
               last_error_message TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               UNIQUE(profile_id, entity_type, entity_id, change_id)
             );
             CREATE INDEX IF NOT EXISTS idx_sync_outbox_ready
               ON sync_outbox(profile_id,status,next_attempt_at,created_at);
             CREATE INDEX IF NOT EXISTS idx_sync_outbox_entity
               ON sync_outbox(profile_id,entity_type,entity_id,status);
             CREATE TABLE IF NOT EXISTS sync_state (
               profile_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
               scope_key TEXT NOT NULL,
               cursor TEXT,
               snapshot_id TEXT,
               snapshot_page_token TEXT,
               snapshot_cursor TEXT,
               snapshot_in_progress INTEGER NOT NULL DEFAULT 0,
               phase TEXT NOT NULL DEFAULT 'local_only',
               pending_count INTEGER NOT NULL DEFAULT 0,
               conflict_count INTEGER NOT NULL DEFAULT 0,
               last_success_at TEXT,
               next_retry_at TEXT,
               last_error_code TEXT,
               last_error_message TEXT,
               updated_at TEXT NOT NULL,
               PRIMARY KEY(profile_id,scope_key)
             );
             CREATE TABLE IF NOT EXISTS sync_conflicts (
               conflict_id TEXT PRIMARY KEY,
               profile_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
               change_id TEXT,
               entity_type TEXT NOT NULL,
               entity_id TEXT NOT NULL,
               conflict_type TEXT NOT NULL,
               base_server_version TEXT NOT NULL,
               server_version TEXT NOT NULL,
               local_payload_json TEXT,
               remote_payload_json TEXT,
               server_deleted INTEGER NOT NULL DEFAULT 0,
               status TEXT NOT NULL DEFAULT 'unresolved'
                 CHECK(status IN ('unresolved','resolved','discarded')),
               resolution TEXT,
               created_at TEXT NOT NULL,
               resolved_at TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_sync_conflicts_profile
               ON sync_conflicts(profile_id,status,created_at);
             CREATE TABLE IF NOT EXISTS sync_metadata (
               profile_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
               entity_type TEXT NOT NULL,
               entity_id TEXT NOT NULL,
               server_version TEXT,
               server_hash TEXT,
               local_hash TEXT,
               last_server_cursor TEXT,
               last_synced_at TEXT,
               PRIMARY KEY(profile_id,entity_type,entity_id)
             );
             CREATE TABLE IF NOT EXISTS sync_snapshot_staging (
               profile_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
               scope_key TEXT NOT NULL,
               entity_type TEXT NOT NULL,
               entity_id TEXT NOT NULL,
               server_version TEXT NOT NULL,
               payload_json TEXT NOT NULL,
               PRIMARY KEY(profile_id,scope_key,entity_type,entity_id)
             );
             CREATE TABLE IF NOT EXISTS sync_materialized_entities (
               profile_id TEXT NOT NULL REFERENCES local_profiles(id) ON DELETE CASCADE,
               entity_type TEXT NOT NULL,
               entity_id TEXT NOT NULL,
               payload_json TEXT,
               deleted_at TEXT,
               updated_at TEXT NOT NULL,
               PRIMARY KEY(profile_id,entity_type,entity_id)
             );
             CREATE TABLE IF NOT EXISTS sync_audit_log (
               id TEXT PRIMARY KEY,
               profile_id TEXT,
               event_type TEXT NOT NULL,
               entity_type TEXT,
               entity_id TEXT,
               details_json TEXT,
               created_at TEXT NOT NULL
             );"
        ).map_err(|error| MigrationError { version: 7, message: error.to_string() })?;

        let existing: Option<String> = tx
            .query_row(
                "SELECT active_profile_id FROM app_profile_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| MigrationError {
                version: 7,
                message: error.to_string(),
            })?;
        let profile_id = existing.unwrap_or_else(|| Uuid::new_v4().to_string());
        let stamp = Utc::now().to_rfc3339();
        tx.execute(
            "INSERT OR IGNORE INTO local_profiles(id,display_name,cloud_binding_state,created_at,updated_at)
             VALUES(?1,'本机资料','local_only',?2,?2)", params![profile_id, stamp]
        ).map_err(|error| MigrationError { version: 7, message: error.to_string() })?;
        tx.execute(
            "INSERT INTO app_profile_state(singleton,active_profile_id,updated_at) VALUES(1,?1,?2)
             ON CONFLICT(singleton) DO UPDATE SET updated_at=excluded.updated_at",
            params![profile_id, stamp],
        )
        .map_err(|error| MigrationError {
            version: 7,
            message: error.to_string(),
        })?;

        let tables = [
            "finance_accounts",
            "transaction_categories",
            "transactions",
            "activities",
            "activity_logs",
            "daily_reviews",
            "note_folders",
            "note_tags",
            "notes",
            "english_learning_records",
            "english_highlights",
            "english_notes",
            "english_vocabulary",
            "english_ai_analysis",
            "workouts",
            "workout_imports",
            "training_notes",
        ];
        let mut migrated = 0usize;
        for table in tables {
            let sql = format!(
                "UPDATE {table} SET user_id=?1 WHERE user_id IS NULL OR trim(user_id)='' OR user_id IN ('local','local-user')"
            );
            match tx.execute(&sql, [&profile_id]) {
                Ok(count) => migrated += count,
                Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                    if message.contains("no such table") || message.contains("no such column") => {}
                Err(error) => {
                    return Err(MigrationError {
                        version: 7,
                        message: format!("migrate owner in {table}: {error}"),
                    })
                }
            }
        }
        let mut report = MigrationReport::default();
        report.migrated = migrated;
        report
            .metrics
            .insert("profile_owned_rows".to_owned(), migrated as i64);
        report.metrics.insert("local_profiles".to_owned(), 1);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, Migration};
    use crate::database::migrations::{
        M0001Framework, M0002Finance, M0003HabitsReviews, M0004Notes, M0005English, M0006Workouts,
    };
    use rusqlite::Connection;

    #[test]
    fn replaces_placeholder_owner_and_creates_sync_tables() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let context = MigrationContext::new(std::env::temp_dir());
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(M0001Framework),
            Box::new(M0002Finance),
            Box::new(M0003HabitsReviews),
            Box::new(M0004Notes),
            Box::new(M0005English),
            Box::new(M0006Workouts),
            Box::new(M0007SyncClient),
        ];
        run(&mut connection, &context, &migrations).unwrap();
        let profile: String = connection
            .query_row(
                "SELECT active_profile_id FROM app_profile_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(uuid::Uuid::parse_str(&profile).is_ok());
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN
             ('sync_outbox','sync_state','sync_conflicts','sync_metadata','sync_snapshot_staging')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 5);
    }
}
