use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

use super::AppState;

const DEFAULT_MODEL: &str = "deepseek-v4-flash";
const DEEPSEEK_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

#[derive(Clone, Copy)]
enum DatasetKind {
    Json,
    Photos,
    PhotoDevices,
    PhotoTasks,
}

#[derive(Clone, Copy)]
struct Dataset {
    key: &'static str,
    label: &'static str,
    table: &'static str,
    kind: DatasetKind,
}

const DATASETS: &[Dataset] = &[
    Dataset {
        key: "activities",
        label: "坚持项目",
        table: "activities",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "activity_logs",
        label: "坚持记录",
        table: "activity_logs",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "transactions",
        label: "账单",
        table: "transactions",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "daily_reviews",
        label: "每日复盘",
        table: "daily_reviews",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "finance_accounts",
        label: "财务账户",
        table: "finance_accounts",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "workout_history",
        label: "训练历史",
        table: "workouts",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "workout_import_records",
        label: "训练导入",
        table: "workout_imports",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "training_notes",
        label: "训练笔记",
        table: "training_notes",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "notes",
        label: "笔记",
        table: "notes",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "note_folders",
        label: "笔记文件夹",
        table: "note_folders",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "note_tags",
        label: "笔记标签",
        table: "note_tags",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "note_revisions",
        label: "笔记版本",
        table: "note_revisions",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "english_articles",
        label: "英语文章",
        table: "english_articles",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "english_learning_records",
        label: "英语学习记录",
        table: "english_learning_records",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "english_vocabulary",
        label: "英语生词",
        table: "english_vocabulary",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "english_highlights",
        label: "英语高亮",
        table: "english_highlights",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "english_notes",
        label: "英语笔记",
        table: "english_notes",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "english_analysis",
        label: "英语分析",
        table: "english_ai_analysis",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "english_sources",
        label: "英语来源",
        table: "english_sources",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "import_uploads",
        label: "导入记录",
        table: "import_uploads",
        kind: DatasetKind::Json,
    },
    Dataset {
        key: "photos",
        label: "照片与视频",
        table: "photos",
        kind: DatasetKind::Photos,
    },
    Dataset {
        key: "photo_devices",
        label: "照片同步设备",
        table: "photo_sync_devices",
        kind: DatasetKind::PhotoDevices,
    },
    Dataset {
        key: "photo_upload_tasks",
        label: "照片同步任务",
        table: "photo_upload_tasks",
        kind: DatasetKind::PhotoTasks,
    },
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    api_key: Option<String>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct ChatMessageInput {
    role: String,
    content: String,
}

#[derive(Deserialize)]
pub struct ChatInput {
    messages: Vec<ChatMessageInput>,
}

#[derive(Deserialize)]
pub struct ConversationQuery {
    id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationInput {
    id: String,
    title: String,
    messages: Value,
}

fn failure(status: StatusCode, message: impl Into<String>, code: &str) -> Response {
    (
        status,
        Json(json!({ "error": message.into(), "code": code })),
    )
        .into_response()
}

fn valid_model(value: &str) -> Option<&str> {
    match value {
        "deepseek-v4-flash" | "deepseek-v4-pro" => Some(value),
        _ => None,
    }
}

fn read_settings(connection: &Connection) -> Result<Option<Value>, String> {
    let raw = connection
        .query_row(
            "SELECT data_json FROM ai_settings WHERE id='deepseek'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    raw.map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS ai_settings(
           id TEXT PRIMARY KEY,
           data_json TEXT NOT NULL,
           updated_at TEXT NOT NULL
         )",
        [],
    )?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS ai_conversations(
           id TEXT PRIMARY KEY,
           title TEXT NOT NULL,
           messages_json TEXT NOT NULL,
           created_at TEXT NOT NULL,
           updated_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS ai_conversations_updated_at_idx
           ON ai_conversations(updated_at DESC);",
    )?;
    Ok(())
}

pub async fn conversations_get(
    State(state): State<AppState>,
    Query(query): Query<ConversationQuery>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLite 锁已损坏",
                "STORAGE",
            )
        }
    };
    if let Some(id) = query.id {
        let item = connection
            .query_row(
                "SELECT id,title,messages_json,created_at,updated_at FROM ai_conversations WHERE id=?1",
                [id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
            )
            .optional();
        return match item {
            Ok(Some((id, title, raw, created_at, updated_at))) => match serde_json::from_str::<Value>(&raw) {
                Ok(messages) => Json(json!({ "item": { "id": id, "title": title, "messages": messages, "createdAt": created_at, "updatedAt": updated_at } })).into_response(),
                Err(error) => failure(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), "STORAGE"),
            },
            Ok(None) => failure(StatusCode::NOT_FOUND, "没有找到该历史会话", "NOT_FOUND"),
            Err(error) => failure(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), "STORAGE"),
        };
    }
    let mut statement = match connection.prepare(
        "SELECT id,title,messages_json,created_at,updated_at FROM ai_conversations ORDER BY updated_at DESC LIMIT 50",
    ) {
        Ok(value) => value,
        Err(error) => return failure(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), "STORAGE"),
    };
    let rows = match statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }) {
        Ok(value) => value,
        Err(error) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
                "STORAGE",
            )
        }
    };
    let mut items = Vec::new();
    for row in rows {
        let (id, title, raw, created_at, updated_at) = match row {
            Ok(value) => value,
            Err(error) => {
                return failure(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error.to_string(),
                    "STORAGE",
                )
            }
        };
        let message_count = serde_json::from_str::<Value>(&raw)
            .ok()
            .and_then(|value| value.as_array().map(Vec::len))
            .unwrap_or(0);
        items.push(json!({ "id": id, "title": title, "messageCount": message_count, "createdAt": created_at, "updatedAt": updated_at }));
    }
    Json(json!({ "items": items })).into_response()
}

