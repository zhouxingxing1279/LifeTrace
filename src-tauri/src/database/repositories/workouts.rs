//! 训记与训练摘要 Repository：真实列与前端 DTO 的转换与读写。
use crate::database::legacy::json_parser;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use uuid::Uuid;
pub const DEFAULT_USER_ID: &str = "local";
fn now() -> String {
    Utc::now().to_rfc3339()
}
fn text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
fn int_value(object: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    object.get(key).and_then(Value::as_i64)
}
fn real_value(object: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(Value::as_f64)
}
fn bool_value(object: &serde_json::Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}
fn user_id(object: &serde_json::Map<String, Value>) -> String {
    json_parser::string_field(object, "userId")
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_USER_ID)
        .to_owned()
}
#[derive(Debug, Clone)]
pub struct WorkoutSetRow {
    pub id: String,
    pub set_number: i64,
    pub weight_kg: Option<f64>,
    pub reps: Option<i64>,
    pub completed: bool,
}
#[derive(Debug, Clone)]
pub struct WorkoutExerciseRow {
    pub id: String,
    pub workout_id: String,
    pub name: String,
    pub sort_order: i64,
    pub planned_sets: i64,
    pub completed_sets: i64,
    pub sets: Vec<WorkoutSetRow>,
}
#[derive(Debug, Clone)]
pub struct WorkoutRow {
    pub id: String,
    pub user_id: String,
    pub source: String,
    pub source_id: Option<String>,
    pub name: String,
    pub occurred_at: String,
    pub local_date: String,
    pub duration_seconds: i64,
    pub exercise_count: i64,
    pub set_count: i64,
    pub planned_set_count: Option<i64>,
    pub volume_kg: Option<f64>,
    pub calories_kcal: Option<f64>,
    pub status: Option<String>,
    pub raw_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}
