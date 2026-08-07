use std::path::{Path, PathBuf};

use axum::{
    body::Body,
    extract::{Multipart, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::AppState;

const MAXIMUM_UPLOAD_SIZE: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportUpload {
    id: String,
    kind: String,
    filename: String,
    content_type: String,
    size: usize,
    status: String,
    object_key: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub(super) struct IdQuery {
    id: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct StatusUpdate {
    id: String,
    status: String,
}

fn failure(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn sanitize_file_name(value: &str) -> String {
    let result: String = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if result.is_empty() {
        "upload.bin".to_owned()
    } else {
        result
    }
}

fn read_item(connection: &Connection, id: &str) -> Result<Option<ImportUpload>, String> {
    let raw = connection
        .query_row(
            "SELECT data_json FROM import_uploads WHERE id=?1",
            [id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|value| value.to_string())?;
    raw.map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn absolute_upload_path(data_dir: &Path, object_key: &str) -> Option<PathBuf> {
    let root = data_dir.join("imports");
    let candidate = root.join(object_key);
    candidate.starts_with(&root).then_some(candidate)
}

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS import_uploads (
           id TEXT PRIMARY KEY,
           data_json TEXT NOT NULL,
           updated_at TEXT NOT NULL
         )",
        [],
    )?;
    Ok(())
}

pub async fn get(State(state): State<AppState>, Query(query): Query<IdQuery>) -> Response {
    if let Some(id) = query.id {
        let item = {
            let connection = match state.database.lock() {
                Ok(value) => value,
                Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
            };
            match read_item(&connection, &id) {
                Ok(Some(value)) => value,
                Ok(None) => return failure(StatusCode::NOT_FOUND, "文件不存在"),
                Err(message) => return failure(StatusCode::INTERNAL_SERVER_ERROR, message),
            }
        };
        let Some(path) = absolute_upload_path(&state.data_dir, &item.object_key) else {
            return failure(StatusCode::BAD_REQUEST, "文件路径无效");
        };
        let bytes = match tokio::fs::read(path).await {
            Ok(value) => value,
            Err(_) => return failure(StatusCode::NOT_FOUND, "文件内容不存在"),
        };
        let mut response = Response::new(Body::from(bytes));
        if let Ok(value) = HeaderValue::from_str(&item.content_type) {
            response.headers_mut().insert(header::CONTENT_TYPE, value);
        }
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        return response;
    }

    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let mut statement =
        match connection.prepare("SELECT data_json FROM import_uploads ORDER BY updated_at DESC") {
            Ok(value) => value,
            Err(value) => return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string()),
        };
    let rows = match statement.query_map([], |row| row.get::<_, String>(0)) {
        Ok(value) => value,
        Err(value) => return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string()),
    };
    let mut items = Vec::new();
    for row in rows {
        let raw = match row {
            Ok(value) => value,
            Err(value) => return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string()),
        };
        match serde_json::from_str::<ImportUpload>(&raw) {
            Ok(value) => items.push(value),
            Err(value) => return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string()),
        }
    }
    Json(json!({ "items": items })).into_response()
}

pub async fn create(State(state): State<AppState>, mut multipart: Multipart) -> Response {
    let mut kind = None;
    let mut upload = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("kind") => kind = field.text().await.ok(),
            Some("file") => {
                let filename = field.file_name().unwrap_or("upload.bin").to_owned();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_owned();
                match field.bytes().await {
                    Ok(bytes) => upload = Some((filename, content_type, bytes)),
                    Err(_) => return failure(StatusCode::BAD_REQUEST, "无法读取上传文件"),
                }
            }
            _ => {}
        }
    }
    let Some(kind) = kind.filter(|value| value == "fitness" || value == "bill") else {
        return failure(StatusCode::BAD_REQUEST, "请选择正确的导入类型");
    };
    let Some((filename, content_type, bytes)) = upload else {
        return failure(StatusCode::BAD_REQUEST, "请选择导入文件");
    };
    if bytes.is_empty() || bytes.len() > MAXIMUM_UPLOAD_SIZE {
        return failure(StatusCode::PAYLOAD_TOO_LARGE, "单个文件必须小于 25MB");
    }

    let id = Uuid::new_v4().to_string();
    let stamp = chrono::Utc::now().to_rfc3339();
    let object_key = format!("{kind}/{id}/{}", sanitize_file_name(&filename));
    let Some(path) = absolute_upload_path(&state.data_dir, &object_key) else {
        return failure(StatusCode::BAD_REQUEST, "文件路径无效");
    };
    if let Some(parent) = path.parent() {
        if let Err(value) = tokio::fs::create_dir_all(parent).await {
            return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string());
        }
    }
    if let Err(value) = tokio::fs::write(&path, &bytes).await {
        return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string());
    }
    let item = ImportUpload {
        id,
        kind,
        filename,
        content_type,
        size: bytes.len(),
        status: "pending".to_owned(),
        object_key,
        created_at: stamp.clone(),
        updated_at: stamp,
    };
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    if let Err(value) = connection.execute(
        "INSERT INTO import_uploads(id,data_json,updated_at) VALUES(?1,?2,?3)",
        params![item.id, json!(item).to_string(), item.updated_at],
    ) {
        return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string());
    }
    Json(json!({ "item": item })).into_response()
}

pub async fn update(State(state): State<AppState>, Json(body): Json<StatusUpdate>) -> Response {
    if body.status != "pending" && body.status != "parsed" {
        return failure(StatusCode::BAD_REQUEST, "状态格式无效");
    }
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let mut item = match read_item(&connection, &body.id) {
        Ok(Some(value)) => value,
        Ok(None) => return failure(StatusCode::NOT_FOUND, "文件不存在"),
        Err(message) => return failure(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    item.status = body.status;
    item.updated_at = chrono::Utc::now().to_rfc3339();
    if let Err(value) = connection.execute(
        "UPDATE import_uploads SET data_json=?1,updated_at=?2 WHERE id=?3",
        params![json!(item).to_string(), item.updated_at, item.id],
    ) {
        return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string());
    }
    Json(json!({ "item": item })).into_response()
}

pub async fn remove(State(state): State<AppState>, Query(query): Query<IdQuery>) -> Response {
    let Some(id) = query.id else {
        return failure(StatusCode::BAD_REQUEST, "缺少文件 ID");
    };
    let item = {
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
        };
        let item = match read_item(&connection, &id) {
            Ok(value) => value,
            Err(message) => return failure(StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        if let Err(value) = connection.execute("DELETE FROM import_uploads WHERE id=?1", [&id]) {
            return failure(StatusCode::INTERNAL_SERVER_ERROR, value.to_string());
        }
        item
    };
    if let Some(item) = item {
        if let Some(path) = absolute_upload_path(&state.data_dir, &item.object_key) {
            let _ = tokio::fs::remove_file(path).await;
        }
    }
    Json(json!({ "ok": true })).into_response()
}