pub async fn conversations_save(
    State(state): State<AppState>,
    Json(input): Json<ConversationInput>,
) -> Response {
    let id = input.id.trim();
    let title = input.title.trim();
    let Some(messages) = input.messages.as_array() else {
        return failure(
            StatusCode::BAD_REQUEST,
            "会话消息格式无效",
            "INVALID_REQUEST",
        );
    };
    if id.is_empty()
        || id.len() > 100
        || title.is_empty()
        || title.chars().count() > 80
        || messages.len() > 200
    {
        return failure(
            StatusCode::BAD_REQUEST,
            "会话内容超出限制",
            "INVALID_REQUEST",
        );
    }
    let now = chrono::Utc::now().to_rfc3339();
    let raw = Value::Array(messages.clone()).to_string();
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLite 锁已损坏",
                "STORAGE",
            )
        }
    };
    match connection.execute(
        "INSERT INTO ai_conversations(id,title,messages_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?4)
         ON CONFLICT(id) DO UPDATE SET title=excluded.title,messages_json=excluded.messages_json,updated_at=excluded.updated_at",
        params![id, title, raw, now],
    ) {
        Ok(_) => Json(json!({ "ok": true, "id": id, "updatedAt": now })).into_response(),
        Err(error) => failure(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), "STORAGE"),
    }
}

pub async fn conversations_remove(
    State(state): State<AppState>,
    Query(query): Query<ConversationQuery>,
) -> Response {
    let Some(id) = query.id.filter(|value| !value.trim().is_empty()) else {
        return failure(StatusCode::BAD_REQUEST, "缺少会话 ID", "INVALID_REQUEST");
    };
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLite 锁已损坏",
                "STORAGE",
            )
        }
    };
    match connection.execute("DELETE FROM ai_conversations WHERE id=?1", [id]) {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(error) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
            "STORAGE",
        ),
    }
}

pub async fn settings_get(State(state): State<AppState>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLite 锁已损坏",
                "STORAGE",
            )
        }
    };
    match read_settings(&connection) {
        Ok(settings) => {
            let model = settings
                .as_ref()
                .and_then(|value| value.get("model"))
                .and_then(Value::as_str)
                .and_then(valid_model)
                .unwrap_or(DEFAULT_MODEL);
            Json(json!({
                "provider": "deepseek",
                "model": model,
                "configured": settings.as_ref().is_some_and(|value| {
                    value.get("apiKey").and_then(Value::as_str).is_some_and(|key| !key.is_empty())
                }),
                "updatedAt": settings.and_then(|value| value.get("updatedAt").cloned())
            }))
            .into_response()
        }
        Err(message) => failure(StatusCode::INTERNAL_SERVER_ERROR, message, "STORAGE"),
    }
}

pub async fn settings_save(
    State(state): State<AppState>,
    Json(input): Json<SettingsInput>,
) -> Response {
    let model = input.model.as_deref().unwrap_or(DEFAULT_MODEL);
    let Some(model) = valid_model(model) else {
        return failure(
            StatusCode::BAD_REQUEST,
            "不支持的 DeepSeek 模型",
            "INVALID_MODEL",
        );
    };
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLite 锁已损坏",
                "STORAGE",
            )
        }
    };
    let previous = read_settings(&connection).ok().flatten();
    let api_key = input
        .api_key
        .map(|value| value.trim().chars().take(300).collect::<String>())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            previous
                .as_ref()
                .and_then(|value| value.get("apiKey"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let Some(api_key) = api_key else {
        return failure(
            StatusCode::BAD_REQUEST,
            "请填写 DeepSeek API Key",
            "INVALID_REQUEST",
        );
    };
    let updated_at = chrono::Utc::now().to_rfc3339();
    let value = json!({ "apiKey": api_key, "model": model, "updatedAt": updated_at });
    if let Err(error) = connection.execute(
        "INSERT INTO ai_settings(id,data_json,updated_at) VALUES('deepseek',?1,?2)
         ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at",
        params![value.to_string(), updated_at],
    ) {
        return failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
            "STORAGE",
        );
    }
    Json(json!({ "provider": "deepseek", "model": model, "configured": true, "updatedAt": updated_at })).into_response()
}