pub fn workout_from_legacy_json(
    _connection: &Connection,
    value: &Value,
    _context: Option<&crate::database::migration_runner::MigrationContext>,
    _transaction: Option<&rusqlite::Transaction>,
) -> Result<(WorkoutRow, Vec<WorkoutExerciseRow>), String> {
    let object = json_parser::as_object(value, "训练记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("训练记录缺少 id: {}", value))?;
    let occurred_at = json_parser::string_field(object, "occurredAt")
        .ok_or_else(|| format!("训练记录 {id} 缺少 occurredAt"))?;
    let local_date = crate::database::repositories::finance::local_date_of(occurred_at)?;
    let exercise_values = object
        .get("exercises")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut exercises = Vec::new();
    for (index, exercise) in exercise_values.iter().enumerate() {
        let exercise_object = exercise.as_object().cloned().unwrap_or_default();
        let name = text(&exercise_object, "name").unwrap_or_else(|| format!("动作 {}", index + 1));
        let sets_values = exercise
            .get("sets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut sets = Vec::new();
        for (set_index, set) in sets_values.iter().enumerate() {
            let set_object = set.as_object().cloned().unwrap_or_default();
            sets.push(WorkoutSetRow {
                id: Uuid::new_v4().to_string(),
                set_number: (set_index + 1) as i64,
                weight_kg: real_value(&set_object, "weight"),
                reps: int_value(&set_object, "reps"),
                completed: bool_value(&set_object, "completed"),
            });
        }
        exercises.push(WorkoutExerciseRow {
            id: Uuid::new_v4().to_string(),
            workout_id: id.to_owned(),
            name,
            sort_order: index as i64,
            planned_sets: int_value(&exercise_object, "plannedSets")
                .or_else(|| Some(sets.len() as i64))
                .unwrap_or(0),
            completed_sets: int_value(&exercise_object, "completedSets")
                .or_else(|| Some(sets.len() as i64))
                .unwrap_or(0),
            sets,
        });
    }
    let set_count: i64 = exercises
        .iter()
        .map(|exercise| exercise.sets.len() as i64)
        .sum();
    let source = json_parser::string_field(object, "source").unwrap_or("manual");
    Ok((
        WorkoutRow {
            id: id.to_owned(),
            user_id: user_id(object),
            source: source.to_owned(),
            source_id: text(object, "sourceId"),
            name: text(object, "name").unwrap_or_else(|| "训练记录".to_owned()),
            occurred_at: occurred_at.to_owned(),
            local_date,
            duration_seconds: int_value(object, "durationSeconds").unwrap_or(0),
            exercise_count: exercises.len() as i64,
            set_count,
            planned_set_count: int_value(object, "plannedSetCount"),
            volume_kg: real_value(object, "volumeKg"),
            calories_kcal: real_value(object, "caloriesKcal"),
            status: text(object, "status"),
            raw_json: Some(value.to_string()),
            created_at: text(object, "createdAt").unwrap_or_else(now),
            updated_at: text(object, "updatedAt").unwrap_or_else(now),
            deleted_at: None,
            version: int_value(object, "version").unwrap_or(1).max(1),
            modified_by_device: None,
        },
        exercises,
    ))
}
pub fn upsert_workout(
    connection: &Connection,
    row: &WorkoutRow,
    exercises: &[WorkoutExerciseRow],
) -> Result<(), String> {
    connection
        .execute(
            "INSERT OR REPLACE INTO workouts(
           id, user_id, source, source_id, name, occurred_at, local_date, duration_seconds,
           exercise_count, set_count, planned_set_count, volume_kg, calories_kcal, status,
           raw_json, created_at, updated_at, deleted_at, version, modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                row.id,
                row.user_id,
                row.source,
                row.source_id,
                row.name,
                row.occurred_at,
                row.local_date,
                row.duration_seconds,
                row.exercise_count,
                row.set_count,
                row.planned_set_count,
                row.volume_kg,
                row.calories_kcal,
                row.status,
                row.raw_json,
                row.created_at,
                row.updated_at,
                row.deleted_at,
                row.version,
                row.modified_by_device
            ],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "DELETE FROM workout_exercises WHERE workout_id = ?1",
            [&row.id],
        )
        .map_err(|error| error.to_string())?;
    for exercise in exercises {
        connection
            .execute(
                "INSERT OR REPLACE INTO workout_exercises(
               id, workout_id, name, sort_order, planned_sets, completed_sets
             ) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    exercise.id,
                    exercise.workout_id,
                    exercise.name,
                    exercise.sort_order,
                    exercise.planned_sets,
                    exercise.completed_sets
                ],
            )
            .map_err(|error| error.to_string())?;
        for set in &exercise.sets {
            connection
                .execute(
                    "INSERT OR REPLACE INTO workout_sets(
                   id, exercise_id, set_number, weight_kg, reps, completed
                 ) VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        set.id,
                        exercise.id,
                        set.set_number,
                        set.weight_kg,
                        set.reps,
                        set.completed
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
fn exercise_dto(connection: &Connection, exercise_id: &str) -> Result<Value, String> {
    let row: Option<(String, String, i64, i64, i64)> = connection
        .query_row(
            "SELECT id, name, sort_order, planned_sets, completed_sets
             FROM workout_exercises WHERE id = ?1",
            [exercise_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (id, name, _sort_order, planned_sets, completed_sets) =
        row.ok_or_else(|| "动作不存在".to_owned())?;
    let mut statement = connection
        .prepare(
            "SELECT id, set_number, weight_kg, reps, completed
             FROM workout_sets WHERE exercise_id = ?1 ORDER BY set_number",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([id.clone()], |row| {
            Ok(json!({
                "weight": row.get::<_, Option<f64>>(2)?,
                "reps": row.get::<_, Option<i64>>(3)?,
                "completed": row.get::<_, bool>(4)?
            }))
        })
        .map_err(|error| error.to_string())?;
    let sets = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "name": name,
        "plannedSets": planned_sets,
        "completedSets": completed_sets,
        "sets": sets
    }))
}
pub fn list_workouts(connection: &Connection) -> Result<Vec<Value>, String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, user_id, source, source_id, name, occurred_at, local_date,
                    duration_seconds, exercise_count, set_count, planned_set_count, volume_kg,
                    calories_kcal, status, created_at, updated_at
             FROM workouts WHERE deleted_at IS NULL AND user_id=?1 ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let mut rows = statement
        .query([profile_id])
        .map_err(|error| error.to_string())?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        items.push(workout_from_row(connection, &row)?);
    }
    Ok(items)
}
fn workout_from_row(connection: &Connection, row: &rusqlite::Row<'_>) -> Result<Value, String> {
    let workout_id: String = row.get(0).map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare("SELECT id FROM workout_exercises WHERE workout_id = ?1 ORDER BY sort_order")
        .map_err(|error| error.to_string())?;
    let mut exercise_rows = statement
        .query([workout_id.clone()])
        .map_err(|error| error.to_string())?;
    let mut exercises = Vec::new();
    while let Some(exercise_row) = exercise_rows.next().map_err(|error| error.to_string())? {
        let exercise_id: String = exercise_row.get(0).map_err(|error| error.to_string())?;
        exercises.push(exercise_dto(connection, &exercise_id)?);
    }
    Ok(json!({
        "id": workout_id,
        "userId": row.get::<_, String>(1).map_err(|error| error.to_string())?,
        "templateId": "",
        "name": row.get::<_, String>(4).map_err(|error| error.to_string())?,
        "occurredAt": row.get::<_, String>(5).map_err(|error| error.to_string())?,
        "local_date": row.get::<_, String>(6).map_err(|error| error.to_string())?,
        "durationSeconds": row.get::<_, i64>(7).map_err(|error| error.to_string())?,
        "exerciseCount": row.get::<_, i64>(8).map_err(|error| error.to_string())?,
        "setCount": row.get::<_, i64>(9).map_err(|error| error.to_string())?,
        "plannedSetCount": row.get::<_, Option<i64>>(10).map_err(|error| error.to_string())?,
        "source": row.get::<_, String>(2).map_err(|error| error.to_string())?,
        "sourceId": row.get::<_, Option<String>>(3).map_err(|error| error.to_string())?,
        "caloriesKcal": row.get::<_, Option<f64>>(12).map_err(|error| error.to_string())?,
        "volumeKg": row.get::<_, Option<f64>>(11).map_err(|error| error.to_string())?,
        "status": row.get::<_, Option<String>>(13).map_err(|error| error.to_string())?,
        "exercises": exercises,
        "createdAt": row.get::<_, String>(14).map_err(|error| error.to_string())?,
        "updatedAt": row.get::<_, String>(15).map_err(|error| error.to_string())?
    }))
}
pub fn get_workout(connection: &Connection, id: &str) -> Result<Option<Value>, String> {
    Ok(list_workouts(connection)?
        .into_iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(id)))
}
pub fn save_workout(connection: &Connection, dto: &Value) -> Result<(), String> {
    let owned = crate::database::profile::assign_active_owner(connection, dto)?;
    let dto = &owned;
    let object = json_parser::as_object(dto, "训练记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut value = dto.clone();
    if let Some(object) = value.as_object_mut() {
        object.insert("id".to_owned(), json!(id));
        object.insert("updatedAt".to_owned(), json!(now()));
    }
    let (mut row, exercises) = workout_from_legacy_json(connection, &value, None, None)?;
    let existing_version: Option<i64> = connection
        .query_row("SELECT version FROM workouts WHERE id=?1", [&id], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| error.to_string())?;
    row.version = existing_version.unwrap_or(0) + 1;
    upsert_workout(connection, &row, &exercises)
}
pub fn delete_workout(connection: &Connection, id: &str) -> Result<(), String> {
    connection
        .execute(
            "UPDATE workouts SET deleted_at=?1, updated_at=?1, version=version+1 WHERE id=?2",
            params![now(), id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}
#[derive(Debug, Clone)]
pub struct WorkoutImportRow {
    pub id: String,
    pub user_id: String,
    pub source: String,
    pub share_url: Option<String>,
    pub status: String,
    pub parser: Option<String>,
    pub parser_version: Option<String>,
    pub error: Option<String>,
    pub raw_json: Option<String>,
    pub workout_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}
pub fn import_from_legacy_json(
    connection: &Connection,
    value: &Value,
    context: Option<&crate::database::migration_runner::MigrationContext>,
    transaction: Option<&rusqlite::Transaction>,
) -> Result<WorkoutImportRow, String> {
    let object = json_parser::as_object(value, "训练导入记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("导入记录缺少 id: {}", value))?;
    let workout_id = text(object, "workoutRecordId");
    if workout_id.is_some()
        && connection
            .query_row(
                "SELECT 1 FROM workouts WHERE id=?1",
                [workout_id.as_deref().unwrap_or_default()],
                |_| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_none()
    {
        if let (Some(transaction), Some(context)) = (transaction, context) {
            let _ = crate::database::migration_runner::record_issue(
                transaction,
                context,
                "workout_imports",
                Some(id),
                "warning",
                &format!("导入记录 {id} 引用的训练 {workout_id:?} 不存在，workout_id 置空"),
                Some(&value.to_string()),
            );
        }
    }
    let raw = value
        .get("rawData")
        .cloned()
        .unwrap_or_else(|| value.get("workout").cloned().unwrap_or(Value::Null));
    Ok(WorkoutImportRow {
        id: id.to_owned(),
        user_id: user_id(object),
        source: json_parser::string_field(object, "source")
            .unwrap_or("xunji")
            .to_owned(),
        share_url: text(object, "shareUrl"),
        status: json_parser::string_field(object, "status")
            .unwrap_or("pending")
            .to_owned(),
        parser: text(object, "parser"),
        parser_version: text(object, "parserVersion"),
        error: text(object, "error"),
        raw_json: if raw.is_null() {
            None
        } else {
            Some(raw.to_string())
        },
        workout_id: workout_id.filter(|id| {
            connection
                .query_row("SELECT 1 FROM workouts WHERE id=?1", [id], |_| Ok(()))
                .optional()
                .ok()
                .flatten()
                .is_some()
        }),
        created_at: text(object, "createdAt").unwrap_or_else(now),
        updated_at: text(object, "updatedAt").unwrap_or_else(now),
        deleted_at: None,
        version: int_value(object, "version").unwrap_or(1).max(1),
        modified_by_device: None,
    })
}
pub fn upsert_import(connection: &Connection, row: &WorkoutImportRow) -> Result<(), String> {
    connection
        .execute(
            "INSERT OR REPLACE INTO workout_imports(
           id, user_id, source, share_url, status, parser, parser_version, error, raw_json,
           workout_id, created_at, updated_at, deleted_at, version, modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                row.id,
                row.user_id,
                row.source,
                row.share_url,
                row.status,
                row.parser,
                row.parser_version,
                row.error,
                row.raw_json,
                row.workout_id,
                row.created_at,
                row.updated_at,
                row.deleted_at,
                row.version,
                row.modified_by_device
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}
fn import_dto(connection: &Connection, row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let raw_json: Option<String> = row.get(8)?;
    let raw = raw_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null);
    let workout_id: Option<String> = row.get(9)?;
    let workout = workout_id
        .as_deref()
        .and_then(|id| get_workout(connection, id).ok())
        .flatten();
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "userId": row.get::<_, String>(1)?,
        "source": row.get::<_, String>(2)?,
        "shareUrl": row.get::<_, Option<String>>(3)?,
        "rawData": raw,
        "workout": workout,
        "status": row.get::<_, String>(4)?,
        "parser": row.get::<_, Option<String>>(5)?,
        "parserVersion": row.get::<_, Option<String>>(6)?,
        "error": row.get::<_, Option<String>>(7)?,
        "workoutRecordId": workout_id,
        "createdAt": row.get::<_, String>(10)?,
        "updatedAt": row.get::<_, String>(11)?
    }))
}
pub fn list_imports(connection: &Connection) -> Result<Vec<Value>, String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, user_id, source, share_url, status, parser, parser_version, error,
                    raw_json, workout_id, created_at, updated_at
             FROM workout_imports WHERE deleted_at IS NULL AND user_id=?1 ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([profile_id], |row| import_dto(connection, row))
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
pub fn get_import(connection: &Connection, id: &str) -> Result<Option<Value>, String> {
    Ok(list_imports(connection)?
        .into_iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(id)))
}
pub fn save_import(connection: &Connection, dto: &Value) -> Result<(), String> {
    let owned = crate::database::profile::assign_active_owner(connection, dto)?;
    let dto = &owned;
    let mut value = dto.clone();
    if let Some(object) = value.as_object_mut() {
        object.insert("updatedAt".to_owned(), json!(now()));
    }
    let row = import_from_legacy_json(connection, &value, None, None)?;
    upsert_import(connection, &row)
}
#[derive(Debug, Clone)]
pub struct TrainingNoteRow {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub workout_id: Option<String>,
    pub source: String,
    pub note_date: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}
pub fn training_note_from_legacy_json(
    connection: &Connection,
    value: &Value,
    context: Option<&crate::database::migration_runner::MigrationContext>,
    transaction: Option<&rusqlite::Transaction>,
) -> Result<TrainingNoteRow, String> {
    let object = json_parser::as_object(value, "训练笔记")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("训练笔记缺少 id: {}", value))?;
    let workout_id = text(object, "workoutRecordId");
    if workout_id.is_some()
        && connection
            .query_row(
                "SELECT 1 FROM workouts WHERE id=?1",
                [workout_id.as_deref().unwrap_or_default()],
                |_| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_none()
    {
        if let (Some(transaction), Some(context)) = (transaction, context) {
            let _ = crate::database::migration_runner::record_issue(
                transaction,
                context,
                "training_notes",
                Some(id),
                "warning",
                &format!("训练笔记 {id} 引用的训练 {workout_id:?} 不存在，workout_id 置空"),
                Some(&value.to_string()),
            );
        }
    }
    Ok(TrainingNoteRow {
        id: id.to_owned(),
        user_id: user_id(object),
        title: text(object, "title").unwrap_or_default(),
        content: text(object, "content").unwrap_or_default(),
        workout_id: workout_id.filter(|id| {
            connection
                .query_row("SELECT 1 FROM workouts WHERE id=?1", [id], |_| Ok(()))
                .optional()
                .ok()
                .flatten()
                .is_some()
        }),
        source: json_parser::string_field(object, "source")
            .unwrap_or("xunji")
            .to_owned(),
        note_date: text(object, "noteDate").unwrap_or_else(now),
        created_at: text(object, "createdAt").unwrap_or_else(now),
        updated_at: text(object, "updatedAt").unwrap_or_else(now),
        deleted_at: None,
        version: int_value(object, "version").unwrap_or(1).max(1),
        modified_by_device: None,
    })
}
pub fn upsert_training_note(connection: &Connection, row: &TrainingNoteRow) -> Result<(), String> {
    connection
        .execute(
            "INSERT OR REPLACE INTO training_notes(
           id, user_id, title, content, workout_id, source, note_date, created_at, updated_at,
           deleted_at, version, modified_by_device
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                row.id,
                row.user_id,
                row.title,
                row.content,
                row.workout_id,
                row.source,
                row.note_date,
                row.created_at,
                row.updated_at,
                row.deleted_at,
                row.version,
                row.modified_by_device
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}
pub fn save_training_note(connection: &Connection, dto: &Value) -> Result<(), String> {
    let owned = crate::database::profile::assign_active_owner(connection, dto)?;
    let dto = &owned;
    let row = training_note_from_legacy_json(connection, dto, None, None)?;
    upsert_training_note(connection, &row)
}
/// 恢复：事务内重建训练数据。
pub fn replace_all(transaction: &rusqlite::Transaction, items: &[Value]) -> Result<(), String> {
    transaction
        .execute("DELETE FROM workout_sets", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM workout_exercises", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM workout_imports", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM training_notes", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM workouts", [])
        .map_err(|error| error.to_string())?;
    for item in items {
        save_workout(transaction, item)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workout_dto_roundtrip() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE workouts(
               id TEXT PRIMARY KEY, user_id TEXT, source TEXT, source_id TEXT, name TEXT,
               occurred_at TEXT, local_date TEXT, duration_seconds INTEGER, exercise_count INTEGER,
               set_count INTEGER, planned_set_count INTEGER, volume_kg REAL, calories_kcal REAL,
               status TEXT, raw_json TEXT, created_at TEXT, updated_at TEXT, deleted_at TEXT,
               version INTEGER, modified_by_device TEXT
             );
             CREATE TABLE workout_exercises(
               id TEXT PRIMARY KEY, workout_id TEXT, name TEXT, sort_order INTEGER,
               planned_sets INTEGER, completed_sets INTEGER
             );
             CREATE TABLE workout_sets(
               id TEXT PRIMARY KEY, exercise_id TEXT, set_number INTEGER, weight_kg REAL,
               reps INTEGER, completed INTEGER
             );
             CREATE TABLE workout_imports(
               id TEXT PRIMARY KEY, user_id TEXT, source TEXT, share_url TEXT, status TEXT,
               parser TEXT, parser_version TEXT, error TEXT, raw_json TEXT, workout_id TEXT,
               created_at TEXT, updated_at TEXT, deleted_at TEXT, version INTEGER,
               modified_by_device TEXT
             );
             CREATE TABLE training_notes(
               id TEXT PRIMARY KEY, user_id TEXT, title TEXT, content TEXT, workout_id TEXT,
               source TEXT, note_date TEXT, created_at TEXT, updated_at TEXT, deleted_at TEXT,
               version INTEGER, modified_by_device TEXT
             );",
            )
            .unwrap();
        let stamp = "2026-07-24T04:00:00+08:00";
        let dto = json!({
            "id": "w1", "userId": "local-user", "name": "练腿", "occurredAt": stamp,
            "durationSeconds": 3600, "exerciseCount": 1, "setCount": 2, "status": "completed",
            "source": "xunji", "sourceId": "s1", "caloriesKcal": 300, "volumeKg": 1000,
            "exercises": [{
                "name": "深蹲", "plannedSets": 2, "completedSets": 2,
                "sets": [{"weight": 80, "reps": 8, "completed": true}, {"weight": 90, "reps": 6, "completed": true}]
            }],
            "createdAt": stamp, "updatedAt": stamp
        });
        save_workout(&connection, &dto).unwrap();
        let items = list_workouts(&connection).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["local_date"], json!("2026-07-24"));
        assert_eq!(
            items[0]["exercises"][0]["sets"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(items[0]["setCount"], json!(2));
    }
}
