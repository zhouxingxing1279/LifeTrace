use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::database::legacy::json_parser;
use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};
use crate::database::repositories::habits;

const LEGACY_ACTIVITIES_TABLE: &str = "legacy_activities_json_v1";
const LEGACY_LOGS_TABLE: &str = "legacy_activity_logs_json_v1";
const LEGACY_REVIEWS_TABLE: &str = "legacy_daily_reviews_json_v1";

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
}

/// m0003：习惯与每日复盘 schema 规范化。
pub struct M0003HabitsReviews;

impl Migration for M0003HabitsReviews {
    fn version(&self) -> i64 {
        3
    }

    fn name(&self) -> &'static str {
        "habits-reviews-normalization"
    }

    fn checksum(&self) -> &'static str {
        "m0003-habits-reviews-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        rename_legacy_tables(transaction)?;
        create_normalized_tables(transaction)?;

        let legacy_activities = if table_exists(transaction, LEGACY_ACTIVITIES_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_ACTIVITIES_TABLE)?
        } else {
            Vec::new()
        };
        let legacy_logs = if table_exists(transaction, LEGACY_LOGS_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_LOGS_TABLE)?
        } else {
            Vec::new()
        };
        let legacy_reviews = if table_exists(transaction, LEGACY_REVIEWS_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_REVIEWS_TABLE)?
        } else {
            Vec::new()
        };

        let mut activity_count = 0usize;
        for value in &legacy_activities {
            let row = habits::activity_from_legacy_json(value)?;
            habits::upsert_activity(transaction, &row)?;
            activity_count += 1;
        }

        let mut log_count = 0usize;
        for value in &legacy_logs {
            let row = habits::activity_log_from_legacy_json(
                transaction,
                value,
                Some(context),
                Some(transaction),
            )?;
            habits::upsert_activity_log(transaction, &row)?;
            log_count += 1;
        }

        // 同日多条复盘：按 updatedAt 倒序，最新一条为活跃记录，其余软删除并记录 issue。
        let mut reviews = legacy_reviews.clone();
        reviews.sort_by(|left, right| {
            let left_time = left
                .get("updatedAt")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let right_time = right
                .get("updatedAt")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            right_time.cmp(left_time)
        });
        let mut seen_dates = HashSet::<(String, String)>::new();
        let mut review_count = 0usize;
        for value in &reviews {
            let mut row =
                habits::daily_review_from_legacy_json(value, Some(context), Some(transaction))?;
            let key = (row.user_id.clone(), row.review_date.clone());
            if !seen_dates.insert(key.clone()) {
                // 重复：保留原始数据，标记软删除。
                let message = format!(
                    "日期 {} 存在多条复盘，保留最新一条（{}），其余软删除但保留原始数据",
                    row.review_date, row.id
                );
                crate::database::migration_runner::record_issue(
                    transaction,
                    context,
                    "daily_reviews",
                    Some(&row.id),
                    "warning",
                    &message,
                    Some(&value.to_string()),
                )?;
                row.deleted_at = Some(row.updated_at.clone());
            }
            habits::upsert_daily_review(transaction, &row)?;
            review_count += 1;
        }

        validate_habits(
            transaction,
            &legacy_activities,
            &legacy_logs,
            &legacy_reviews,
        )?;

        let mut report = MigrationReport::default();
        report.migrated = activity_count + log_count + review_count;
        report
            .metrics
            .insert("activities".to_owned(), activity_count as i64);
        report
            .metrics
            .insert("activity_logs".to_owned(), log_count as i64);
        report
            .metrics
            .insert("daily_reviews".to_owned(), review_count as i64);
        Ok(report)
    }
}

fn rename_legacy_tables(connection: &Connection) -> Result<(), MigrationError> {
    for (source, legacy) in [
        ("activities", LEGACY_ACTIVITIES_TABLE),
        ("activity_logs", LEGACY_LOGS_TABLE),
        ("daily_reviews", LEGACY_REVIEWS_TABLE),
    ] {
        if table_exists(connection, source) && !table_exists(connection, legacy) {
            connection
                .execute(&format!("ALTER TABLE {source} RENAME TO {legacy}"), [])
                .map_err(|error| MigrationError {
                    version: 3,
                    message: format!("重命名 {source} 失败: {error}"),
                })?;
        }
    }
    Ok(())
}

