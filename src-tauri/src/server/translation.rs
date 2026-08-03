use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    app_id: String,
    secret: Option<String>,
}

#[derive(Deserialize)]
pub struct TranslateInput {
    text: String,
}

fn failure(status: StatusCode, message: impl Into<String>, code: &str) -> Response {
    (
        status,
        Json(json!({ "error": message.into(), "code": code })),
    )
        .into_response()
}

fn read_settings(connection: &Connection) -> Result<Option<Value>, String> {
    let raw = connection
        .query_row(
            "SELECT data_json FROM translation_settings WHERE id='baidu'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|value| value.to_string())?;
    raw.map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS translation_settings(
           id TEXT PRIMARY KEY,
           data_json TEXT NOT NULL,
           updated_at TEXT NOT NULL
         )",
        [],
    )?;
    Ok(())
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
        Ok(settings) => Json(json!({
            "appId": settings.as_ref().and_then(|value| value.get("appId")).and_then(Value::as_str).unwrap_or_default(),
            "configured": settings.as_ref().is_some_and(|value| {
                value.get("appId").and_then(Value::as_str).is_some_and(|value| !value.is_empty())
                    && value.get("secret").and_then(Value::as_str).is_some_and(|value| !value.is_empty())
            }),
            "updatedAt": settings.and_then(|value| value.get("updatedAt").cloned())
        }))
        .into_response(),
        Err(message) => failure(StatusCode::INTERNAL_SERVER_ERROR, message, "STORAGE"),
    }
}

pub async fn settings_save(
    State(state): State<AppState>,
    Json(input): Json<SettingsInput>,
) -> Response {
    let app_id = input.app_id.trim().chars().take(120).collect::<String>();
    if app_id.is_empty() {
        return failure(
            StatusCode::BAD_REQUEST,
            "App ID 不能为空",
            "INVALID_REQUEST",
        );
    }
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
    let old = read_settings(&connection).ok().flatten();
    let secret = input
        .secret
        .map(|value| value.trim().chars().take(240).collect::<String>())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            old.as_ref()
                .and_then(|value| value.get("secret"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let updated_at = chrono::Utc::now().to_rfc3339();
    let value = json!({ "appId": app_id, "secret": secret, "updatedAt": updated_at });
    if let Err(error) = connection.execute(
        "INSERT INTO translation_settings(id,data_json,updated_at) VALUES('baidu',?1,?2)
         ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at",
        params![value.to_string(), updated_at],
    ) {
        return failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
            "STORAGE",
        );
    }
    Json(json!({
        "appId": app_id,
        "configured": secret.is_some(),
        "updatedAt": updated_at
    }))
    .into_response()
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
    match connection.execute("DELETE FROM translation_settings WHERE id='baidu'", []) {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(value) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            value.to_string(),
            "STORAGE",
        ),
    }
}

pub async fn translate(
    State(state): State<AppState>,
    Json(input): Json<TranslateInput>,
) -> Response {
    let query = input.text.trim();
    if query.is_empty() || query.chars().count() > 2_000 {
        return failure(
            StatusCode::BAD_REQUEST,
            "请选择不超过 2000 字符的英文内容",
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
                    "翻译服务尚未配置",
                    "NOT_CONFIGURED",
                )
            }
        }
    };
    let app_id = settings
        .get("appId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let secret = settings
        .get("secret")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if app_id.is_empty() || secret.is_empty() {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "翻译服务尚未配置",
            "NOT_CONFIGURED",
        );
    }
    let salt = Uuid::new_v4().simple().to_string();
    let sign = format!(
        "{:x}",
        md5::compute(format!("{app_id}{query}{salt}{secret}"))
    );
    let response = reqwest::Client::new()
        .post("https://fanyi-api.baidu.com/api/trans/vip/translate")
        .form(&[
            ("q", query),
            ("from", "en"),
            ("to", "zh"),
            ("appid", app_id),
            ("salt", &salt),
            ("sign", &sign),
        ])
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await;
    let payload: Value = match response {
        Ok(value) if value.status().is_success() => match value.json().await {
            Ok(value) => value,
            Err(_) => {
                return failure(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "翻译响应无效",
                    "UNAVAILABLE",
                )
            }
        },
        _ => {
            return failure(
                StatusCode::SERVICE_UNAVAILABLE,
                "当前无法连接翻译服务",
                "UNAVAILABLE",
            )
        }
    };
    let Some(items) = payload.get("trans_result").and_then(Value::as_array) else {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            payload
                .get("error_msg")
                .and_then(Value::as_str)
                .unwrap_or("翻译服务返回错误"),
            "UNAVAILABLE",
        );
    };
    let source_text = items
        .iter()
        .filter_map(|item| item.get("src").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let translated_text = items
        .iter()
        .filter_map(|item| item.get("dst").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    Json(json!({
        "sourceText": source_text,
        "translatedText": translated_text,
        "from": payload.get("from").and_then(Value::as_str).unwrap_or("en"),
        "to": payload.get("to").and_then(Value::as_str).unwrap_or("zh"),
        "provider": "baidu"
    }))
    .into_response()
}
