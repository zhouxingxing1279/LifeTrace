//! `/api/notes` 处理器：统一委托给笔记 Repository。
//!
//! Desktop 本地写入在这一层同时写入 sync outbox；Repository 本身保持无写入来源
//! 假设，避免远端 Pull/Migration 复用 Repository 时产生回声同步。

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use lifetrace_contracts::registry::EntityType;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::database::repositories::notes as notes_repo;
use crate::sync::outbox::{enqueue_delete, enqueue_upsert, MutationOrigin};

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

fn meta_entity(connection: &Connection, collection: &str, id: &str) -> Result<Value, String> {
    let meta = notes_repo::meta(connection)?;
    meta.get(collection)
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_str) == Some(id))
        })
        .cloned()
        .ok_or_else(|| "笔记元数据保存后无法重新读取".to_owned())
}

fn assign_meta_owner(connection: &Connection, table: &str, id: &str) -> Result<(), String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    connection
        .execute(
            &format!("UPDATE {table} SET user_id=?1 WHERE id=?2"),
            rusqlite::params![profile_id, id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn enqueue_note(connection: &Connection, note: &Value) -> Result<(), String> {
    enqueue_upsert(
        connection,
        EntityType::NOTE_NOTE,
        note,
        None,
        MutationOrigin::Local,
    )?;
    Ok(())
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
                Some(note) => crate::database::note_links::enrich_note(&connection, note),
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
        "create" | "update" => {
            let note = body.get("note").ok_or_else(|| "缺少笔记内容".to_owned())?;
            let create_revision = action == "update"
                && body
                    .get("createRevision")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            let saved = notes_repo::save_note(
                &transaction,
                note,
                action == "update",
                create_revision,
            )?;
            crate::database::note_links::sync_note_links(&transaction, &saved)?;
            let enriched = crate::database::note_links::enrich_note(&transaction, saved)?;
            enqueue_note(&transaction, &enriched)?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(enriched)
        }
        "trash" | "restore" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            let deleting = action == "trash";
            notes_repo::set_deleted(&transaction, note_id, deleting)?;
            if deleting {
                enqueue_delete(
                    &transaction,
                    EntityType::NOTE_NOTE,
                    note_id,
                    None,
                    MutationOrigin::Local,
                )?;
            } else {
                let restored = notes_repo::get_note(&transaction, note_id)?
                    .ok_or_else(|| "笔记不存在".to_owned())?;
                enqueue_note(&transaction, &restored)?;
            }
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "delete" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            notes_repo::delete_note(&transaction, note_id)?;
            enqueue_delete(
                &transaction,
                EntityType::NOTE_NOTE,
                note_id,
                None,
                MutationOrigin::Local,
            )?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "duplicate" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            let duplicated = notes_repo::duplicate_note(&transaction, note_id)?;
            crate::database::note_links::sync_note_links(&transaction, &duplicated)?;
            let enriched = crate::database::note_links::enrich_note(&transaction, duplicated)?;
            enqueue_note(&transaction, &enriched)?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(enriched)
        }
        "folder.save" | "tag.save" => {
            let is_folder = action == "folder.save";
            let key = if is_folder { "folder" } else { "tag" };
            let input = body.get(key).ok_or_else(|| format!("缺少{key}数据"))?;
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            let entity_id = if is_folder {
                notes_repo::save_folder(&transaction, input)?
            } else {
                notes_repo::save_tag(&transaction, input)?
            };
            // The legacy repository still writes historical `local` ownership for
            // folder/tag rows. Normalize it at the Desktop write boundary so the
            // active profile remains isolated until the repository is fully modernized.
            assign_meta_owner(
                &transaction,
                if is_folder { "note_folders" } else { "note_tags" },
                &entity_id,
            )?;
            let entity = meta_entity(
                &transaction,
                if is_folder { "folders" } else { "tags" },
                &entity_id,
            )?;
            enqueue_upsert(
                &transaction,
                if is_folder { EntityType::NOTE_FOLDER } else { EntityType::NOTE_TAG },
                &entity,
                None,
                MutationOrigin::Local,
            )?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(json!({ "ok": true, "id": entity_id }))
        }
        "folder.delete" | "tag.delete" => {
            let entity_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少 id".to_owned())?;
            let is_folder = action == "folder.delete";
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            if is_folder {
                notes_repo::delete_folder(&transaction, entity_id)?;
            } else {
                notes_repo::delete_tag(&transaction, entity_id)?;
            }
            enqueue_delete(
                &transaction,
                if is_folder { EntityType::NOTE_FOLDER } else { EntityType::NOTE_TAG },
                entity_id,
                None,
                MutationOrigin::Local,
            )?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "revision.restore" => {
            let revision_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少版本 id".to_owned())?;
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            let restored = notes_repo::restore_revision(&transaction, revision_id)?;
            crate::database::note_links::sync_note_links(&transaction, &restored)?;
            let enriched = crate::database::note_links::enrich_note(&transaction, restored)?;
            enqueue_note(&transaction, &enriched)?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(enriched)
        }
        "attachment.record" => {
            let file = body.get("file").ok_or_else(|| "缺少附件数据".to_owned())?;
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
            let data = body.get("data").ok_or_else(|| "备份格式错误".to_owned())?;
            crate::database::backup::create_backup(
                &connection,
                &state.data_dir,
                "before-notes-restore",
            )?;
            notes_repo::restore_backup(&mut *connection, data)?;
            crate::database::note_links::rebuild_all(&connection)?;
            // Backup restore is explicitly a local mutation. Queue restored note
            // entities so the next background sync can reconcile the cloud copy.
            for note in notes_repo::list_notes(
                &connection,
                None,
                Some("all"),
                None,
                None,
                None,
                None,
                250,
            )? {
                if let Some(id) = note.get("id").and_then(Value::as_str) {
                    if let Some(full) = notes_repo::get_note(&connection, id)? {
                        enqueue_note(&connection, &full)?;
                    }
                }
            }
            let meta = notes_repo::meta(&connection)?;
            for (collection, entity_type) in [
                ("folders", EntityType::NOTE_FOLDER),
                ("tags", EntityType::NOTE_TAG),
            ] {
                for entity in meta
                    .get(collection)
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                {
                    enqueue_upsert(
                        &connection,
                        entity_type,
                        &entity,
                        None,
                        MutationOrigin::Local,
                    )?;
                }
            }
            Ok(json!({ "ok": true }))
        }
        _ => Err("不支持的笔记操作".to_owned()),
    })();
    finish(result)
}
