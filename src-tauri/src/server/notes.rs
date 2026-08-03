use std::cmp::Ordering;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use uuid::Uuid;

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

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn object(value: &Value) -> Result<&Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| "笔记数据格式错误".to_owned())
}

fn read_entity(
    connection: &Connection,
    table: &str,
    entity_id: &str,
) -> Result<Option<Value>, String> {
    let raw = connection
        .query_row(
            &format!("SELECT data_json FROM {table} WHERE id=?1"),
            [entity_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|value| value.to_string())?;
    raw.map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_entities(connection: &Connection, table: &str) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT data_json FROM {table} ORDER BY updated_at DESC"
        ))
        .map_err(|value| value.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|value| value.to_string())?;
    let mut values = Vec::new();
    for row in rows {
        values.push(
            serde_json::from_str(&row.map_err(|value| value.to_string())?)
                .map_err(|value| value.to_string())?,
        );
    }
    Ok(values)
}

fn write_entity(connection: &Connection, table: &str, value: &Value) -> Result<(), String> {
    let entity_id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let updated_at = value
        .get("updatedAt")
        .and_then(Value::as_str)
        .unwrap_or_default();
    connection
        .execute(
            &format!(
                "INSERT INTO {table}(id,data_json,updated_at) VALUES(?1,?2,?3)
                 ON CONFLICT(id) DO UPDATE SET
                   data_json=excluded.data_json,
                   updated_at=excluded.updated_at"
            ),
            params![entity_id, value.to_string(), updated_at],
        )
        .map_err(|value| value.to_string())?;
    Ok(())
}

fn delete_entity(connection: &Connection, table: &str, entity_id: &str) -> Result<(), String> {
    connection
        .execute(&format!("DELETE FROM {table} WHERE id=?1"), [entity_id])
        .map(|_| ())
        .map_err(|value| value.to_string())
}

fn tag_values(connection: &Connection, ids: &[Value]) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    for tag_id in ids.iter().filter_map(Value::as_str) {
        if let Some(tag) = read_entity(connection, "note_tags_v2", tag_id)? {
            result.push(tag);
        }
    }
    Ok(result)
}

