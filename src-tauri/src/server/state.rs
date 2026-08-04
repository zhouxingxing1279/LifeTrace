use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Map, Value};

use crate::database::repositories::{finance, habits, workouts};

use super::AppState;

const JSON_TABLES: [(&str, &str); 1] = [("settings", "settings")];

fn table_name(key: &str) -> Option<&'static str> {
    JSON_TABLES
        .iter()
        .find_map(|(candidate, table)| (*candidate == key).then_some(*table))
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn updated_at(value: &Value) -> String {
    value
        .get("updatedAt")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
}

fn put(connection: &Connection, table: &str, value: &Value) -> Result<(), String> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let sql = format!(
        "INSERT INTO {table} (id, data_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
           data_json=excluded.data_json,
           updated_at=excluded.updated_at"
    );
    connection
        .execute(&sql, params![id, value.to_string(), updated_at(value)])
        .map_err(|value| value.to_string())?;
    Ok(())
}

fn put_transaction(
    transaction: &Transaction<'_>,
    table: &str,
    value: &Value,
) -> Result<(), String> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "恢复数据缺少 id".to_owned())?;
    let sql = format!(
        "INSERT INTO {table} (id, data_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
           data_json=excluded.data_json,
           updated_at=excluded.updated_at"
    );
    transaction
        .execute(&sql, params![id, value.to_string(), updated_at(value)])
        .map_err(|value| value.to_string())?;
    Ok(())
}

fn read_table(connection: &Connection, table: &str) -> Result<Value, String> {
    let sql = format!("SELECT data_json FROM {table} ORDER BY updated_at DESC");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|value| value.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|value| value.to_string())?;
    let mut values = Vec::new();
    for row in rows {
        let raw = row.map_err(|value| value.to_string())?;
        values.push(serde_json::from_str(&raw).map_err(|value| value.to_string())?);
    }
    Ok(Value::Array(values))
}

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    for (_, table) in JSON_TABLES {
        connection.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                   id TEXT PRIMARY KEY,
                   data_json TEXT NOT NULL,
                   updated_at TEXT NOT NULL
                 )"
            ),
            [],
        )?;
    }
    let settings_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))?;
    if settings_count == 0 {
        let stamp = chrono::Utc::now().to_rfc3339();
        let settings = json!({
            "id": "preferences",
            "dark": false,
            "timer": null,
            "updatedAt": stamp,
        });
        put(connection, "settings", &settings)
            .map_err(|message| rusqlite::Error::ToSqlConversionFailure(message.into()))?;
    }
    Ok(())
}

