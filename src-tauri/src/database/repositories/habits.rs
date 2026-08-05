//! 习惯与每日复盘 Repository：真实列与前端 DTO 的转换与读写。

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::database::legacy::json_parser;
use crate::database::repositories::finance;

pub const DEFAULT_USER_ID: &str = "local";

const ACTIVITY_TYPES: [&str; 5] = ["duration", "count", "completion", "weekly", "control"];
const TARGET_PERIODS: [&str; 2] = ["daily", "weekly"];
const SCHEDULE_TYPES: [&str; 3] = ["daily", "weekly", "custom"];
const CHECKIN_METHODS: [&str; 2] = ["manual", "automatic"];
const SYNC_SOURCES: [&str; 2] = ["fitness", "english"];
const LOG_STATUSES: [&str; 3] = ["completed", "partial", "skipped"];

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn user_id(object: &serde_json::Map<String, Value>) -> String {
    json_parser::string_field(object, "userId")
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_USER_ID)
        .to_owned()
}

fn optional_text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    json_parser::string_field(object, key)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn optional_number(object: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    json_parser::number_field(object, key).filter(|value| value.is_finite())
}

/// 规范化习惯行。
#[derive(Debug, Clone)]
pub struct ActivityRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub activity_type: String,
    pub unit: String,
    pub minimum_target: Option<f64>,
    pub normal_target: Option<f64>,
    pub target_period: String,
    pub target_days_json: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub schedule_type: Option<String>,
    pub start_date: Option<String>,
    pub checkin_method: Option<String>,
    pub sync_source: Option<String>,
    pub description: Option<String>,
    pub is_archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}

/// 规范化打卡行。
#[derive(Debug, Clone)]
pub struct ActivityLogRow {
    pub id: String,
    pub user_id: String,
    pub activity_id: Option<String>,
    pub log_date: String,
    pub value: Option<f64>,
    pub status: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}

/// 规范化每日复盘行。
#[derive(Debug, Clone)]
pub struct DailyReviewRow {
    pub id: String,
    pub user_id: String,
    pub review_date: String,
    pub energy: Option<i64>,
    pub mood: Option<i64>,
    pub completion_score: Option<f64>,
    pub best_thing: Option<String>,
    pub problem: Option<String>,
    pub tomorrow_priority: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}

/// 旧 JSON 习惯 → 规范化行。
pub fn activity_from_legacy_json(value: &Value) -> Result<ActivityRow, String> {
    let object = json_parser::as_object(value, "习惯记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("习惯缺少 id: {}", value))?;
    let name = json_parser::string_field(object, "name")
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("习惯 {id} 缺少 name"))?;
    let activity_type = json_parser::string_field(object, "type")
        .ok_or_else(|| format!("习惯 {id} 缺少 type"))?
        .to_owned();
    if !ACTIVITY_TYPES.contains(&activity_type.as_str()) {
        return Err(format!("习惯 {id} 类型不合法: {activity_type}"));
    }
    let target_period = json_parser::string_field(object, "targetPeriod")
        .unwrap_or("daily")
        .to_owned();
    if !TARGET_PERIODS.contains(&target_period.as_str()) {
        return Err(format!("习惯 {id} 目标周期不合法: {target_period}"));
    }
    let schedule_type = optional_text(object, "scheduleType");
    if let Some(value) = &schedule_type {
        if !SCHEDULE_TYPES.contains(&value.as_str()) {
            return Err(format!("习惯 {id} 计划类型不合法: {value}"));
        }
    }
    let checkin_method = optional_text(object, "checkinMethod");
    if let Some(value) = &checkin_method {
        if !CHECKIN_METHODS.contains(&value.as_str()) {
            return Err(format!("习惯 {id} 打卡方式不合法: {value}"));
        }
    }
    let sync_source = optional_text(object, "syncSource");
    if let Some(value) = &sync_source {
        if !SYNC_SOURCES.contains(&value.as_str()) {
            return Err(format!("习惯 {id} 同步来源不合法: {value}"));
        }
    }
    let target_days_json = object
        .get("targetDays")
        .filter(|value| value.is_array())
        .map(|value| value.to_string());
    Ok(ActivityRow {
        id: id.to_owned(),
        user_id: user_id(object),
        name: name.to_owned(),
        activity_type,
        unit: optional_text(object, "unit").unwrap_or_default(),
        minimum_target: optional_number(object, "minimumTarget"),
        normal_target: optional_number(object, "normalTarget"),
        target_period,
        target_days_json,
        icon: optional_text(object, "icon"),
        color: optional_text(object, "color"),
        schedule_type,
        start_date: optional_text(object, "startDate"),
        checkin_method,
        sync_source,
        description: optional_text(object, "description"),
        is_archived: object
            .get("isArchived")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        created_at: optional_text(object, "createdAt").unwrap_or_else(now),
        updated_at: optional_text(object, "updatedAt").unwrap_or_else(now),
        deleted_at: None,
        version: 1,
        modified_by_device: None,
    })
}

