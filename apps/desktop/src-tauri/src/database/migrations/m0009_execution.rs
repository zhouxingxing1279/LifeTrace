use rusqlite::Transaction;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

/// m0009：EPIC-20 个人执行系统核心本地 Schema。
///
/// 仅负责可以由 SQLite 稳定保证的数据完整性；依赖环检测、状态机、
/// Memo 转换幂等性等跨记录业务规则由 Execution Domain Service 统一处理。
pub struct M0009Execution;

impl Migration for M0009Execution {
    fn version(&self) -> i64 {
        9
    }

    fn name(&self) -> &'static str {
        "personal-execution-core"
    }

    fn checksum(&self) -> &'static str {
        "m0009-personal-execution-core-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS execution_projects (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  name TEXT NOT NULL CHECK(length(trim(name)) > 0),
                  description TEXT,
                  status TEXT NOT NULL DEFAULT 'active'
                    CHECK(status IN ('active','completed','archived','cancelled')),
                  color TEXT,
                  icon TEXT,
                  sort_order INTEGER NOT NULL DEFAULT 0,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_execution_projects_user_status
                  ON execution_projects(user_id, status, deleted_at, sort_order);

                CREATE TABLE IF NOT EXISTS execution_recurrence_rules (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  frequency TEXT NOT NULL CHECK(frequency IN ('daily','weekly','monthly')),
                  interval_value INTEGER NOT NULL DEFAULT 1 CHECK(interval_value >= 1),
                  weekdays_json TEXT,
                  month_day INTEGER CHECK(month_day BETWEEN 1 AND 31),
                  timezone TEXT,
                  until_at TEXT,
                  max_occurrences INTEGER CHECK(max_occurrences IS NULL OR max_occurrences >= 1),
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT,
                  CHECK(frequency = 'weekly' OR weekdays_json IS NULL),
                  CHECK(frequency = 'monthly' OR month_day IS NULL)
                );
                CREATE INDEX IF NOT EXISTS idx_execution_recurrence_rules_user
                  ON execution_recurrence_rules(user_id, deleted_at);

                CREATE TABLE IF NOT EXISTS execution_tasks (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  project_id TEXT REFERENCES execution_projects(id) ON DELETE SET NULL,
                  parent_task_id TEXT REFERENCES execution_tasks(id) ON DELETE SET NULL,
                  title TEXT NOT NULL CHECK(length(trim(title)) > 0),
                  description TEXT,
                  status TEXT NOT NULL DEFAULT 'todo'
                    CHECK(status IN ('todo','in_progress','waiting','done','cancelled')),
                  priority TEXT NOT NULL DEFAULT 'normal'
                    CHECK(priority IN ('low','normal','high','urgent')),
                  estimated_minutes INTEGER CHECK(estimated_minutes IS NULL OR estimated_minutes >= 0),
                  actual_minutes INTEGER CHECK(actual_minutes IS NULL OR actual_minutes >= 0),
                  due_at TEXT,
                  scheduled_start_at TEXT,
                  scheduled_end_at TEXT,
                  timezone TEXT,
                  context TEXT,
                  completed_at TEXT,
                  cancelled_at TEXT,
                  recurrence_rule_id TEXT REFERENCES execution_recurrence_rules(id) ON DELETE SET NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT,
                  CHECK(parent_task_id IS NULL OR parent_task_id <> id),
                  CHECK(
                    scheduled_start_at IS NULL OR scheduled_end_at IS NULL
                    OR scheduled_end_at >= scheduled_start_at
                  )
                );
                CREATE INDEX IF NOT EXISTS idx_execution_tasks_user_status_due
                  ON execution_tasks(user_id, status, due_at, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_tasks_project
                  ON execution_tasks(project_id, status, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_tasks_parent
                  ON execution_tasks(parent_task_id, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_tasks_schedule
                  ON execution_tasks(scheduled_start_at, scheduled_end_at, deleted_at);

                CREATE TABLE IF NOT EXISTS execution_task_dependencies (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  task_id TEXT NOT NULL REFERENCES execution_tasks(id) ON DELETE CASCADE,
                  depends_on_task_id TEXT NOT NULL REFERENCES execution_tasks(id) ON DELETE CASCADE,
                  dependency_type TEXT NOT NULL DEFAULT 'finish_before_start'
                    CHECK(dependency_type IN ('finish_before_start')),
                  created_at TEXT NOT NULL,
                  CHECK(task_id <> depends_on_task_id),
                  UNIQUE(task_id, depends_on_task_id, dependency_type)
                );
                CREATE INDEX IF NOT EXISTS idx_execution_task_dependencies_task
                  ON execution_task_dependencies(task_id);
                CREATE INDEX IF NOT EXISTS idx_execution_task_dependencies_prerequisite
                  ON execution_task_dependencies(depends_on_task_id);

                CREATE TABLE IF NOT EXISTS execution_task_occurrences (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  task_id TEXT NOT NULL REFERENCES execution_tasks(id) ON DELETE CASCADE,
                  occurrence_key TEXT NOT NULL,
                  scheduled_start_at TEXT,
                  scheduled_end_at TEXT,
                  due_at TEXT,
                  status TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending','in_progress','waiting','done','skipped','cancelled')),
                  title_override TEXT,
                  description_override TEXT,
                  completed_at TEXT,
                  skipped_at TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT,
                  CHECK(
                    scheduled_start_at IS NULL OR scheduled_end_at IS NULL
                    OR scheduled_end_at >= scheduled_start_at
                  ),
                  UNIQUE(task_id, occurrence_key)
                );
                CREATE INDEX IF NOT EXISTS idx_execution_task_occurrences_window
                  ON execution_task_occurrences(task_id, scheduled_start_at, status, deleted_at);

                CREATE TABLE IF NOT EXISTS execution_waiting_items (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  title TEXT NOT NULL CHECK(length(trim(title)) > 0),
                  description TEXT,
                  status TEXT NOT NULL DEFAULT 'open'
                    CHECK(status IN ('open','resolved','cancelled')),
                  waiting_for TEXT NOT NULL CHECK(length(trim(waiting_for)) > 0),
                  expected_at TEXT,
                  follow_up_at TEXT,
                  resolved_at TEXT,
                  resolution_summary TEXT,
                  source_task_id TEXT REFERENCES execution_tasks(id) ON DELETE SET NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_execution_waiting_items_user_status
                  ON execution_waiting_items(user_id, status, follow_up_at, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_waiting_items_source_task
                  ON execution_waiting_items(source_task_id, deleted_at);

                CREATE TABLE IF NOT EXISTS execution_calendar_events (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  title TEXT NOT NULL CHECK(length(trim(title)) > 0),
                  description TEXT,
                  is_all_day INTEGER NOT NULL DEFAULT 0 CHECK(is_all_day IN (0,1)),
                  start_at TEXT,
                  end_at TEXT,
                  start_local_date TEXT,
                  end_local_date TEXT,
                  timezone TEXT,
                  status TEXT NOT NULL DEFAULT 'scheduled'
                    CHECK(status IN ('scheduled','cancelled')),
                  recurrence_rule_id TEXT REFERENCES execution_recurrence_rules(id) ON DELETE SET NULL,
                  source_task_id TEXT REFERENCES execution_tasks(id) ON DELETE SET NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT,
                  CHECK(
                    (is_all_day = 1
                      AND start_local_date IS NOT NULL
                      AND end_local_date IS NOT NULL
                      AND start_at IS NULL
                      AND end_at IS NULL
                      AND end_local_date >= start_local_date)
                    OR
                    (is_all_day = 0
                      AND start_at IS NOT NULL
                      AND end_at IS NOT NULL
                      AND end_at >= start_at
                      AND start_local_date IS NULL
                      AND end_local_date IS NULL)
                  )
                );
                CREATE INDEX IF NOT EXISTS idx_execution_calendar_events_timed
                  ON execution_calendar_events(user_id, start_at, end_at, status, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_calendar_events_all_day
                  ON execution_calendar_events(user_id, start_local_date, end_local_date, status, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_calendar_events_source_task
                  ON execution_calendar_events(source_task_id, deleted_at);

                CREATE TABLE IF NOT EXISTS execution_calendar_occurrences (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  event_id TEXT NOT NULL REFERENCES execution_calendar_events(id) ON DELETE CASCADE,
                  occurrence_key TEXT NOT NULL,
                  is_all_day INTEGER NOT NULL CHECK(is_all_day IN (0,1)),
                  start_at TEXT,
                  end_at TEXT,
                  start_local_date TEXT,
                  end_local_date TEXT,
                  status TEXT NOT NULL DEFAULT 'scheduled'
                    CHECK(status IN ('scheduled','cancelled','skipped')),
                  title_override TEXT,
                  description_override TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT,
                  CHECK(
                    (is_all_day = 1
                      AND start_local_date IS NOT NULL
                      AND end_local_date IS NOT NULL
                      AND start_at IS NULL
                      AND end_at IS NULL
                      AND end_local_date >= start_local_date)
                    OR
                    (is_all_day = 0
                      AND start_at IS NOT NULL
                      AND end_at IS NOT NULL
                      AND end_at >= start_at
                      AND start_local_date IS NULL
                      AND end_local_date IS NULL)
                  ),
                  UNIQUE(event_id, occurrence_key)
                );
                CREATE INDEX IF NOT EXISTS idx_execution_calendar_occurrences_event
                  ON execution_calendar_occurrences(event_id, occurrence_key, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_calendar_occurrences_timed
                  ON execution_calendar_occurrences(user_id, start_at, end_at, status, deleted_at);

                CREATE TABLE IF NOT EXISTS execution_memos (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  content TEXT NOT NULL CHECK(length(trim(content)) > 0),
                  plain_text TEXT NOT NULL CHECK(length(trim(plain_text)) > 0),
                  is_pinned INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0,1)),
                  status TEXT NOT NULL DEFAULT 'active'
                    CHECK(status IN ('active','archived')),
                  archived_at TEXT,
                  context TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT,
                  CHECK(status = 'archived' OR archived_at IS NULL)
                );
                CREATE INDEX IF NOT EXISTS idx_execution_memos_user_state
                  ON execution_memos(user_id, status, is_pinned, updated_at, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_memos_plain_text
                  ON execution_memos(plain_text);

                CREATE TABLE IF NOT EXISTS execution_memo_tags (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  name TEXT NOT NULL CHECK(length(trim(name)) > 0),
                  normalized_name TEXT NOT NULL CHECK(length(trim(normalized_name)) > 0),
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT
                );
                CREATE UNIQUE INDEX IF NOT EXISTS uq_execution_memo_tags_user_name
                  ON execution_memo_tags(user_id, normalized_name)
                  WHERE deleted_at IS NULL;

                CREATE TABLE IF NOT EXISTS execution_memo_tag_relations (
                  memo_id TEXT NOT NULL REFERENCES execution_memos(id) ON DELETE CASCADE,
                  tag_id TEXT NOT NULL REFERENCES execution_memo_tags(id) ON DELETE CASCADE,
                  created_at TEXT NOT NULL,
                  PRIMARY KEY(memo_id, tag_id)
                );
                CREATE INDEX IF NOT EXISTS idx_execution_memo_tag_relations_tag
                  ON execution_memo_tag_relations(tag_id, memo_id);

                CREATE TABLE IF NOT EXISTS execution_reminders (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  subject_type TEXT NOT NULL
                    CHECK(subject_type IN ('task','calendar_event','waiting_item','memo')),
                  subject_id TEXT NOT NULL,
                  trigger_at TEXT NOT NULL,
                  timezone TEXT,
                  status TEXT NOT NULL DEFAULT 'scheduled'
                    CHECK(status IN ('scheduled','fired','dismissed','cancelled')),
                  snoozed_until TEXT,
                  last_fired_at TEXT,
                  fire_key TEXT NOT NULL CHECK(length(trim(fire_key)) > 0),
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT
                );
                CREATE UNIQUE INDEX IF NOT EXISTS uq_execution_reminders_fire_key
                  ON execution_reminders(user_id, fire_key)
                  WHERE deleted_at IS NULL;
                CREATE INDEX IF NOT EXISTS idx_execution_reminders_due
                  ON execution_reminders(user_id, status, trigger_at, snoozed_until, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_reminders_subject
                  ON execution_reminders(subject_type, subject_id, status, deleted_at);

                CREATE TABLE IF NOT EXISTS execution_completion_results (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  task_id TEXT NOT NULL REFERENCES execution_tasks(id) ON DELETE CASCADE,
                  summary TEXT NOT NULL DEFAULT '',
                  completed_at TEXT NOT NULL,
                  actual_minutes INTEGER CHECK(actual_minutes IS NULL OR actual_minutes >= 0),
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT
                );
                CREATE UNIQUE INDEX IF NOT EXISTS uq_execution_completion_results_task
                  ON execution_completion_results(task_id)
                  WHERE deleted_at IS NULL;

                CREATE TABLE IF NOT EXISTS execution_entity_links (
                  id TEXT PRIMARY KEY,
                  user_id TEXT NOT NULL DEFAULT 'local',
                  source_type TEXT NOT NULL CHECK(length(trim(source_type)) > 0),
                  source_id TEXT NOT NULL CHECK(length(trim(source_id)) > 0),
                  relation_type TEXT NOT NULL
                    CHECK(relation_type IN ('related_to','derived_from','converted_to','attachment','reference')),
                  target_type TEXT NOT NULL CHECK(length(trim(target_type)) > 0),
                  target_id TEXT NOT NULL CHECK(length(trim(target_id)) > 0),
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT,
                  version INTEGER NOT NULL DEFAULT 1 CHECK(version >= 1),
                  modified_by_device TEXT,
                  CHECK(NOT (source_type = target_type AND source_id = target_id))
                );
                CREATE UNIQUE INDEX IF NOT EXISTS uq_execution_entity_links_active
                  ON execution_entity_links(user_id, source_type, source_id, relation_type, target_type, target_id)
                  WHERE deleted_at IS NULL;
                CREATE INDEX IF NOT EXISTS idx_execution_entity_links_source
                  ON execution_entity_links(source_type, source_id, relation_type, deleted_at);
                CREATE INDEX IF NOT EXISTS idx_execution_entity_links_target
                  ON execution_entity_links(target_type, target_id, relation_type, deleted_at);
                "#,
            )
            .map_err(|error| MigrationError {
                version: 9,
                message: format!("创建 EPIC-20 执行系统 Schema 失败: {error}"),
            })?;

        let mut report = MigrationReport::default();
        report.metrics.insert("execution_tables".to_owned(), 14);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn migrate() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let transaction = connection.transaction().unwrap();
        M0009Execution
            .up(&transaction, &MigrationContext::new(std::env::temp_dir()))
            .expect("execution migration should succeed");
        transaction.commit().unwrap();
        connection
    }

    #[test]
    fn creates_execution_schema_and_is_repeatable() {
        let mut connection = migrate();
        let expected = [
            "execution_projects",
            "execution_recurrence_rules",
            "execution_tasks",
            "execution_task_dependencies",
            "execution_task_occurrences",
            "execution_waiting_items",
            "execution_calendar_events",
            "execution_calendar_occurrences",
            "execution_memos",
            "execution_memo_tags",
            "execution_memo_tag_relations",
            "execution_reminders",
            "execution_completion_results",
            "execution_entity_links",
        ];
        for table in expected {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing table {table}");
        }

        let transaction = connection.transaction().unwrap();
        M0009Execution
            .up(&transaction, &MigrationContext::new(std::env::temp_dir()))
            .expect("execution migration should be repeatable");
        transaction.commit().unwrap();
    }

    #[test]
    fn enforces_task_dependency_and_schedule_constraints() {
        let connection = migrate();
        connection
            .execute(
                "INSERT INTO execution_tasks(id,title,created_at,updated_at) VALUES('t1','first','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_tasks(id,title,created_at,updated_at) VALUES('t2','second','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO execution_task_dependencies(id,task_id,depends_on_task_id,created_at) VALUES('d1','t1','t1','2026-08-08')",
                [],
            )
            .is_err());
        connection
            .execute(
                "INSERT INTO execution_task_dependencies(id,task_id,depends_on_task_id,created_at) VALUES('d2','t2','t1','2026-08-08')",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO execution_task_dependencies(id,task_id,depends_on_task_id,created_at) VALUES('d3','t2','t1','2026-08-08')",
                [],
            )
            .is_err());
        assert!(connection
            .execute(
                "INSERT INTO execution_tasks(id,title,scheduled_start_at,scheduled_end_at,created_at,updated_at) VALUES('bad','bad','2026-08-08T12:00:00Z','2026-08-08T11:00:00Z','2026-08-08','2026-08-08')",
                [],
            )
            .is_err());
    }

    #[test]
    fn recurring_occurrences_are_idempotent_per_series_key() {
        let connection = migrate();
        connection
            .execute(
                "INSERT INTO execution_tasks(id,title,created_at,updated_at) VALUES('series','daily','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_task_occurrences(id,task_id,occurrence_key,created_at,updated_at) VALUES('o1','series','2026-08-08','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO execution_task_occurrences(id,task_id,occurrence_key,created_at,updated_at) VALUES('o2','series','2026-08-08','2026-08-08','2026-08-08')",
                [],
            )
            .is_err());
    }

    #[test]
    fn all_day_and_timed_events_use_distinct_time_representations() {
        let connection = migrate();
        connection
            .execute(
                "INSERT INTO execution_calendar_events(id,title,is_all_day,start_local_date,end_local_date,created_at,updated_at) VALUES('all-day','holiday',1,'2026-08-08','2026-08-08','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_calendar_events(id,title,is_all_day,start_at,end_at,timezone,created_at,updated_at) VALUES('timed','meeting',0,'2026-08-08T01:00:00Z','2026-08-08T02:00:00Z','Asia/Shanghai','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO execution_calendar_events(id,title,is_all_day,start_at,end_at,created_at,updated_at) VALUES('mixed','bad',1,'2026-08-08T01:00:00Z','2026-08-08T02:00:00Z','2026-08-08','2026-08-08')",
                [],
            )
            .is_err());
    }

    #[test]
    fn memo_reminder_and_conversion_links_preserve_identity() {
        let connection = migrate();
        connection
            .execute(
                "INSERT INTO execution_memos(id,content,plain_text,created_at,updated_at) VALUES('m1','remember this','remember this','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_reminders(id,subject_type,subject_id,trigger_at,fire_key,created_at,updated_at) VALUES('r1','memo','m1','2026-08-09T00:00:00Z','memo:m1:20260809','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_tasks(id,title,created_at,updated_at) VALUES('t1','remember this','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_entity_links(id,source_type,source_id,relation_type,target_type,target_id,created_at,updated_at) VALUES('l1','memo','m1','converted_to','task','t1','2026-08-08','2026-08-08')",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO execution_entity_links(id,source_type,source_id,relation_type,target_type,target_id,created_at,updated_at) VALUES('l2','memo','m1','converted_to','task','t1','2026-08-08','2026-08-08')",
                [],
            )
            .is_err());
    }
}