fn create_normalized_tables(connection: &Connection) -> Result<(), MigrationError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS activities (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               name TEXT NOT NULL,
               activity_type TEXT NOT NULL CHECK (activity_type IN (
                 'duration','count','completion','weekly','control'
               )),
               unit TEXT NOT NULL DEFAULT '',
               minimum_target REAL,
               normal_target REAL,
               target_period TEXT NOT NULL DEFAULT 'daily' CHECK (target_period IN ('daily','weekly')),
               target_days_json TEXT,
               icon TEXT,
               color TEXT,
               schedule_type TEXT CHECK (schedule_type IN ('daily','weekly','custom')),
               start_date TEXT,
               checkin_method TEXT CHECK (checkin_method IN ('manual','automatic')),
               sync_source TEXT CHECK (sync_source IN ('fitness','english')),
               description TEXT,
               is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE TABLE IF NOT EXISTS activity_logs (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               activity_id TEXT REFERENCES activities(id),
               log_date TEXT NOT NULL,
               value REAL,
               status TEXT CHECK (status IN ('completed','partial','skipped')),
               note TEXT,
               metadata_json TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_activity_logs_activity_date
               ON activity_logs(activity_id, log_date, deleted_at);
             CREATE INDEX IF NOT EXISTS idx_activity_logs_log_date
               ON activity_logs(log_date, deleted_at);
             CREATE TABLE IF NOT EXISTS daily_reviews (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               review_date TEXT NOT NULL,
               energy INTEGER CHECK (energy BETWEEN 1 AND 10),
               mood INTEGER CHECK (mood BETWEEN 1 AND 10),
               completion_score REAL,
               best_thing TEXT,
               problem TEXT,
               tomorrow_priority TEXT,
               note TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE UNIQUE INDEX IF NOT EXISTS uq_daily_reviews_date
               ON daily_reviews(user_id, review_date)
               WHERE deleted_at IS NULL;",
        )
        .map_err(|error| MigrationError {
            version: 3,
            message: format!("创建习惯/复盘规范化表失败: {error}"),
        })
}

fn validate_habits(
    connection: &Connection,
    legacy_activities: &[serde_json::Value],
    legacy_logs: &[serde_json::Value],
    legacy_reviews: &[serde_json::Value],
) -> Result<(), MigrationError> {
    let new_activities: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM activities WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|error| MigrationError {
            version: 3,
            message: error.to_string(),
        })?;
    let new_logs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM activity_logs WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|error| MigrationError {
            version: 3,
            message: error.to_string(),
        })?;
    let new_reviews: i64 = connection
        .query_row("SELECT COUNT(*) FROM daily_reviews", [], |row| row.get(0))
        .map_err(|error| MigrationError {
            version: 3,
            message: error.to_string(),
        })?;
    if new_activities != legacy_activities.len() as i64 {
        return Err(MigrationError {
            version: 3,
            message: format!(
                "习惯数量不一致: 旧 {} 条，新 {new_activities} 条",
                legacy_activities.len()
            ),
        });
    }
    if new_logs != legacy_logs.len() as i64 {
        return Err(MigrationError {
            version: 3,
            message: format!(
                "打卡数量不一致: 旧 {} 条，新 {new_logs} 条",
                legacy_logs.len()
            ),
        });
    }
    if new_reviews != legacy_reviews.len() as i64 {
        return Err(MigrationError {
            version: 3,
            message: format!(
                "复盘数量不一致: 旧 {} 条，新 {new_reviews} 条",
                legacy_reviews.len()
            ),
        });
    }
    // 孤立打卡：activity_id 为 NULL 且原 JSON 有 activityId 的记录必须已写入 issue。
    let orphan_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM activity_logs
             WHERE deleted_at IS NULL AND activity_id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|error| MigrationError {
            version: 3,
            message: error.to_string(),
        })?;
    let activity_ids: HashSet<String> = legacy_activities
        .iter()
        .filter_map(|value| {
            value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    let legacy_orphans = legacy_logs
        .iter()
        .filter(|value| {
            value
                .get("activityId")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| !activity_ids.contains(id))
        })
        .count();
    if orphan_count as usize != legacy_orphans {
        return Err(MigrationError {
            version: 3,
            message: format!("孤立打卡数量校验失败: 新 {orphan_count}，旧引用 {legacy_orphans}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::run;
    use crate::database::migrations::{M0001Framework, M0002Finance};
    use rusqlite::Connection;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-habits-{label}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn seed_legacy_json(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE activities(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE activity_logs(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE daily_reviews(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO activities VALUES('piano', ?1, '2026-01-01T00:00:00Z')",
                rusqlite::params![json!({
                    "id": "piano", "userId": "local-user", "name": "钢琴练习",
                    "type": "duration", "unit": "分钟", "minimumTarget": 10,
                    "normalTarget": 30, "targetPeriod": "daily", "icon": "music",
                    "isArchived": false, "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-01T00:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
        for (id, activity_id, created) in [
            ("log-1", "piano", "2026-07-22T16:48:10.166Z"),
            ("log-2", "missing-habit", "2026-07-23T16:48:10.166Z"),
        ] {
            connection
                .execute(
                    "INSERT INTO activity_logs VALUES(?1, ?2, ?3)",
                    rusqlite::params![
                        id,
                        json!({
                            "id": id, "userId": "local-user", "activityId": activity_id,
                            "value": 15, "status": "completed", "createdAt": created,
                            "updatedAt": created
                        })
                        .to_string(),
                        created
                    ],
                )
                .unwrap();
        }
        for (id, review_date, updated) in [
            ("r1", "2026-07-22", "2026-07-22T10:00:00Z"),
            ("r2", "2026-07-22", "2026-07-22T12:00:00Z"),
        ] {
            connection
                .execute(
                    "INSERT INTO daily_reviews VALUES(?1, ?2, ?3)",
                    rusqlite::params![
                        id,
                        json!({
                            "id": id, "userId": "local-user", "reviewDate": review_date,
                            "energy": 7, "mood": 7, "createdAt": updated, "updatedAt": updated
                        })
                        .to_string(),
                        updated
                    ],
                )
                .unwrap();
        }
    }

    #[test]
    fn migrates_habits_reviews_with_duplicates_and_orphans() {
        let directory = temp_dir("migrate");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        seed_legacy_json(&connection);
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(M0001Framework),
            Box::new(M0002Finance),
            Box::new(M0003HabitsReviews),
        ];
        let summary = run(&mut connection, &context, &migrations).unwrap();
        assert_eq!(summary.applied.len(), 3);

        let activities = habits::list_activities(&connection).unwrap();
        let logs = habits::list_activity_logs(&connection).unwrap();
        let reviews = habits::list_daily_reviews(&connection).unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(logs.len(), 2);
        // 同日重复复盘：只有最新一条活跃。
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0]["id"], json!("r2"));

        // 孤立打卡 activity_id 为 NULL，但记录仍在。
        let orphan: Option<String> = connection
            .query_row(
                "SELECT activity_id FROM activity_logs WHERE id='log-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(orphan, None);
        // issue 至少两条：孤立打卡 + 重复复盘。
        let issues: i64 = connection
            .query_row("SELECT COUNT(*) FROM migration_issues", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(issues >= 2, "issues = {issues}");
        // 重复复盘原始数据仍保留（软删除）。
        let r1_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM daily_reviews WHERE id='r1' AND deleted_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(r1_count, 1);

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn empty_database_migrates_cleanly() {
        let directory = temp_dir("empty");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(M0001Framework),
            Box::new(M0002Finance),
            Box::new(M0003HabitsReviews),
        ];
        run(&mut connection, &context, &migrations).unwrap();
        assert_eq!(habits::list_activities(&connection).unwrap().len(), 0);
        assert_eq!(habits::list_activity_logs(&connection).unwrap().len(), 0);
        assert_eq!(habits::list_daily_reviews(&connection).unwrap().len(), 0);
        fs::remove_dir_all(&directory).ok();
    }
}
