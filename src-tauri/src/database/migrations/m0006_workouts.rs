use rusqlite::{Connection, OptionalExtension, Transaction};
use serde_json::Value;

use crate::database::legacy::json_parser;
use crate::database::migration_runner::{Migration, MigrationContext, MigrationError, MigrationReport};
use crate::database::repositories::workouts;

const LEGACY_HISTORY_TABLE: &str = "legacy_workout_history_json_v1";
const LEGACY_IMPORTS_TABLE: &str = "legacy_workout_import_records_json_v1";
const LEGACY_TRAINING_NOTES_TABLE: &str = "legacy_training_notes_json_v1";

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

/// m0006：训记与训练摘要 schema 规范化。
pub struct M0006Workouts;

impl Migration for M0006Workouts {
    fn version(&self) -> i64 {
        6
    }

    fn name(&self) -> &'static str {
        "workouts-imports-normalization"
    }

    fn checksum(&self) -> &'static str {
        "m0006-workouts-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        rename_legacy_tables(transaction)?;
        create_normalized_tables(transaction)?;

        let legacy_history = read_legacy(transaction, LEGACY_HISTORY_TABLE)?;
        let legacy_imports = read_legacy(transaction, LEGACY_IMPORTS_TABLE)?;
        let legacy_training_notes = read_legacy(transaction, LEGACY_TRAINING_NOTES_TABLE)?;

        let mut workout_count = 0usize;
        let mut exercise_count = 0usize;
        let mut set_count = 0usize;
        for value in &legacy_history {
            let (row, exercises) =
                workouts::workout_from_legacy_json(transaction, value, Some(context), Some(transaction))?;
            set_count += exercises.iter().map(|exercise| exercise.sets.len()).sum::<usize>();
            exercise_count += exercises.len();
            workouts::upsert_workout(transaction, &row, &exercises)?;
            workout_count += 1;
        }

        let mut import_count = 0usize;
        for value in &legacy_imports {
            let row = workouts::import_from_legacy_json(transaction, value, Some(context), Some(transaction))?;
            workouts::upsert_import(transaction, &row)?;
            import_count += 1;
        }

        let mut training_note_count = 0usize;
        for value in &legacy_training_notes {
            let row = workouts::training_note_from_legacy_json(
                transaction,
                value,
                Some(context),
                Some(transaction),
            )?;
            workouts::upsert_training_note(transaction, &row)?;
            training_note_count += 1;
        }

        validate_workouts(
            transaction,
            &legacy_history,
            &legacy_imports,
            &legacy_training_notes,
        )?;

        let mut report = MigrationReport::default();
        report.migrated = workout_count + import_count + training_note_count;
        report.metrics.insert("workouts".to_owned(), workout_count as i64);
        report
            .metrics
            .insert("workout_exercises".to_owned(), exercise_count as i64);
        report.metrics.insert("workout_sets".to_owned(), set_count as i64);
        report
            .metrics
            .insert("workout_imports".to_owned(), import_count as i64);
        report
            .metrics
            .insert("training_notes".to_owned(), training_note_count as i64);
        Ok(report)
    }
}

fn read_legacy(connection: &Connection, table: &str) -> Result<Vec<Value>, MigrationError> {
    if table_exists(connection, table) {
        json_parser::read_json_rows(connection, table).map_err(|message| MigrationError {
            version: 6,
            message,
        })
    } else {
        Ok(Vec::new())
    }
}

fn rename_legacy_tables(connection: &Connection) -> Result<(), MigrationError> {
    for (source, legacy) in [
        ("workout_history", LEGACY_HISTORY_TABLE),
        ("workout_import_records", LEGACY_IMPORTS_TABLE),
        ("training_notes", LEGACY_TRAINING_NOTES_TABLE),
    ] {
        if table_exists(connection, source) && !table_exists(connection, legacy) {
            connection
                .execute(&format!("ALTER TABLE {source} RENAME TO {legacy}"), [])
                .map_err(|error| MigrationError {
                    version: 6,
                    message: format!("重命名 {source} 失败: {error}"),
                })?;
        }
    }
    Ok(())
}