pub async fn get(State(state): State<AppState>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let mut result = Map::new();
    match habits::list_activities(&connection) {
        Ok(activities) => {
            result.insert("activities".to_owned(), Value::Array(activities));
        }
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
    match habits::list_activity_logs(&connection) {
        Ok(logs) => {
            result.insert("logs".to_owned(), Value::Array(logs));
        }
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
    match habits::list_daily_reviews(&connection) {
        Ok(reviews) => {
            result.insert("reviews".to_owned(), Value::Array(reviews));
        }
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
    match workouts::list_workouts(&connection) {
        Ok(workout_history) => {
            result.insert("workoutHistory".to_owned(), Value::Array(workout_history));
        }
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
    for (key, table) in JSON_TABLES {
        let values = match read_table(&connection, table) {
            Ok(value) => value,
            Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        if key == "settings" {
            result.insert(
                key.to_owned(),
                values
                    .as_array()
                    .and_then(|items| items.first())
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            );
        } else {
            result.insert(key.to_owned(), values);
        }
    }
    match finance::list_accounts(&connection) {
        Ok(accounts) => {
            result.insert("accounts".to_owned(), Value::Array(accounts));
        }
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
    match finance::list_transactions(&connection) {
        Ok(transactions) => {
            result.insert("transactions".to_owned(), Value::Array(transactions));
        }
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
    Json(Value::Object(result)).into_response()
}

pub async fn mutate(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let operation = body
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };

    let outcome = match operation {
        "put" => {
            let key = body
                .get("table")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some(value) = body.get("value") else {
                return error(StatusCode::BAD_REQUEST, "缺少写入数据");
            };
            if key == "accounts" {
                return match finance::save_account(&connection, value) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            if key == "transactions" {
                return match finance::save_transaction(&connection, value) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            if key == "activities" {
                return match habits::save_activity(&connection, value) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            if key == "logs" {
                return match habits::save_activity_log(&connection, value) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            if key == "reviews" {
                return match habits::save_daily_review(&connection, value) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            if key == "workoutHistory" {
                return match workouts::save_workout(&connection, value) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            let Some(table) = table_name(key) else {
                return error(StatusCode::BAD_REQUEST, "不支持的数据表");
            };
            put(&connection, table, value)
        }
        "patch" => {
            let key = body
                .get("table")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if key == "settings" {
                return error(StatusCode::BAD_REQUEST, "设置不支持局部更新");
            }
            let Some(id) = body.get("id").and_then(Value::as_str) else {
                return error(StatusCode::BAD_REQUEST, "缺少数据 id");
            };
            let Some(patch) = body.get("patch").and_then(Value::as_object) else {
                return error(StatusCode::BAD_REQUEST, "缺少更新内容");
            };
            if key == "accounts" {
                let existing = finance::list_accounts(&connection)
                    .ok()
                    .and_then(|items| items.into_iter().find(|item| item.get("id").and_then(Value::as_str) == Some(id)))
                    .ok_or_else(|| "项目不存在".to_owned());
                return match existing {
                    Ok(mut value) => {
                        if let Some(object) = value.as_object_mut() {
                            object.extend(patch.clone());
                            object.insert("id".to_owned(), Value::String(id.to_owned()));
                        }
                        match finance::save_account(&connection, &value) {
                            Ok(()) => Json(json!({ "ok": true })).into_response(),
                            Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                        }
                    }
                    Err(message) => error(StatusCode::NOT_FOUND, message),
                };
            }
            if key == "activities" {
                let existing = habits::list_activities(&connection)
                    .ok()
                    .and_then(|items| items.into_iter().find(|item| item.get("id").and_then(Value::as_str) == Some(id)))
                    .ok_or_else(|| "项目不存在".to_owned());
                return match existing {
                    Ok(mut value) => {
                        if let Some(object) = value.as_object_mut() {
                            object.extend(patch.clone());
                            object.insert("id".to_owned(), Value::String(id.to_owned()));
                        }
                        match habits::save_activity(&connection, &value) {
                            Ok(()) => Json(json!({ "ok": true })).into_response(),
                            Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                        }
                    }
                    Err(message) => error(StatusCode::NOT_FOUND, message),
                };
            }
            let Some(table) = table_name(key) else {
                return error(StatusCode::BAD_REQUEST, "不支持的数据表");
            };
            let sql = format!("SELECT data_json FROM {table} WHERE id=?1");
            let raw = match connection
                .query_row(&sql, [id], |row| row.get::<_, String>(0))
                .optional()
            {
                Ok(Some(value)) => value,
                Ok(None) => return error(StatusCode::NOT_FOUND, "项目不存在"),
                Err(value) => return error(StatusCode::INTERNAL_SERVER_ERROR, value.to_string()),
            };
            let mut value: Value = match serde_json::from_str(&raw) {
                Ok(value) => value,
                Err(value) => return error(StatusCode::INTERNAL_SERVER_ERROR, value.to_string()),
            };
            let Some(object) = value.as_object_mut() else {
                return error(StatusCode::INTERNAL_SERVER_ERROR, "数据库记录格式错误");
            };
            object.extend(patch.clone());
            object.insert("id".to_owned(), Value::String(id.to_owned()));
            put(&connection, table, &value)
        }
        "delete" => {
            let key = body
                .get("table")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if key == "settings" {
                return error(StatusCode::BAD_REQUEST, "不能删除设置");
            }
            let Some(id) = body.get("id").and_then(Value::as_str) else {
                return error(StatusCode::BAD_REQUEST, "缺少数据 id");
            };
            if key == "accounts" {
                return match finance::delete_account(&connection, id) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            if key == "transactions" {
                return match finance::delete_transaction(&connection, id) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            if key == "workoutHistory" {
                return match workouts::delete_workout(&connection, id) {
                    Ok(()) => Json(json!({ "ok": true })).into_response(),
                    Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
                };
            }
            let Some(table) = table_name(key) else {
                return error(StatusCode::BAD_REQUEST, "不支持的数据表");
            };
            connection
                .execute(&format!("DELETE FROM {table} WHERE id=?1"), [id])
                .map(|_| ())
                .map_err(|value| value.to_string())
        }
        "restore" => {
            let Some(data) = body.get("data").and_then(Value::as_object) else {
                return error(StatusCode::BAD_REQUEST, "备份数据格式错误");
            };
            // 恢复前必须创建一致性备份。
            if let Err(message) = crate::database::backup::create_backup(
                &connection,
                &state.data_dir,
                "before-restore",
            ) {
                return error(StatusCode::INTERNAL_SERVER_ERROR, message);
            }
            let transaction = match connection.transaction() {
                Ok(value) => value,
                Err(value) => return error(StatusCode::INTERNAL_SERVER_ERROR, value.to_string()),
            };
            let accounts = data
                .get("accounts")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let transactions = data
                .get("transactions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Err(message) = finance::replace_all(&transaction, &accounts, &transactions) {
                return error(StatusCode::INTERNAL_SERVER_ERROR, message);
            }
            let activities = data
                .get("activities")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let logs = data
                .get("logs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let reviews = data
                .get("reviews")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Err(message) = habits::replace_all(&transaction, &activities, &logs, &reviews) {
                return error(StatusCode::INTERNAL_SERVER_ERROR, message);
            }
            let workout_history = data
                .get("workoutHistory")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Err(message) = workouts::replace_all(&transaction, &workout_history) {
                return error(StatusCode::INTERNAL_SERVER_ERROR, message);
            }
            let mut restore_error = None;
            for (key, table) in JSON_TABLES {
                if key == "settings" {
                    continue;
                }
                if let Err(value) = transaction.execute(&format!("DELETE FROM {table}"), []) {
                    restore_error = Some(value.to_string());
                    break;
                }
                let items = data
                    .get(key)
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for item in items {
                    if let Err(message) = put_transaction(&transaction, table, &item) {
                        restore_error = Some(message);
                        break;
                    }
                }
                if restore_error.is_some() {
                    break;
                }
            }
            match restore_error {
                Some(message) => Err(message),
                None => transaction.commit().map_err(|value| value.to_string()),
            }
        }
        _ => return error(StatusCode::BAD_REQUEST, "不支持的数据操作"),
    };

    match outcome {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}