fn note_from_input(
    connection: &Connection,
    input: &Value,
    existing: Option<&Value>,
) -> Result<Value, String> {
    let source = object(input)?;
    let stamp = now();
    let note_id = source
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| existing.and_then(|value| value.get("id")?.as_str().map(str::to_owned)))
        .unwrap_or_else(id);
    let created_at = existing
        .and_then(|value| value.get("createdAt"))
        .and_then(Value::as_str)
        .unwrap_or(&stamp)
        .to_owned();
    let version = existing
        .and_then(|value| value.get("version"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + 1;
    let tags = tag_values(
        connection,
        source
            .get("tagIds")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    )?;
    Ok(json!({
        "id": note_id,
        "title": source.get("title").cloned().unwrap_or(Value::Null),
        "noteType": source.get("noteType").cloned().unwrap_or_else(|| json!("document")),
        "folderId": source.get("folderId").cloned().unwrap_or(Value::Null),
        "contentJson": source.get("contentJson").cloned().unwrap_or_else(|| json!({"type":"doc","content":[]})),
        "contentHtml": source.get("contentHtml").cloned().unwrap_or_else(|| json!("")),
        "contentText": source.get("contentText").cloned().unwrap_or_else(|| json!("")),
        "contentMarkdown": source.get("contentMarkdown").cloned().unwrap_or_else(|| json!("")),
        "summary": source.get("summary").cloned().unwrap_or_else(|| json!("")),
        "isPinned": source.get("isPinned").and_then(Value::as_bool).unwrap_or(false),
        "isFavorite": source.get("isFavorite").and_then(Value::as_bool).unwrap_or(false),
        "isArchived": source.get("isArchived").and_then(Value::as_bool).unwrap_or(false),
        "createdAt": created_at,
        "updatedAt": stamp,
        "deletedAt": existing.and_then(|value| value.get("deletedAt")).cloned().unwrap_or(Value::Null),
        "version": version,
        "tags": tags,
        "relations": source.get("relations").cloned().unwrap_or_else(|| json!([])),
        "attachments": existing.and_then(|value| value.get("attachments")).cloned().unwrap_or_else(|| json!([]))
    }))
}

fn save_revision(connection: &Connection, note: &Value) -> Result<(), String> {
    let revision = json!({
        "id": id(),
        "noteId": note.get("id").cloned().unwrap_or(Value::Null),
        "version": note.get("version").cloned().unwrap_or_else(|| json!(1)),
        "title": note.get("title").cloned().unwrap_or(Value::Null),
        "contentJson": note.get("contentJson").cloned().unwrap_or_else(|| json!({})),
        "contentHtml": note.get("contentHtml").cloned().unwrap_or_else(|| json!("")),
        "contentMarkdown": note.get("contentMarkdown").cloned().unwrap_or_else(|| json!("")),
        "createdAt": now()
    });
    write_entity(connection, "note_revisions_v2", &revision)
}

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    for table in [
        "notes_v2",
        "note_folders_v2",
        "note_tags_v2",
        "note_revisions_v2",
    ] {
        connection.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {table}(
                   id TEXT PRIMARY KEY,
                   data_json TEXT NOT NULL,
                   updated_at TEXT NOT NULL
                 )"
            ),
            [],
        )?;
    }
    let count: i64 =
        connection.query_row("SELECT COUNT(*) FROM note_folders_v2", [], |row| row.get(0))?;
    if count == 0 {
        let stamp = now();
        for (sort_order, (name, icon, color)) in [
            ("工作", "briefcase", "#416b5c"),
            ("学习", "book", "#5975a4"),
            ("健身", "dumbbell", "#b06943"),
            ("生活", "home", "#887257"),
            ("财务", "wallet", "#8b654d"),
            ("项目", "folder", "#6c668f"),
        ]
        .into_iter()
        .enumerate()
        {
            let folder = json!({
                "id": id(),
                "name": name,
                "icon": icon,
                "color": color,
                "sortOrder": sort_order,
                "createdAt": stamp,
                "updatedAt": stamp
            });
            write_entity(connection, "note_folders_v2", &folder)
                .map_err(|message| rusqlite::Error::ToSqlConversionFailure(message.into()))?;
        }
    }
    Ok(())
}