fn activity_exists(connection: &Connection, id: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM activities WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
}

/// 旧 JSON 打卡 → 规范化行；孤立打卡置空 activity_id 并记录 issue。
pub fn activity_log_from_legacy_json(
    connection: &Connection,
    value: &Value,
    context: Option<&crate::database::migration_runner::MigrationContext>,
    transaction: Option<&Transaction>,
) -> Result<ActivityLogRow, String> {
    let object = json_parser::as_object(value, "打卡记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("打卡缺少 id: {}", value))?;
    let activity_id = json_parser::string_field(object, "activityId").map(str::to_owned);
    let resolved = activity_id
        .as_deref()
        .filter(|id| activity_exists(connection, id));
    if activity_id.is_some() && resolved.is_none() {
        let message = format!(
            "打卡 {id} 引用的习惯 {activity_id:?} 不存在，activity_id 置空（原始数据保留在 raw_json 与 JSON 表）"
        );
        if let (Some(transaction), Some(context)) = (transaction, context) {
            let _ = crate::database::migration_runner::record_issue(
                transaction,
                context,
                "activity_logs",
                Some(id),
                "warning",
                &message,
                Some(&value.to_string()),
            );
        }
    }
    let created_at = json_parser::string_field(object, "createdAt")
        .or_else(|| json_parser::string_field(object, "updatedAt"))
        .ok_or_else(|| format!("打卡 {id} 缺少 createdAt"))?;
    let log_date = finance::local_date_of(created_at)?;
    let status = optional_text(object, "status");
    if let Some(value) = &status {
        if !LOG_STATUSES.contains(&value.as_str()) {
            return Err(format!("打卡 {id} 状态不合法: {value}"));
        }
    }
    let metadata_json = object
        .get("metadata")
        .filter(|value| value.is_object())
        .map(|value| value.to_string());
    Ok(ActivityLogRow {
        id: id.to_owned(),
        user_id: user_id(object),
        activity_id: resolved.map(str::to_owned),
        log_date,
        value: optional_number(object, "value"),
        status,
        note: optional_text(object, "note"),
        metadata_json,
        created_at: created_at.to_owned(),
        updated_at: optional_text(object, "updatedAt").unwrap_or_else(now),
        deleted_at: None,
        version: 1,
        modified_by_device: None,
    })
}

fn normalize_energy_mood(
    object: &serde_json::Map<String, Value>,
    key: &str,
    entity_id: &str,
    context: Option<&crate::database::migration_runner::MigrationContext>,
    transaction: Option<&Transaction>,
) -> Option<i64> {
    let Some(value) = object.get(key).and_then(Value::as_i64) else {
        return None;
    };
    if (1..=10).contains(&value) {
        return Some(value);
    }
    if let (Some(transaction), Some(context)) = (transaction, context) {
        let _ = crate::database::migration_runner::record_issue(
            transaction,
            context,
            "daily_reviews",
            Some(entity_id),
            "warning",
            &format!("复盘 {entity_id} 的 {key}={value} 超出 1-10，已置空"),
            None,
        );
    }
    None
}

/// 旧 JSON 复盘 → 规范化行。
pub fn daily_review_from_legacy_json(
    value: &Value,
    context: Option<&crate::database::migration_runner::MigrationContext>,
    transaction: Option<&Transaction>,
) -> Result<DailyReviewRow, String> {
    let object = json_parser::as_object(value, "复盘记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("复盘缺少 id: {}", value))?;
    let review_date = json_parser::string_field(object, "reviewDate")
        .ok_or_else(|| format!("复盘 {id} 缺少 reviewDate"))?;
    if !review_date
        .chars()
        .all(|character| character.is_ascii_digit() || character == '-')
        || finance::local_date_of(review_date).is_err()
    {
        return Err(format!("复盘 {id} 日期不合法: {review_date}"));
    }
    let energy = normalize_energy_mood(object, "energy", id, context, transaction);
    let mood = normalize_energy_mood(object, "mood", id, context, transaction);
    Ok(DailyReviewRow {
        id: id.to_owned(),
        user_id: user_id(object),
        review_date: review_date.to_owned(),
        energy,
        mood,
        completion_score: optional_number(object, "completionScore"),
        best_thing: optional_text(object, "bestThing"),
        problem: optional_text(object, "problem"),
        tomorrow_priority: optional_text(object, "tomorrowPriority"),
        note: optional_text(object, "note"),
        created_at: optional_text(object, "createdAt").unwrap_or_else(now),
        updated_at: optional_text(object, "updatedAt").unwrap_or_else(now),
        deleted_at: None,
        version: 1,
        modified_by_device: None,
    })
}

/// 插入或更新习惯。
pub fn upsert_activity(connection: &Connection, row: &ActivityRow) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO activities(
               id, user_id, name, activity_type, unit, minimum_target, normal_target,
               target_period, target_days_json, icon, color, schedule_type, start_date,
               checkin_method, sync_source, description, is_archived, created_at, updated_at,
               deleted_at, version, modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)
             ON CONFLICT(id) DO UPDATE SET
               user_id=excluded.user_id, name=excluded.name, activity_type=excluded.activity_type,
               unit=excluded.unit, minimum_target=excluded.minimum_target,
               normal_target=excluded.normal_target, target_period=excluded.target_period,
               target_days_json=excluded.target_days_json, icon=excluded.icon,
               color=excluded.color, schedule_type=excluded.schedule_type,
               start_date=excluded.start_date, checkin_method=excluded.checkin_method,
               sync_source=excluded.sync_source, description=excluded.description,
               is_archived=excluded.is_archived, updated_at=excluded.updated_at,
               deleted_at=excluded.deleted_at, version=excluded.version,
               modified_by_device=excluded.modified_by_device",
            params![
                row.id,
                row.user_id,
                row.name,
                row.activity_type,
                row.unit,
                row.minimum_target,
                row.normal_target,
                row.target_period,
                row.target_days_json,
                row.icon,
                row.color,
                row.schedule_type,
                row.start_date,
                row.checkin_method,
                row.sync_source,
                row.description,
                row.is_archived,
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

/// 插入或更新打卡。
pub fn upsert_activity_log(connection: &Connection, row: &ActivityLogRow) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO activity_logs(
               id, user_id, activity_id, log_date, value, status, note, metadata_json,
               created_at, updated_at, deleted_at, version, modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(id) DO UPDATE SET
               user_id=excluded.user_id, activity_id=excluded.activity_id,
               log_date=excluded.log_date, value=excluded.value, status=excluded.status,
               note=excluded.note, metadata_json=excluded.metadata_json,
               updated_at=excluded.updated_at, deleted_at=excluded.deleted_at,
               version=excluded.version, modified_by_device=excluded.modified_by_device",
            params![
                row.id,
                row.user_id,
                row.activity_id,
                row.log_date,
                row.value,
                row.status,
                row.note,
                row.metadata_json,
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

/// 插入或更新复盘。
pub fn upsert_daily_review(connection: &Connection, row: &DailyReviewRow) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO daily_reviews(
               id, user_id, review_date, energy, mood, completion_score, best_thing,
               problem, tomorrow_priority, note, created_at, updated_at, deleted_at,
               version, modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
             ON CONFLICT(id) DO UPDATE SET
               user_id=excluded.user_id, review_date=excluded.review_date,
               energy=excluded.energy, mood=excluded.mood,
               completion_score=excluded.completion_score, best_thing=excluded.best_thing,
               problem=excluded.problem, tomorrow_priority=excluded.tomorrow_priority,
               note=excluded.note, updated_at=excluded.updated_at,
               deleted_at=excluded.deleted_at, version=excluded.version,
               modified_by_device=excluded.modified_by_device",
            params![
                row.id,
                row.user_id,
                row.review_date,
                row.energy,
                row.mood,
                row.completion_score,
                row.best_thing,
                row.problem,
                row.tomorrow_priority,
                row.note,
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

fn activity_to_dto(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let target_days_json: Option<String> = row.get(8)?;
    let target_days = target_days_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null);
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "userId": row.get::<_, String>(1)?,
        "name": row.get::<_, String>(2)?,
        "type": row.get::<_, String>(3)?,
        "unit": row.get::<_, String>(4)?,
        "minimumTarget": row.get::<_, Option<f64>>(5)?,
        "normalTarget": row.get::<_, Option<f64>>(6)?,
        "targetPeriod": row.get::<_, String>(7)?,
        "targetDays": target_days,
        "icon": row.get::<_, Option<String>>(9)?,
        "color": row.get::<_, Option<String>>(10)?,
        "scheduleType": row.get::<_, Option<String>>(11)?,
        "startDate": row.get::<_, Option<String>>(12)?,
        "checkinMethod": row.get::<_, Option<String>>(13)?,
        "syncSource": row.get::<_, Option<String>>(14)?,
        "description": row.get::<_, Option<String>>(15)?,
        "isArchived": row.get::<_, bool>(16)?,
        "createdAt": row.get::<_, String>(17)?,
        "updatedAt": row.get::<_, String>(18)?
    }))
}

/// 习惯列表（DTO）。
pub fn list_activities(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, user_id, name, activity_type, unit, minimum_target, normal_target,
                    target_period, target_days_json, icon, color, schedule_type, start_date,
                    checkin_method, sync_source, description, is_archived, created_at, updated_at
             FROM activities
             WHERE deleted_at IS NULL
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], activity_to_dto)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn activity_log_to_dto(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let metadata_json: Option<String> = row.get(7)?;
    let metadata = metadata_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null);
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "userId": row.get::<_, String>(1)?,
        "activityId": row.get::<_, Option<String>>(2)?,
        "value": row.get::<_, Option<f64>>(4)?,
        "status": row.get::<_, Option<String>>(5)?,
        "note": row.get::<_, Option<String>>(6)?,
        "metadata": metadata,
        "createdAt": row.get::<_, String>(8)?,
        "updatedAt": row.get::<_, String>(9)?
    }))
}