pub async fn settings_remove(State(state): State<AppState>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLite 锁已损坏",
                "STORAGE",
            )
        }
    };
    match connection.execute("DELETE FROM ai_settings WHERE id='deepseek'", []) {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(error) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
            "STORAGE",
        ),
    }
}

fn dataset(key: &str) -> Option<Dataset> {
    DATASETS.iter().copied().find(|item| item.key == key)
}

fn count_dataset(connection: &Connection, source: Dataset) -> Result<i64, String> {
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {}", source.table),
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn catalog_value(connection: &Connection) -> Result<Value, String> {
    let mut groups = Vec::new();
    for source in DATASETS {
        groups.push(json!({
            "key": source.key,
            "label": source.label,
            "count": count_dataset(connection, *source)?
        }));
    }
    Ok(json!({ "datasets": groups, "readOnly": true }))
}

pub async fn catalog(State(state): State<AppState>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLite 锁已损坏",
                "STORAGE",
            )
        }
    };
    match catalog_value(&connection) {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(StatusCode::INTERNAL_SERVER_ERROR, message, "STORAGE"),
    }
}

fn json_records(
    connection: &Connection,
    source: Dataset,
    query: &str,
    date: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<Value>, String> {
    let query_pattern = if query.is_empty() {
        String::new()
    } else {
        format!("%{}%", query.to_lowercase())
    };
    let date_pattern = if date.is_empty() {
        String::new()
    } else {
        format!("{date}%")
    };
    let normalized_expression = normalized_json_expression(source.key);
    let (select_expression, date_expression) = if let Some(expression) = normalized_expression {
        (
            expression.to_owned(),
            normalized_date_expression(source.key).to_owned(),
        )
    } else {
        let date_expression = match source.key {
            "activity_logs" => "json_extract(data_json,'$.createdAt')",
            "transactions" => "json_extract(data_json,'$.occurredAt')",
            "daily_reviews" => "json_extract(data_json,'$.reviewDate')",
            "workout_history" => "json_extract(data_json,'$.occurredAt')",
            "workout_import_records" | "english_highlights" | "english_notes" | "english_analysis" | "import_uploads" => "json_extract(data_json,'$.createdAt')",
            "training_notes" => "COALESCE(json_extract(data_json,'$.noteDate'),json_extract(data_json,'$.createdAt'))",
            "english_articles" => "COALESCE(json_extract(data_json,'$.publishedAt'),json_extract(data_json,'$.createdTime'))",
            "english_learning_records" => "json_extract(data_json,'$.date')",
            "note_revisions" => "json_extract(data_json,'$.createdAt')",
            _ => "COALESCE(json_extract(data_json,'$.updatedAt'),json_extract(data_json,'$.createdAt'))",
        };
        ("data_json".to_owned(), date_expression.to_owned())
    };
    let search_clause = if normalized_expression.is_some() {
        format!("lower({select_expression}) LIKE ?2")
    } else {
        "lower(data_json) LIKE ?2".to_owned()
    };
    let sql = format!(
        "SELECT {select_expression} FROM {} WHERE (?1='' OR {search_clause}) AND (?3='' OR {date_expression} LIKE ?4) ORDER BY updated_at DESC LIMIT ?5 OFFSET ?6",
        source.table,
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            params![
                query,
                query_pattern,
                date,
                date_pattern,
                limit as i64,
                offset as i64
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| error.to_string())?;
    rows.map(|row| {
        let raw = row.map_err(|error| error.to_string())?;
        serde_json::from_str(&raw).map_err(|error| error.to_string())
    })
    .collect()
}

/// 规范化表的 JSON 视图表达式（用于个人数据目录查询）。
fn normalized_json_expression(key: &str) -> Option<&'static str> {
    match key {
        "workout_history" => Some("json_object('id',id,'name',name,'occurredAt',occurred_at,'durationSeconds',duration_seconds,'source',source,'sourceId',source_id,'createdAt',created_at,'updatedAt',updated_at)"),
        "workout_import_records" => Some("json_object('id',id,'source',source,'shareUrl',share_url,'status',status,'workoutRecordId',workout_id,'createdAt',created_at,'updatedAt',updated_at)"),
        "training_notes" => Some("json_object('id',id,'title',title,'content',content,'noteDate',note_date,'workoutRecordId',workout_id,'createdAt',created_at,'updatedAt',updated_at)"),
        "activities" => Some("json_object('id',id,'name',name,'type',activity_type,'unit',unit,'isArchived',is_archived,'createdAt',created_at,'updatedAt',updated_at)"),
        "activity_logs" => Some("json_object('id',id,'activityId',activity_id,'value',value,'status',status,'createdAt',created_at,'updatedAt',updated_at)"),
        "transactions" => Some("json_object('id',id,'type',transaction_type,'amount',amount_cents/100.0,'category',COALESCE(legacy_category_name,''),'account',COALESCE(legacy_account_name,''),'occurredAt',occurred_at,'createdAt',created_at,'updatedAt',updated_at)"),
        "daily_reviews" => Some("json_object('id',id,'reviewDate',review_date,'energy',energy,'mood',mood,'createdAt',created_at,'updatedAt',updated_at)"),
        "finance_accounts" => Some("json_object('id',id,'name',name,'type',account_type,'balance',opening_balance_cents/100.0,'createdAt',created_at,'updatedAt',updated_at)"),
        "notes" => Some("json_object('id',id,'title',title,'noteType',note_type,'summary',summary,'isArchived',is_archived,'createdAt',created_at,'updatedAt',updated_at)"),
        "note_folders" => Some("json_object('id',id,'name',name,'createdAt',created_at,'updatedAt',updated_at)"),
        "note_tags" => Some("json_object('id',id,'name',name,'createdAt',created_at,'updatedAt',updated_at)"),
        "note_revisions" => Some("json_object('id',id,'noteId',note_id,'version',revision_version,'createdAt',created_at)"),
        "english_articles" => Some("json_object('id',id,'title',title,'level',level,'category',category,'wordCount',word_count,'createdTime',created_time,'createdAt',created_at,'updatedAt',updated_at)"),
        "english_learning_records" => Some("json_object('id',id,'date',record_date,'articleId',article_id,'readingTimeSeconds',reading_time_seconds,'summary',summary,'createdAt',created_at,'updatedAt',updated_at)"),
        "english_vocabulary" => Some("json_object('id',id,'word',display_word,'normalizedWord',normalized_word,'status',status,'nextReviewAt',next_review_at,'createdAt',created_at,'updatedAt',updated_at)"),
        "english_highlights" => Some("json_object('id',id,'articleId',article_id,'text',selected_text,'createdAt',created_at,'updatedAt',updated_at)"),
        "english_notes" => Some("json_object('id',id,'articleId',article_id,'content',content,'createdAt',created_at,'updatedAt',updated_at)"),
        "english_analysis" => Some("json_object('id',id,'recordId',record_id,'articleId',article_id,'score',score,'createdAt',created_at,'updatedAt',updated_at)"),
        _ => None,
    }
}

/// 规范化表的日期表达式（真实列）。
fn normalized_date_expression(key: &str) -> &'static str {
    match key {
        "workout_history" => "occurred_at",
        "workout_import_records" => "created_at",
        "training_notes" => "COALESCE(note_date, created_at)",
        "activity_logs" => "created_at",
        "transactions" => "occurred_at",
        "daily_reviews" => "review_date",
        "finance_accounts" => "created_at",
        "notes" | "note_folders" | "note_tags" => "created_at",
        "note_revisions" => "created_at",
        "english_articles" => "COALESCE(published_at, created_time, created_at)",
        "english_learning_records" => "record_date",
        "english_vocabulary" | "english_highlights" | "english_notes" | "english_analysis" => {
            "created_at"
        }
        _ => "updated_at",
    }
}

