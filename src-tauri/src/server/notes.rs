//! `/api/notes` 处理器：统一委托给笔记 Repository。
//!
//! 处理器不直接承担数据库转换逻辑；schema 由版本化 Migration 管理，
//! 本模块不再创建任何业务表。

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::database::repositories::notes as notes_repo;

use super::AppState;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteQuery {
    action: Option<String>,
    id: Option<String>,
    q: Option<String>,
    scope: Option<String>,
    folder_id: Option<String>,
    tag_id: Option<String>,
    note_type: Option<String>,
    sort: Option<String>,
    limit: Option<usize>,
}

fn failure(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn finish(result: Result<Value, String>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(
            if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            },
            message,
        ),
    }
}

pub async fn get(State(state): State<AppState>, Query(query): Query<NoteQuery>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let action = query.action.as_deref().unwrap_or("list");
    let result: Result<Value, String> = (|| match action {
        "get" => {
            let note_id = query
                .id
                .as_deref()
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            match notes_repo::get_note(&connection, note_id)? {
                Some(note) => Ok(note),
                None => Err("笔记不存在".to_owned()),
            }
        }
        "meta" => notes_repo::meta(&connection),
        "revisions" => {
            let note_id = query.id.as_deref().unwrap_or_default();
            Ok(Value::Array(notes_repo::list_revisions(
                &connection,
                note_id,
            )?))
        }
        "backup" => notes_repo::backup(&connection),
        _ => notes_repo::list_notes(
            &connection,
            query.q.as_deref(),
            query.scope.as_deref(),
            query.folder_id.as_deref(),
            query.tag_id.as_deref(),
            query.note_type.as_deref(),
            query.sort.as_deref(),
            query.limit.unwrap_or(100),
        )
        .map(Value::Array),
    })();
    finish(result)
}

pub async fn mutate(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let action = body
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let result: Result<Value, String> = (|| match action {
        "create" => {
            let note = body
                .get("note")
                .ok_or_else(|| "缺少笔记内容".to_owned())?;
            notes_repo::save_note(&connection, note, false, false)
        }
        "update" => {
            let note = body
                .get("note")
                .ok_or_else(|| "缺少笔记内容".to_owned())?;
            let create_revision = body
                .get("createRevision")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            notes_repo::save_note(&connection, note, true, create_revision)
        }
        "trash" | "restore" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            notes_repo::set_deleted(&connection, note_id, action == "trash")?;
            Ok(json!({ "ok": true }))
        }
        "delete" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            notes_repo::delete_note(&connection, note_id)?;
            Ok(json!({ "ok": true }))
        }
        "duplicate" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            notes_repo::duplicate_note(&connection, note_id)
        }
        "folder.save" | "tag.save" => {
            let key = if action == "folder.save" { "folder" } else { "tag" };
            let input = body
                .get(key)
                .ok_or_else(|| format!("缺少{key}数据"))?;
            let entity_id = if action == "folder.save" {
                notes_repo::save_folder(&connection, input)?
            } else {
                notes_repo::save_tag(&connection, input)?
            };
            Ok(json!({ "ok": true, "id": entity_id }))
        }
        "folder.delete" => {
            let entity_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少 id".to_owned())?;
            notes_repo::delete_folder(&connection, entity_id)?;
            Ok(json!({ "ok": true }))
        }
        "tag.delete" => {
            let entity_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少 id".to_owned())?;
            notes_repo::delete_tag(&connection, entity_id)?;
            Ok(json!({ "ok": true }))
        }
        "revision.restore" => {
            let revision_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少版本 id".to_owned())?;
            notes_repo::restore_revision(&connection, revision_id)
        }
        "attachment.record" => {
            let file = body
                .get("file")
                .ok_or_else(|| "缺少附件数据".to_owned())?;
            let note_id = file
                .get("noteId")
                .and_then(Value::as_str)
                .ok_or_else(|| "附件缺少 noteId".to_owned())?;
            notes_repo::record_attachment(&connection, note_id, file)?;
            Ok(json!({ "ok": true }))
        }
        "attachment.delete" => {
            let attachment_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少附件 id".to_owned())?;
            notes_repo::delete_attachment(&connection, attachment_id)?;
            Ok(json!({ "ok": true }))
        }
        "backup.restore" => {
            let data = body
                .get("data")
                .ok_or_else(|| "备份格式错误".to_owned())?;
                notes_repo::restore_backup(&mut *connection, data)?;
            Ok(json!({ "ok": true }))
        }
        _ => Err("不支持的笔记操作".to_owned()),
    })();
    finish(result)
}