/// 打卡列表（DTO）。
pub fn list_activity_logs(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, user_id, activity_id, log_date, value, status, note, metadata_json,
                    created_at, updated_at
             FROM activity_logs
             WHERE deleted_at IS NULL
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], activity_log_to_dto)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn daily_review_to_dto(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "userId": row.get::<_, String>(1)?,
        "reviewDate": row.get::<_, String>(2)?,
        "energy": row.get::<_, Option<i64>>(3)?,
        "mood": row.get::<_, Option<i64>>(4)?,
        "completionScore": row.get::<_, Option<f64>>(5)?,
        "bestThing": row.get::<_, Option<String>>(6)?,
        "problem": row.get::<_, Option<String>>(7)?,
        "tomorrowPriority": row.get::<_, Option<String>>(8)?,
        "note": row.get::<_, Option<String>>(9)?,
        "createdAt": row.get::<_, String>(10)?,
        "updatedAt": row.get::<_, String>(11)?
    }))
}

/// 复盘列表（DTO）。
pub fn list_daily_reviews(connection: &Connection) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, user_id, review_date, energy, mood, completion_score, best_thing,
                    problem, tomorrow_priority, note, created_at, updated_at
             FROM daily_reviews
             WHERE deleted_at IS NULL
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], daily_review_to_dto)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn existing_version(connection: &Connection, table: &str, id: &str) -> Result<i64, String> {
    let value: Option<i64> = connection
        .query_row(
            &format!("SELECT version FROM {table} WHERE id=?1"),
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(value.unwrap_or(0))
}

/// 保存习惯 DTO（写路径）。
pub fn save_activity(connection: &Connection, dto: &Value) -> Result<(), String> {
    let object = json_parser::as_object(dto, "习惯数据")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut row = activity_from_legacy_json(dto)?;
    row.id = id.clone();
    row.version = existing_version(connection, "activities", &id)? + 1;
    row.updated_at = optional_text(object, "updatedAt").unwrap_or_else(now);
    upsert_activity(connection, &row)
}

/// 保存打卡 DTO（写路径）。
pub fn save_activity_log(connection: &Connection, dto: &Value) -> Result<(), String> {
    let object = json_parser::as_object(dto, "打卡数据")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut row = activity_log_from_legacy_json(connection, dto, None, None)?;
    row.id = id.clone();
    row.version = existing_version(connection, "activity_logs", &id)? + 1;
    row.updated_at = optional_text(object, "updatedAt").unwrap_or_else(now);
    upsert_activity_log(connection, &row)
}

/// 保存复盘 DTO（写路径）；同日已有其他活跃复盘时软删除旧记录。
pub fn save_daily_review(connection: &Connection, dto: &Value) -> Result<(), String> {
    let object = json_parser::as_object(dto, "复盘数据")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut row = daily_review_from_legacy_json(dto, None, None)?;
    row.id = id.clone();
    // 同日期其它活跃复盘软删除。
    connection
        .execute(
            "UPDATE daily_reviews SET deleted_at=?1, updated_at=?1
             WHERE user_id=?2 AND review_date=?3 AND id<>?4 AND deleted_at IS NULL",
            params![now(), row.user_id, row.review_date, id],
        )
        .map_err(|error| error.to_string())?;
    row.version = existing_version(connection, "daily_reviews", &id)? + 1;
    row.updated_at = optional_text(object, "updatedAt").unwrap_or_else(now);
    upsert_daily_review(connection, &row)
}

/// 恢复：事务内重建习惯、打卡与复盘（打卡必须晚于习惯）。
pub fn replace_all(
    transaction: &Transaction,
    activities: &[Value],
    logs: &[Value],
    reviews: &[Value],
) -> Result<(), String> {
    transaction
        .execute("DELETE FROM activity_logs", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM daily_reviews", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM activities", [])
        .map_err(|error| error.to_string())?;
    for activity in activities {
        let row = activity_from_legacy_json(activity)?;
        upsert_activity(transaction, &row)?;
    }
    for item in logs {
        let row = activity_log_from_legacy_json(transaction, item, None, None)?;
        upsert_activity_log(transaction, &row)?;
    }
    for review in reviews {
        let row = daily_review_from_legacy_json(review, None, None)?;
        upsert_daily_review(transaction, &row)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_conversion_roundtrip() {
        let value = json!({
            "id": "piano", "userId": "local-user", "name": "钢琴练习",
            "type": "duration", "unit": "分钟", "minimumTarget": 10, "normalTarget": 30,
            "targetPeriod": "daily", "icon": "music", "isArchived": false,
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        });
        let row = activity_from_legacy_json(&value).unwrap();
        assert_eq!(row.activity_type, "duration");
        assert_eq!(row.minimum_target, Some(10.0));
        assert_eq!(row.target_period, "daily");
    }

    #[test]
    fn invalid_activity_type_is_rejected() {
        let value = json!({"id": "x", "name": "x", "type": "bogus", "targetPeriod": "daily"});
        assert!(activity_from_legacy_json(&value).is_err());
    }

    #[test]
    fn log_date_derives_from_created_at() {
        let connection = Connection::open_in_memory().unwrap();
        let value = json!({
            "id": "log-1", "userId": "local-user", "activityId": "a1", "value": 15,
            "status": "completed", "createdAt": "2026-07-22T16:48:10.166Z",
            "updatedAt": "2026-07-22T16:48:10.166Z"
        });
        let row = activity_log_from_legacy_json(&connection, &value, None, None).unwrap();
        assert_eq!(row.log_date, "2026-07-22");
        assert_eq!(row.activity_id, None); // a1 不存在 → 置空
    }

    #[test]
    fn review_energy_out_of_range_becomes_null() {
        let value = json!({
            "id": "r1", "userId": "local-user", "reviewDate": "2026-07-22",
            "energy": 0, "mood": 7, "createdAt": "2026-07-22T00:00:00Z",
            "updatedAt": "2026-07-22T00:00:00Z"
        });
        let row = daily_review_from_legacy_json(&value, None, None).unwrap();
        assert_eq!(row.energy, None);
        assert_eq!(row.mood, Some(7));
    }
}