fn trim_value(value: &mut Value, string_limit: usize, array_limit: usize, depth: usize) {
    if depth > 10 {
        *value = Value::String("[内容层级过深，已省略]".to_owned());
        return;
    }
    match value {
        Value::String(text) => {
            if text.chars().count() > string_limit {
                let mut shortened = text.chars().take(string_limit).collect::<String>();
                shortened.push_str("…[已截断，可按具体记录继续查询]");
                *text = shortened;
            }
        }
        Value::Array(items) => {
            items.truncate(array_limit);
            for item in items {
                trim_value(item, string_limit, array_limit, depth + 1);
            }
        }
        Value::Object(object) => {
            for item in object.values_mut() {
                trim_value(item, string_limit, array_limit, depth + 1);
            }
        }
        _ => {}
    }
}

fn compact_record(key: &str, mut value: Value, detail: &str) -> Value {
    if detail != "full" {
        if let Some(object) = value.as_object_mut() {
            for field in ["contentJson", "contentHtml", "contentMarkdown", "rawData"] {
                object.remove(field);
            }
            if key == "english_articles" {
                for field in ["content", "questions", "vocabulary"] {
                    object.remove(field);
                }
            }
            if key == "note_revisions" {
                object.remove("contentText");
            }
        }
        trim_value(&mut value, 800, 24, 0);
    } else {
        trim_value(&mut value, 12_000, 100, 0);
    }
    value
}

