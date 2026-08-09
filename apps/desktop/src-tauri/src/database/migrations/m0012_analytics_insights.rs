use rusqlite::Transaction;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

/// EPIC-14: analytics/insights derived-data schema.
///
/// All tables created here are projections or generated snapshots. Domain tables remain the
/// source of truth, so every projection can be discarded and rebuilt without losing user data.
pub struct M0012AnalyticsInsights;

impl Migration for M0012AnalyticsInsights {
    fn version(&self) -> i64 {
        12
    }

    fn name(&self) -> &'static str {
        "analytics-insights-projections"
    }

    fn checksum(&self) -> &'static str {
        "m0012-analytics-insights-v2"
    }

    fn up(
        &self,
        transaction: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS analytics_projection_state (
                  user_id TEXT PRIMARY KEY,
                  dirty INTEGER NOT NULL DEFAULT 1 CHECK(dirty IN (0,1)),
                  projection_version INTEGER NOT NULL DEFAULT 1 CHECK(projection_version >= 1),
                  last_rebuilt_at TEXT,
                  last_error TEXT
                );

                CREATE TABLE IF NOT EXISTS analytics_events (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  occurred_at TEXT NOT NULL,
                  ended_at TEXT,
                  local_date TEXT NOT NULL,
                  timezone TEXT,
                  domain TEXT NOT NULL,
                  event_type TEXT NOT NULL,
                  title TEXT NOT NULL,
                  summary TEXT NOT NULL DEFAULT '',
                  entity_type TEXT NOT NULL,
                  entity_id TEXT NOT NULL,
                  source_updated_at TEXT,
                  metrics_json TEXT NOT NULL DEFAULT '{}',
                  tags_json TEXT NOT NULL DEFAULT '[]',
                  search_text TEXT NOT NULL DEFAULT '',
                  projection_version INTEGER NOT NULL DEFAULT 1 CHECK(projection_version >= 1),
                  projected_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_analytics_events_user_time
                  ON analytics_events(user_id, occurred_at DESC, id);
                CREATE INDEX IF NOT EXISTS idx_analytics_events_user_domain_time
                  ON analytics_events(user_id, domain, occurred_at DESC, id);
                CREATE INDEX IF NOT EXISTS idx_analytics_events_entity
                  ON analytics_events(user_id, entity_type, entity_id);

                CREATE TABLE IF NOT EXISTS analytics_search_documents (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  domain TEXT NOT NULL,
                  entity_type TEXT NOT NULL,
                  entity_id TEXT NOT NULL,
                  title TEXT NOT NULL,
                  body TEXT NOT NULL DEFAULT '',
                  keywords TEXT NOT NULL DEFAULT '',
                  tags_json TEXT NOT NULL DEFAULT '[]',
                  occurred_at TEXT,
                  updated_at TEXT NOT NULL,
                  projection_version INTEGER NOT NULL DEFAULT 1 CHECK(projection_version >= 1),
                  projected_at TEXT NOT NULL,
                  UNIQUE(user_id, entity_type, entity_id)
                );
                CREATE INDEX IF NOT EXISTS idx_analytics_search_user_domain_updated
                  ON analytics_search_documents(user_id, domain, updated_at DESC);
                CREATE INDEX IF NOT EXISTS idx_analytics_search_entity
                  ON analytics_search_documents(user_id, entity_type, entity_id);
                CREATE INDEX IF NOT EXISTS idx_analytics_search_title
                  ON analytics_search_documents(title);

                CREATE TABLE IF NOT EXISTS analytics_reports (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  report_type TEXT NOT NULL CHECK(report_type IN ('weekly','monthly','custom')),
                  period_start TEXT NOT NULL,
                  period_end TEXT NOT NULL,
                  timezone TEXT NOT NULL DEFAULT 'UTC',
                  facts_json TEXT NOT NULL,
                  narrative_json TEXT,
                  source_coverage_json TEXT NOT NULL DEFAULT '{}',
                  facts_version INTEGER NOT NULL DEFAULT 1 CHECK(facts_version >= 1),
                  prompt_version TEXT,
                  model_info_json TEXT,
                  generated_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  UNIQUE(user_id, report_type, period_start, period_end, timezone, facts_version)
                );
                CREATE INDEX IF NOT EXISTS idx_analytics_reports_user_period
                  ON analytics_reports(user_id, period_start DESC, period_end DESC);

                CREATE TABLE IF NOT EXISTS analytics_insights (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  insight_type TEXT NOT NULL,
                  period_start TEXT NOT NULL,
                  period_end TEXT NOT NULL,
                  title TEXT NOT NULL,
                  summary TEXT NOT NULL,
                  evidence_json TEXT NOT NULL DEFAULT '{}',
                  sample_size INTEGER NOT NULL DEFAULT 0 CHECK(sample_size >= 0),
                  confidence_json TEXT NOT NULL DEFAULT '{}',
                  algorithm_version TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_analytics_insights_user_period
                  ON analytics_insights(user_id, period_start DESC, period_end DESC);
                CREATE UNIQUE INDEX IF NOT EXISTS uq_analytics_insight_snapshot
                  ON analytics_insights(
                    user_id, insight_type, period_start, period_end, algorithm_version
                  );

                CREATE TRIGGER IF NOT EXISTS trg_analytics_transactions_insert
                AFTER INSERT ON transactions BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_transactions_update
                AFTER UPDATE ON transactions BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_transactions_delete
                AFTER DELETE ON transactions BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;

                CREATE TRIGGER IF NOT EXISTS trg_analytics_activities_update
                AFTER UPDATE ON activities BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_activity_logs_insert
                AFTER INSERT ON activity_logs BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_activity_logs_update
                AFTER UPDATE ON activity_logs BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_activity_logs_delete
                AFTER DELETE ON activity_logs BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_daily_reviews_change
                AFTER INSERT ON daily_reviews BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_daily_reviews_update
                AFTER UPDATE ON daily_reviews BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_daily_reviews_delete
                AFTER DELETE ON daily_reviews BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;

                CREATE TRIGGER IF NOT EXISTS trg_analytics_notes_insert
                AFTER INSERT ON notes BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_notes_update
                AFTER UPDATE ON notes BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_notes_delete
                AFTER DELETE ON notes BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;

                CREATE TRIGGER IF NOT EXISTS trg_analytics_english_records_insert
                AFTER INSERT ON english_learning_records BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_english_records_update
                AFTER UPDATE ON english_learning_records BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_english_records_delete
                AFTER DELETE ON english_learning_records BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_english_vocab_insert
                AFTER INSERT ON english_vocabulary BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_english_vocab_update
                AFTER UPDATE ON english_vocabulary BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_english_vocab_delete
                AFTER DELETE ON english_vocabulary BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_english_articles_update
                AFTER UPDATE ON english_articles BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  SELECT DISTINCT user_id,1,1 FROM english_learning_records
                   WHERE article_id=NEW.id AND deleted_at IS NULL
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;

                CREATE TRIGGER IF NOT EXISTS trg_analytics_workouts_insert
                AFTER INSERT ON workouts BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_workouts_update
                AFTER UPDATE ON workouts BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_workouts_delete
                AFTER DELETE ON workouts BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;

                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_tasks_insert
                AFTER INSERT ON execution_tasks BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_tasks_update
                AFTER UPDATE ON execution_tasks BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_tasks_delete
                AFTER DELETE ON execution_tasks BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_calendar_insert
                AFTER INSERT ON execution_calendar_events BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_calendar_update
                AFTER UPDATE ON execution_calendar_events BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_calendar_delete
                AFTER DELETE ON execution_calendar_events BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_memos_insert
                AFTER INSERT ON execution_memos BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_memos_update
                AFTER UPDATE ON execution_memos BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(NEW.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                CREATE TRIGGER IF NOT EXISTS trg_analytics_execution_memos_delete
                AFTER DELETE ON execution_memos BEGIN
                  INSERT INTO analytics_projection_state(user_id,dirty,projection_version)
                  VALUES(OLD.user_id,1,1)
                  ON CONFLICT(user_id) DO UPDATE SET dirty=1,last_error=NULL;
                END;
                "#,
            )
            .map_err(|error| MigrationError {
                version: 12,
                message: format!("create analytics schema: {error}"),
            })?;

        let mut report = MigrationReport::default();
        report.metrics.insert("analytics_events".to_owned(), 1);
        report
            .metrics
            .insert("analytics_search_documents".to_owned(), 1);
        report.metrics.insert("analytics_reports".to_owned(), 1);
        report.metrics.insert("analytics_insights".to_owned(), 1);
        report
            .metrics
            .insert("analytics_projection_state".to_owned(), 1);
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
    fn migration_creates_projection_tables_and_source_triggers() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-m0012-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let context = MigrationContext::new(directory);
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        run(&mut connection, &context, &all()).unwrap();

        for table in [
            "analytics_projection_state",
            "analytics_events",
            "analytics_search_documents",
            "analytics_reports",
            "analytics_insights",
        ] {
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing analytics table {table}");
        }

        connection
            .execute(
                "INSERT INTO transactions(
                   id,user_id,transaction_type,amount_cents,currency,occurred_at,local_date,status,
                   source_type,created_at,updated_at,version
                 ) VALUES(
                   'tx-analytics-test','local','expense',1200,'CNY','2026-08-09T08:00:00Z',
                   '2026-08-09','confirmed','manual','2026-08-09T08:00:00Z',
                   '2026-08-09T08:00:00Z',1
                 )",
                [],
            )
            .unwrap();
        let dirty: i64 = connection
            .query_row(
                "SELECT dirty FROM analytics_projection_state WHERE user_id='local'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dirty, 1);
    }
}