pub async fn get(State(state): State<AppState>, Query(query): Query<NoteQuery>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let action = query.action.as_deref().unwrap_or("list");
    let result: Result<Value, String> = (|| match action {
        "get" => match query.id.as_deref() {
            Some(note_id) => read_entity(&connection, "notes_v2", note_id)?
                .ok_or_else(|| "笔记不存在".to_owned()),
            None => Err("缺少笔记 id".to_owned()),
        },
        "meta" => {
            let folders = read_entities(&connection, "note_folders_v2")?;
            let mut tags = read_entities(&connection, "note_tags_v2")?;
            let notes = read_entities(&connection, "notes_v2")?;
            for tag in &mut tags {
                let Some(tag_id) = tag.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let usage_count = notes
                    .iter()
                    .filter(|note| {
                        note.get("tags")
                            .and_then(Value::as_array)
                            .is_some_and(|items| {
                                items.iter().any(|item| {
                                    item.get("id").and_then(Value::as_str) == Some(tag_id)
                                })
                            })
                    })
                    .count();
                if let Some(tag) = tag.as_object_mut() {
                    tag.insert("usageCount".to_owned(), json!(usage_count));
                }
            }
            Ok(json!({ "folders": folders, "tags": tags }))
        }
        "revisions" => {
            let note_id = query.id.as_deref().unwrap_or_default();
            let mut revisions: Vec<Value> = read_entities(&connection, "note_revisions_v2")?
                .into_iter()
                .filter(|value| value.get("noteId").and_then(Value::as_str) == Some(note_id))
                .collect();
            revisions.sort_by(|left, right| {
                right
                    .get("version")
                    .and_then(Value::as_u64)
                    .cmp(&left.get("version").and_then(Value::as_u64))
            });
            revisions.truncate(20);
            Ok(Value::Array(revisions))
        }
        "backup" => Ok(json!({
            "format": "lifetrace-notes",
            "version": 2,
            "createdAt": now(),
            "notes": read_entities(&connection, "notes_v2")?,
            "folders": read_entities(&connection, "note_folders_v2")?,
            "tags": read_entities(&connection, "note_tags_v2")?,
            "revisions": read_entities(&connection, "note_revisions_v2")?
        })),
        _ => {
            let search = query.q.unwrap_or_default().to_lowercase();
            let scope = query.scope.unwrap_or_else(|| "all".to_owned());
            let limit = query.limit.unwrap_or(100).clamp(1, 250);
            let mut notes: Vec<Value> = read_entities(&connection, "notes_v2")?
                .into_iter()
                .filter(|note| {
                    let deleted = note.get("deletedAt").is_some_and(|value| !value.is_null());
                    let archived = note
                        .get("isArchived")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    if scope == "trash" {
                        if !deleted {
                            return false;
                        }
                    } else if deleted || (scope != "archived" && archived) {
                        return false;
                    }
                    if scope == "favorite"
                        && !note
                            .get("isFavorite")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                    {
                        return false;
                    }
                    if scope == "pinned"
                        && !note
                            .get("isPinned")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                    {
                        return false;
                    }
                    if scope == "quick"
                        && note.get("noteType").and_then(Value::as_str) != Some("quick")
                    {
                        return false;
                    }
                    if let Some(folder_id) = query.folder_id.as_deref() {
                        if note.get("folderId").and_then(Value::as_str) != Some(folder_id) {
                            return false;
                        }
                    }
                    if let Some(note_type) = query.note_type.as_deref() {
                        if note.get("noteType").and_then(Value::as_str) != Some(note_type) {
                            return false;
                        }
                    }
                    if let Some(tag_id) = query.tag_id.as_deref() {
                        if !note
                            .get("tags")
                            .and_then(Value::as_array)
                            .is_some_and(|tags| {
                                tags.iter().any(|tag| {
                                    tag.get("id").and_then(Value::as_str) == Some(tag_id)
                                })
                            })
                        {
                            return false;
                        }
                    }
                    if !search.is_empty() {
                        let searchable = ["title", "contentText", "summary"]
                            .iter()
                            .filter_map(|key| note.get(key).and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_lowercase();
                        if !searchable.contains(&search) {
                            return false;
                        }
                    }
                    true
                })
                .collect();
            let sort = query.sort.as_deref().unwrap_or("updated_desc");
            notes.sort_by(|left, right| match sort {
                "created_asc" => text(left, "createdAt").cmp(&text(right, "createdAt")),
                "created_desc" => text(right, "createdAt").cmp(&text(left, "createdAt")),
                "title_asc" => text(left, "title").cmp(&text(right, "title")),
                "title_desc" => text(right, "title").cmp(&text(left, "title")),
                _ => {
                    let pinned = right
                        .get("isPinned")
                        .and_then(Value::as_bool)
                        .cmp(&left.get("isPinned").and_then(Value::as_bool));
                    if pinned == Ordering::Equal {
                        text(right, "updatedAt").cmp(&text(left, "updatedAt"))
                    } else {
                        pinned
                    }
                }
            });
            notes.truncate(limit);
            Ok(Value::Array(notes))
        }
    })();
    match result {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(
            if message == "笔记不存在" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            },
            message,
        ),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

pub async fn mutate(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let action = body
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "SQLite 锁已损坏"),
    };
    let result: Result<Value, String> = (|| match action {
        "create" | "update" => {
            let input = body.get("note").ok_or_else(|| "缺少笔记内容".to_owned())?;
            let existing = input
                .get("id")
                .and_then(Value::as_str)
                .map(|note_id| read_entity(&connection, "notes_v2", note_id))
                .transpose()?
                .flatten();
            if action == "update" && existing.is_none() {
                Err("笔记不存在".to_owned())
            } else {
                if existing.is_some()
                    && input
                        .get("createRevision")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                {
                    save_revision(&connection, existing.as_ref().expect("checked above"))?;
                }
                let note = note_from_input(&connection, input, existing.as_ref())?;
                write_entity(&connection, "notes_v2", &note)?;
                Ok(note)
            }
        }
        "trash" | "restore" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            let mut note = read_entity(&connection, "notes_v2", note_id)?
                .ok_or_else(|| "笔记不存在".to_owned())?;
            let object = note
                .as_object_mut()
                .ok_or_else(|| "笔记格式错误".to_owned())?;
            object.insert(
                "deletedAt".to_owned(),
                if action == "trash" {
                    json!(now())
                } else {
                    Value::Null
                },
            );
            object.insert("updatedAt".to_owned(), json!(now()));
            write_entity(&connection, "notes_v2", &note)?;
            Ok(json!({ "ok": true }))
        }
        "delete" => {
            let note_id = body
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少笔记 id".to_owned())?;
            delete_entity(&connection, "notes_v2", note_id)?;
            let revisions = read_entities(&connection, "note_revisions_v2")?;
            for revision in revisions
                .into_iter()
                .filter(|revision| revision.get("noteId").and_then(Value::as_str) == Some(note_id))
            {
                if let Some(revision_id) = revision.get("id").and_then(Value::as_str) {
                    delete_entity(&connection, "note_revisions_v2", revision_id)?;
                }
            }
            Ok(json!({ "ok": true }))
        }
        "duplicate" => {
            let note_id = body.get("id").and_then(Value::as_str).unwrap_or_default();
            let mut note = read_entity(&connection, "notes_v2", note_id)?
                .ok_or_else(|| "笔记不存在".to_owned())?;
            let stamp = now();
            let object = note
                .as_object_mut()
                .ok_or_else(|| "笔记格式错误".to_owned())?;
            object.insert("id".to_owned(), json!(id()));
            let title = object
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("无标题笔记");
            object.insert("title".to_owned(), json!(format!("{title} · 副本")));
            object.insert("isPinned".to_owned(), json!(false));
            object.insert("createdAt".to_owned(), json!(stamp));
            object.insert("updatedAt".to_owned(), json!(now()));
            object.insert("deletedAt".to_owned(), Value::Null);
            object.insert("version".to_owned(), json!(1));
            write_entity(&connection, "notes_v2", &note)?;
            Ok(note)
        }
        "folder.save" | "tag.save" => {
            let key = if action == "folder.save" {
                "folder"
            } else {
                "tag"
            };
            let table = if action == "folder.save" {
                "note_folders_v2"
            } else {
                "note_tags_v2"
            };
            let input = body
                .get(key)
                .and_then(Value::as_object)
                .ok_or_else(|| format!("缺少{key}数据"))?;
            let stamp = now();
            let entity_id = input
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(id);
            let mut entity = input.clone();
            entity.insert("id".to_owned(), json!(entity_id));
            entity
                .entry("color".to_owned())
                .or_insert_with(|| json!("#5f7d70"));
            if action == "folder.save" {
                entity
                    .entry("icon".to_owned())
                    .or_insert_with(|| json!("folder"));
                entity
                    .entry("sortOrder".to_owned())
                    .or_insert_with(|| json!(0));
            }
            entity
                .entry("createdAt".to_owned())
                .or_insert_with(|| json!(stamp));
            entity.insert("updatedAt".to_owned(), json!(now()));
            write_entity(&connection, table, &Value::Object(entity))?;
            Ok(json!({ "ok": true, "id": entity_id }))
        }
        "folder.delete" | "tag.delete" => {
            let entity_id = body.get("id").and_then(Value::as_str).unwrap_or_default();
            let table = if action == "folder.delete" {
                "note_folders_v2"
            } else {
                "note_tags_v2"
            };
            delete_entity(&connection, table, entity_id)?;
            let notes = read_entities(&connection, "notes_v2")?;
            for mut note in notes {
                let mut changed = false;
                if action == "folder.delete"
                    && note.get("folderId").and_then(Value::as_str) == Some(entity_id)
                {
                    note.as_object_mut()
                        .expect("notes are objects")
                        .insert("folderId".to_owned(), Value::Null);
                    changed = true;
                }
                if action == "tag.delete" {
                    if let Some(tags) = note.get_mut("tags").and_then(Value::as_array_mut) {
                        let before = tags.len();
                        tags.retain(|tag| tag.get("id").and_then(Value::as_str) != Some(entity_id));
                        changed = before != tags.len();
                    }
                }
                if changed {
                    write_entity(&connection, "notes_v2", &note)?;
                }
            }
            Ok(json!({ "ok": true }))
        }
        "revision.restore" => {
            let revision_id = body.get("id").and_then(Value::as_str).unwrap_or_default();
            let revision = read_entity(&connection, "note_revisions_v2", revision_id)?
                .ok_or_else(|| "历史版本不存在".to_owned())?;
            let note_id = revision
                .get("noteId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let mut note = read_entity(&connection, "notes_v2", note_id)?
                .ok_or_else(|| "笔记不存在".to_owned())?;
            save_revision(&connection, &note)?;
            let object = note
                .as_object_mut()
                .ok_or_else(|| "笔记格式错误".to_owned())?;
            for key in ["title", "contentJson", "contentHtml", "contentMarkdown"] {
                if let Some(value) = revision.get(key) {
                    object.insert(key.to_owned(), value.clone());
                }
            }
            object.insert(
                "contentText".to_owned(),
                revision
                    .get("contentMarkdown")
                    .cloned()
                    .unwrap_or_else(|| json!("")),
            );
            object.insert("updatedAt".to_owned(), json!(now()));
            let version = object.get("version").and_then(Value::as_u64).unwrap_or(0) + 1;
            object.insert("version".to_owned(), json!(version));
            write_entity(&connection, "notes_v2", &note)?;
            Ok(note)
        }
        "attachment.record" => {
            let file = body
                .get("file")
                .cloned()
                .ok_or_else(|| "缺少附件数据".to_owned())?;
            let note_id = file
                .get("noteId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let mut note = read_entity(&connection, "notes_v2", note_id)?
                .ok_or_else(|| "笔记不存在".to_owned())?;
            let attachments = note
                .as_object_mut()
                .ok_or_else(|| "笔记格式错误".to_owned())?
                .entry("attachments")
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .ok_or_else(|| "附件格式错误".to_owned())?;
            attachments.push(file);
            write_entity(&connection, "notes_v2", &note)?;
            Ok(json!({ "ok": true }))
        }
        "attachment.delete" => {
            let attachment_id = body.get("id").and_then(Value::as_str).unwrap_or_default();
            for mut note in read_entities(&connection, "notes_v2")? {
                let Some(attachments) = note.get_mut("attachments").and_then(Value::as_array_mut)
                else {
                    continue;
                };
                let before = attachments.len();
                attachments
                    .retain(|item| item.get("id").and_then(Value::as_str) != Some(attachment_id));
                if before != attachments.len() {
                    write_entity(&connection, "notes_v2", &note)?;
                    break;
                }
            }
            Ok(json!({ "ok": true }))
        }
        "backup.restore" => {
            let data = body
                .get("data")
                .and_then(Value::as_object)
                .ok_or_else(|| "备份格式错误".to_owned())?;
            if data.get("format").and_then(Value::as_str) != Some("lifetrace-notes") {
                Err("不支持的笔记备份格式".to_owned())
            } else {
                for (key, table) in [
                    ("notes", "notes_v2"),
                    ("folders", "note_folders_v2"),
                    ("tags", "note_tags_v2"),
                    ("revisions", "note_revisions_v2"),
                ] {
                    connection
                        .execute(&format!("DELETE FROM {table}"), [])
                        .map_err(|value| value.to_string())?;
                    for item in data
                        .get(key)
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                    {
                        write_entity(&connection, table, &item)?;
                    }
                }
                Ok(json!({ "ok": true }))
            }
        }
        _ => Err("不支持的笔记操作".to_owned()),
    })();

    match result {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(
            if message == "笔记不存在" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            },
            message,
        ),
    }
}