fn photo_records(
    connection: &Connection,
    source: Dataset,
    query: &str,
    date: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<Value>, String> {
    let (select, search_column, date_column, order_column) = match source.kind {
        DatasetKind::Photos => (
            "SELECT id,original_file_name,media_type,mime_type,file_size,width,height,duration_ms,captured_at,imported_at,processing_status,processing_error,source_device_id,deleted_at FROM photos",
            "original_file_name",
            "COALESCE(captured_at,imported_at)",
            "COALESCE(captured_at,imported_at)",
        ),
        DatasetKind::PhotoDevices => (
            "SELECT id,device_name,device_type,status,paired_at,last_seen_at,revoked_at FROM photo_sync_devices",
            "device_name",
            "paired_at",
            "paired_at",
        ),
        DatasetKind::PhotoTasks => (
            "SELECT id,device_id,original_file_name,media_type,mime_type,captured_at,expected_file_size,received_file_size,status,photo_id,created_at,updated_at,expires_at,error_code,error_message,is_duplicate FROM photo_upload_tasks",
            "original_file_name",
            "COALESCE(captured_at,created_at)",
            "updated_at",
        ),
        DatasetKind::Json => return Err("数据类型错误".to_owned()),
    };
    let query_pattern = if query.is_empty() {
        String::new()
    } else {
        format!("%{}%", query.to_lowercase())
    };
    let date_pattern = if date.is_empty() {
        String::new()
    } else {
        format!("{date}%")
    };
    let sql = format!(
        "{select} WHERE (?1='' OR lower({search_column}) LIKE ?2) AND (?3='' OR {date_column} LIKE ?4) ORDER BY {order_column} DESC LIMIT ?5 OFFSET ?6"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let column_count = statement.column_count();
    let names = statement
        .column_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let rows = statement
        .query_map(
            params![
                query,
                query_pattern,
                date,
                date_pattern,
                limit as i64,
                offset as i64
            ],
            move |row| {
                let mut object = Map::new();
                for index in 0..column_count {
                    let name = names[index].clone();
                    let value = match row.get_ref(index)? {
                        rusqlite::types::ValueRef::Null => Value::Null,
                        rusqlite::types::ValueRef::Integer(value) => json!(value),
                        rusqlite::types::ValueRef::Real(value) => json!(value),
                        rusqlite::types::ValueRef::Text(value) => {
                            Value::String(String::from_utf8_lossy(value).into_owned())
                        }
                        rusqlite::types::ValueRef::Blob(_) => Value::String("[binary]".to_owned()),
                    };
                    object.insert(name, value);
                }
                Ok(Value::Object(object))
            },
        )
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string()))
        .collect()
}

fn read_dataset(
    connection: &Connection,
    key: &str,
    query: &str,
    date: &str,
    limit: usize,
    offset: usize,
    detail: &str,
) -> Result<Value, String> {
    let source = dataset(key).ok_or_else(|| format!("未知数据集：{key}"))?;
    let limit = if detail == "full" {
        limit.clamp(1, 10)
    } else {
        limit.clamp(1, 60)
    };
    let records = match source.kind {
        DatasetKind::Json => json_records(connection, source, query, date, limit, offset)?,
        _ => photo_records(connection, source, query, date, limit, offset)?,
    }
    .into_iter()
    .map(|value| compact_record(key, value, detail))
    .collect::<Vec<_>>();
    Ok(json!({
        "dataset": source.key,
        "label": source.label,
        "records": records,
        "returned": records.len(),
        "total": count_dataset(connection, source)?,
        "limit": limit,
        "offset": offset
    }))
}