fn create_normalized_tables(connection: &Connection) -> Result<(), MigrationError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS workouts (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               source TEXT NOT NULL DEFAULT 'manual',
               source_id TEXT,
               name TEXT NOT NULL DEFAULT '训练记录',
               occurred_at TEXT NOT NULL,
               local_date TEXT NOT NULL,
               duration_seconds INTEGER NOT NULL DEFAULT 0,
               exercise_count INTEGER NOT NULL DEFAULT 0,
               set_count INTEGER NOT NULL DEFAULT 0,
               planned_set_count INTEGER,
               volume_kg REAL,
               calories_kcal REAL,
               status TEXT,
               raw_json TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_workouts_date
               ON workouts(user_id, local_date, deleted_at);
             CREATE TABLE IF NOT EXISTS workout_exercises (
               id TEXT PRIMARY KEY,
               workout_id TEXT NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
               name TEXT NOT NULL,
               sort_order INTEGER NOT NULL DEFAULT 0,
               planned_sets INTEGER NOT NULL DEFAULT 0,
               completed_sets INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_workout_exercises_workout
               ON workout_exercises(workout_id);
             CREATE TABLE IF NOT EXISTS workout_sets (
               id TEXT PRIMARY KEY,
               exercise_id TEXT NOT NULL REFERENCES workout_exercises(id) ON DELETE CASCADE,
               set_number INTEGER NOT NULL,
               weight_kg REAL,
               reps INTEGER,
               completed INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_workout_sets_exercise
               ON workout_sets(exercise_id);
             CREATE TABLE IF NOT EXISTS workout_imports (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               source TEXT NOT NULL DEFAULT 'xunji',
               share_url TEXT,
               status TEXT NOT NULL DEFAULT 'pending',
               parser TEXT,
               parser_version TEXT,
               error TEXT,
               raw_json TEXT,
               workout_id TEXT REFERENCES workouts(id),
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_workout_imports_status
               ON workout_imports(status, deleted_at);
             CREATE TABLE IF NOT EXISTS training_notes (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               title TEXT NOT NULL DEFAULT '',
               content TEXT NOT NULL DEFAULT '',
               workout_id TEXT REFERENCES workouts(id),
               source TEXT NOT NULL DEFAULT 'xunji',
               note_date TEXT NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_training_notes_workout
               ON training_notes(workout_id);",
        )
        .map_err(|error| MigrationError {
            version: 6,
            message: format!("创建训练规范化表失败: {error}"),
        })
}

fn validate_workouts(
    connection: &Connection,
    legacy_history: &[Value],
    legacy_imports: &[Value],
    legacy_training_notes: &[Value],
) -> Result<(), MigrationError> {
    let counts: (i64, i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM workouts),
               (SELECT COUNT(*) FROM workout_imports),
               (SELECT COUNT(*) FROM training_notes),
               (SELECT COUNT(*) FROM workout_exercises)",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(|error| MigrationError { version: 6, message: error.to_string() })?;
    if counts.0 != legacy_history.len() as i64 {
        return Err(MigrationError { version: 6, message: format!("训练数量不一致: 旧 {}，新 {}", legacy_history.len(), counts.0) });
    }
    if counts.1 != legacy_imports.len() as i64 {
        return Err(MigrationError { version: 6, message: format!("导入记录数量不一致: 旧 {}，新 {}", legacy_imports.len(), counts.1) });
    }
    if counts.2 != legacy_training_notes.len() as i64 {
        return Err(MigrationError { version: 6, message: format!("训练笔记数量不一致: 旧 {}，新 {}", legacy_training_notes.len(), counts.2) });
    }
    let legacy_exercises = legacy_history
        .iter()
        .filter_map(|value| value.get("exercises").and_then(Value::as_array).map(Vec::len))
        .sum::<usize>();
    if counts.3 != legacy_exercises as i64 {
        return Err(MigrationError { version: 6, message: format!("动作数量不一致: 旧 {legacy_exercises}，新 {}", counts.3) });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::{M0001Framework, M0002Finance, M0003HabitsReviews, M0004Notes, M0005English};
    use crate::database::migration_runner::run;
    use rusqlite::Connection;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-workouts-{label}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn migrates_workout_history_imports_and_training_notes() {
        let directory = temp_dir("migrate");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE workout_history(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE workout_import_records(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE training_notes(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );",
            )
            .unwrap();
        let stamp = "2026-07-24T04:00:00+08:00";
        connection
            .execute(
                "INSERT INTO workout_history VALUES('w1', ?1, ?2)",
                rusqlite::params![
                    json!({
                        "id": "w1", "userId": "local-user", "templateId": "", "name": "练腿",
                        "occurredAt": stamp, "durationSeconds": 3600, "exerciseCount": 1,
                        "setCount": 2, "status": "completed", "source": "xunji", "sourceId": "s1",
                        "caloriesKcal": 300, "volumeKg": 1000,
                        "exercises": [{
                            "name": "深蹲", "plannedSets": 2, "completedSets": 2,
                            "sets": [{"weight": 80, "reps": 8, "completed": true},
                                     {"weight": 90, "reps": 6, "completed": true}]
                        }],
                        "createdAt": stamp, "updatedAt": stamp
                    })
                    .to_string(),
                    stamp
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO workout_import_records VALUES('i1', ?1, ?2)",
                rusqlite::params![
                    json!({
                        "id": "i1", "userId": "local-user", "source": "xunji",
                        "shareUrl": "https://api.xunjiapp.cn/app_share/x",
                        "rawData": {"exercises": []}, "workout": {"title": "练腿"},
                        "status": "success", "workoutRecordId": "w1",
                        "createdAt": stamp, "updatedAt": stamp
                    })
                    .to_string(),
                    stamp
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO training_notes VALUES('n1', ?1, ?2)",
                rusqlite::params![
                    json!({
                        "id": "n1", "userId": "local-user", "title": "训练记录",
                        "content": "内容", "workoutRecordId": "w1", "source": "xunji",
                        "noteDate": "2026-07-24", "createdAt": stamp, "updatedAt": stamp
                    })
                    .to_string(),
                    stamp
                ],
            )
            .unwrap();
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(M0001Framework),
            Box::new(M0002Finance),
            Box::new(M0003HabitsReviews),
            Box::new(M0004Notes),
            Box::new(M0005English),
            Box::new(M0006Workouts),
        ];
        run(&mut connection, &context, &migrations).unwrap();
        let workouts = workouts::list_workouts(&connection).unwrap();
        let imports = workouts::list_imports(&connection).unwrap();
        let notes = workouts::list_training_notes(&connection).unwrap();
        assert_eq!(workouts.len(), 1);
        assert_eq!(workouts[0]["setCount"], json!(2));
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0]["workoutRecordId"], json!("w1"));
        assert_eq!(notes.len(), 1);
        let exercise_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM workout_exercises", [], |row| row.get(0))
            .unwrap();
        let set_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM workout_sets", [], |row| row.get(0))
            .unwrap();
        assert_eq!(exercise_count, 1);
        assert_eq!(set_count, 2);
        fs::remove_dir_all(&directory).ok();
    }
}