fn day_snapshot(connection: &Connection, date: &str) -> Result<Value, String> {
    if date.len() != 10
        || !date.chars().enumerate().all(|(index, value)| {
            matches!(index, 4 | 7) && value == '-'
                || !matches!(index, 4 | 7) && value.is_ascii_digit()
        })
    {
        return Err("日期必须使用 YYYY-MM-DD 格式".to_owned());
    }
    let mut result = Map::new();
    result.insert("date".to_owned(), json!(date));
    for key in [
        "activity_logs",
        "transactions",
        "daily_reviews",
        "workout_history",
        "notes",
        "english_learning_records",
        "english_highlights",
        "english_notes",
        "photos",
    ] {
        let value = read_dataset(connection, key, "", date, 40, 0, "summary")?;
        result.insert(
            key.to_owned(),
            value.get("records").cloned().unwrap_or_else(|| json!([])),
        );
    }
    result.insert(
        "activities".to_owned(),
        read_dataset(connection, "activities", "", "", 60, 0, "summary")?
            .get("records")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    Ok(Value::Object(result))
}

fn tools() -> Value {
    let keys = DATASETS
        .iter()
        .map(|source| Value::String(source.key.to_owned()))
        .collect::<Vec<_>>();
    json!([
        {
            "type": "function",
            "function": {
                "name": "get_data_catalog",
                "description": "查看 LifeTrace 可访问的数据集以及每个数据集的记录数量。",
                "strict": true,
                "parameters": { "type": "object", "properties": {}, "additionalProperties": false }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_day_snapshot",
                "description": "读取某一天跨坚持、账单、复盘、训练、笔记、英语与照片的完整生活快照。总结某天时优先使用。",
                "strict": true,
                "parameters": {
                    "type": "object",
                    "properties": { "date": { "type": "string", "description": "YYYY-MM-DD 格式的本地日期" } },
                    "required": ["date"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_life_data",
                "description": "只读检索指定 LifeTrace 数据集。可用日期或关键词缩小范围，需要更多记录时使用 offset 翻页。",
                "strict": true,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "dataset": { "type": "string", "enum": keys },
                        "query": { "type": "string", "description": "可选的关键词" },
                        "date": { "type": "string", "description": "可选的 YYYY-MM-DD 或 YYYY-MM 日期前缀" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 60 },
                        "detail": { "type": "string", "enum": ["summary", "full"], "description": "默认 summary；只有需要核对具体原文时才用 full，full 单次最多 10 条" },
                        "offset": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["dataset"],
                    "additionalProperties": false
                }
            }
        }
    ])
}

fn execute_tool(
    connection: &Connection,
    name: &str,
    arguments: &str,
) -> Result<(Value, Vec<String>), String> {
    let input: Value =
        serde_json::from_str(arguments).map_err(|_| "AI 工具参数格式无效".to_owned())?;
    match name {
        "get_data_catalog" => Ok((catalog_value(connection)?, Vec::new())),
        "get_day_snapshot" => {
            let date = input
                .get("date")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let used = [
                "activity_logs",
                "transactions",
                "daily_reviews",
                "workout_history",
                "notes",
                "english_learning_records",
                "english_highlights",
                "english_notes",
                "photos",
                "activities",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect();
            Ok((day_snapshot(connection, date)?, used))
        }
        "read_life_data" => {
            let key = input
                .get("dataset")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let query = input
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let date = input
                .get("date")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let limit = input.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            let offset = input.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let detail = input
                .get("detail")
                .and_then(Value::as_str)
                .unwrap_or("summary");
            Ok((
                read_dataset(connection, key, query, date, limit, offset, detail)?,
                vec![key.to_owned()],
            ))
        }
        _ => Err("AI 请求了未知工具".to_owned()),
    }
}

fn system_prompt() -> String {
    let date = chrono::Local::now().format("%Y-%m-%d");
    format!(
        "你是 LifeTrace 的 AI 管家，一个克制、可靠、尊重隐私的中文个人生活助理。今天的本地日期是 {date}。\n\
你可以通过只读工具访问用户的坚持、训练、财务、复盘、笔记、英语学习、照片元数据和导入记录。回答涉及个人事实时必须先调用工具，不得猜测或虚构记录。\n\
需要总结一天时使用 get_day_snapshot；不知道有哪些数据时使用 get_data_catalog；分析趋势时按需使用 read_life_data，并说明样本范围。相关性不等于因果。\n\
节省用户费用：先用 summary、最小可用 limit 和明确的日期或关键词检索；只有回答必须核对原文时才使用 full；不要重复调用相同工具；不要读取与问题无关的数据集。\n\
照片工具只返回元数据，不能看懂照片画面；不要声称看到了图片内容。不要暴露内部工具名、数据库字段或 API 密钥。\n\
你只有读取权限。如果用户要求修改、删除或创建数据，应明确说明当前管家不会代为写入，并给出用户可执行的步骤。回答尽量简洁、具体、可行动。"
    )
}

pub async fn chat(State(state): State<AppState>, Json(input): Json<ChatInput>) -> Response {
    if input.messages.is_empty() || input.messages.len() > 20 {
        return failure(
            StatusCode::BAD_REQUEST,
            "对话记录数量无效",
            "INVALID_REQUEST",
        );
    }
    let settings = {
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => {
                return failure(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "SQLite 锁已损坏",
                    "STORAGE",
                )
            }
        };
        match read_settings(&connection) {
            Ok(Some(value)) => value,
            _ => {
                return failure(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "请先在设置中配置 DeepSeek API Key",
                    "NOT_CONFIGURED",
                )
            }
        }
    };
    let api_key = settings
        .get("apiKey")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let model = settings
        .get("model")
        .and_then(Value::as_str)
        .and_then(valid_model)
        .unwrap_or(DEFAULT_MODEL)
        .to_owned();
    if api_key.is_empty() {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "请先在设置中配置 DeepSeek API Key",
            "NOT_CONFIGURED",
        );
    }

    let mut messages = vec![json!({ "role": "system", "content": system_prompt() })];
    for message in input
        .messages
        .into_iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if !matches!(message.role.as_str(), "user" | "assistant") {
            continue;
        }
        let maximum = if message.role == "user" { 2_500 } else { 4_500 };
        let content = message
            .content
            .trim()
            .chars()
            .take(maximum)
            .collect::<String>();
        if !content.is_empty() {
            messages.push(json!({ "role": message.role, "content": content }));
        }
    }
    if messages.len() == 1
        || messages
            .last()
            .and_then(|value| value.get("role"))
            .and_then(Value::as_str)
            != Some("user")
    {
        return failure(
            StatusCode::BAD_REQUEST,
            "最后一条消息必须来自用户",
            "INVALID_REQUEST",
        );
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
    {
        Ok(value) => value,
        Err(error) => {
            return failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
                "CLIENT",
            )
        }
    };
    let mut used = BTreeSet::new();
    let mut tool_requests = BTreeSet::new();
    for _ in 0..6 {
        let response = client
            .post(DEEPSEEK_ENDPOINT)
            .bearer_auth(&api_key)
            .json(&json!({
                "model": model,
                "messages": messages,
                "tools": tools(),
                "tool_choice": "auto",
                "thinking": { "type": "disabled" },
                "max_tokens": 1000
            }))
            .send()
            .await;
        let response = match response {
            Ok(value) => value,
            Err(_) => {
                return failure(
                    StatusCode::BAD_GATEWAY,
                    "当前无法连接 DeepSeek 服务",
                    "UPSTREAM_UNAVAILABLE",
                )
            }
        };
        let status = response.status();
        let payload: Value = match response.json().await {
            Ok(value) => value,
            Err(_) => {
                return failure(
                    StatusCode::BAD_GATEWAY,
                    "DeepSeek 返回了无效响应",
                    "UPSTREAM_INVALID",
                )
            }
        };
        if !status.is_success() {
            let message = payload
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("DeepSeek 请求失败");
            return failure(StatusCode::BAD_GATEWAY, message, "UPSTREAM_ERROR");
        }
        let usage = payload.get("usage").cloned().unwrap_or(Value::Null);
        let Some(message) = payload.pointer("/choices/0/message").cloned() else {
            return failure(
                StatusCode::BAD_GATEWAY,
                "DeepSeek 响应缺少消息内容",
                "UPSTREAM_INVALID",
            );
        };
        let tool_calls = message
            .get("tool_calls")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if tool_calls.is_empty() {
            let content = message
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if content.is_empty() {
                return failure(
                    StatusCode::BAD_GATEWAY,
                    "DeepSeek 未返回可显示的回答",
                    "UPSTREAM_INVALID",
                );
            }
            return Json(json!({
                "message": content,
                "model": model,
                "datasets": used.into_iter().collect::<Vec<_>>(),
                "usage": usage
            }))
            .into_response();
        }
        messages.push(message);
        // DeepSeek requires every assistant tool call to be followed by a
        // matching tool result before the conversation can continue.
        for call in tool_calls {
            let call_id = call.get("id").and_then(Value::as_str).unwrap_or_default();
            let name = call
                .pointer("/function/name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = call
                .pointer("/function/arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let fingerprint = format!("{name}:{arguments}");
            let repeated = !tool_requests.insert(fingerprint);
            let tool_result = if repeated {
                Ok((
                    json!({ "notice": "相同查询结果已在本次对话前文提供，请直接使用前文数据。" }),
                    Vec::new(),
                ))
            } else {
                let connection = match state.database.lock() {
                    Ok(value) => value,
                    Err(_) => {
                        return failure(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "SQLite 锁已损坏",
                            "STORAGE",
                        )
                    }
                };
                execute_tool(&connection, name, arguments)
            };
            let (result, datasets) = match tool_result {
                Ok(value) => value,
                Err(message) => (json!({ "error": message }), Vec::new()),
            };
            used.extend(datasets);
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": result.to_string()
            }));
        }
    }
    failure(
        StatusCode::BAD_GATEWAY,
        "AI 管家执行的数据查询过多，请缩小问题范围后重试",
        "TOOL_LIMIT",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_allowlist_rejects_legacy_and_unknown_names() {
        assert_eq!(valid_model("deepseek-v4-flash"), Some("deepseek-v4-flash"));
        assert_eq!(valid_model("deepseek-v4-pro"), Some("deepseek-v4-pro"));
        assert_eq!(valid_model("deepseek-chat"), None);
    }

    #[test]
    fn date_snapshot_requires_iso_date() {
        let connection = Connection::open_in_memory().unwrap();
        for source in DATASETS {
            match source.kind {
                DatasetKind::Json => {
                    let normalized = matches!(
                        source.key,
                        "activities"
                            | "activity_logs"
                            | "transactions"
                            | "daily_reviews"
                            | "finance_accounts"
                            | "notes"
                            | "note_folders"
                            | "note_tags"
                            | "note_revisions"
                            | "workout_history"
                            | "workout_import_records"
                            | "training_notes"
                            | "english_articles"
                            | "english_learning_records"
                            | "english_vocabulary"
                            | "english_highlights"
                            | "english_notes"
                            | "english_analysis"
                    );
                    if normalized {
                        let sql = match source.key {
                            "activities" => "CREATE TABLE activities(id TEXT PRIMARY KEY,name TEXT,activity_type TEXT,unit TEXT,is_archived INTEGER,created_at TEXT,updated_at TEXT)",
                            "activity_logs" => "CREATE TABLE activity_logs(id TEXT PRIMARY KEY,activity_id TEXT,value REAL,status TEXT,created_at TEXT,updated_at TEXT)",
                            "transactions" => "CREATE TABLE transactions(id TEXT PRIMARY KEY,transaction_type TEXT,amount_cents INTEGER,legacy_category_name TEXT,legacy_account_name TEXT,occurred_at TEXT,created_at TEXT,updated_at TEXT)",
                            "daily_reviews" => "CREATE TABLE daily_reviews(id TEXT PRIMARY KEY,review_date TEXT,energy INTEGER,mood INTEGER,created_at TEXT,updated_at TEXT)",
                            "finance_accounts" => "CREATE TABLE finance_accounts(id TEXT PRIMARY KEY,name TEXT,account_type TEXT,opening_balance_cents INTEGER,created_at TEXT,updated_at TEXT)",
                            "notes" => "CREATE TABLE notes(id TEXT PRIMARY KEY,title TEXT,note_type TEXT,summary TEXT,is_archived INTEGER,created_at TEXT,updated_at TEXT)",
                            "note_folders" => "CREATE TABLE note_folders(id TEXT PRIMARY KEY,name TEXT,created_at TEXT,updated_at TEXT)",
                            "note_tags" => "CREATE TABLE note_tags(id TEXT PRIMARY KEY,name TEXT,created_at TEXT,updated_at TEXT)",
                            "note_revisions" => "CREATE TABLE note_revisions(id TEXT PRIMARY KEY,note_id TEXT,revision_version INTEGER,created_at TEXT)",
                            "workout_history" => "CREATE TABLE workouts(id TEXT PRIMARY KEY,name TEXT,occurred_at TEXT,duration_seconds INTEGER,source TEXT,source_id TEXT,created_at TEXT,updated_at TEXT)",
                            "workout_import_records" => "CREATE TABLE workout_imports(id TEXT PRIMARY KEY,source TEXT,share_url TEXT,status TEXT,workout_id TEXT,created_at TEXT,updated_at TEXT)",
                            "training_notes" => "CREATE TABLE training_notes(id TEXT PRIMARY KEY,title TEXT,content TEXT,note_date TEXT,workout_id TEXT,created_at TEXT,updated_at TEXT)",
                            "english_articles" => "CREATE TABLE english_articles(id TEXT PRIMARY KEY,title TEXT,level TEXT,category TEXT,word_count INTEGER,created_time TEXT,published_at TEXT,created_at TEXT,updated_at TEXT)",
                            "english_learning_records" => "CREATE TABLE english_learning_records(id TEXT PRIMARY KEY,article_id TEXT,record_date TEXT,reading_time_seconds INTEGER,summary TEXT,created_at TEXT,updated_at TEXT)",
                            "english_vocabulary" => "CREATE TABLE english_vocabulary(id TEXT PRIMARY KEY,display_word TEXT,normalized_word TEXT,status TEXT,next_review_at TEXT,created_at TEXT,updated_at TEXT)",
                            "english_highlights" => "CREATE TABLE english_highlights(id TEXT PRIMARY KEY,article_id TEXT,selected_text TEXT,created_at TEXT,updated_at TEXT)",
                            "english_notes" => "CREATE TABLE english_notes(id TEXT PRIMARY KEY,article_id TEXT,content TEXT,created_at TEXT,updated_at TEXT)",
                            _ => "CREATE TABLE english_ai_analysis(id TEXT PRIMARY KEY,record_id TEXT,article_id TEXT,score REAL,created_at TEXT,updated_at TEXT)",
                        };
                        connection.execute(sql, []).ok();
                    } else {
                        connection.execute(&format!("CREATE TABLE {}(id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL)", source.table), []).ok();
                    }
                }
                DatasetKind::Photos => {
                    connection.execute("CREATE TABLE photos(id TEXT,original_file_name TEXT,media_type TEXT,mime_type TEXT,file_size INTEGER,width INTEGER,height INTEGER,duration_ms INTEGER,captured_at TEXT,imported_at TEXT,processing_status TEXT,processing_error TEXT,source_device_id TEXT,deleted_at TEXT)", []).ok();
                }
                DatasetKind::PhotoDevices => {
                    connection.execute("CREATE TABLE photo_sync_devices(id TEXT,device_name TEXT,device_type TEXT,status TEXT,paired_at TEXT,last_seen_at TEXT,revoked_at TEXT)", []).ok();
                }
                DatasetKind::PhotoTasks => {
                    connection.execute("CREATE TABLE photo_upload_tasks(id TEXT,device_id TEXT,original_file_name TEXT,media_type TEXT,mime_type TEXT,captured_at TEXT,expected_file_size INTEGER,received_file_size INTEGER,status TEXT,photo_id TEXT,created_at TEXT,updated_at TEXT,expires_at TEXT,error_code TEXT,error_message TEXT,is_duplicate INTEGER)", []).ok();
                }
            }
        }
        assert!(day_snapshot(&connection, "2026/08/04").is_err());
        assert!(day_snapshot(&connection, "2026-08-04").is_ok());
    }

    #[test]
    fn schema_creates_conversation_history_table() {
        let connection = Connection::open_in_memory().unwrap();
        ensure_schema(&connection).unwrap();
        let table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='ai_conversations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table, "ai_conversations");
    }
}
