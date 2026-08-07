//! 笔记 Repository：真实列与前端 DTO 的转换与读写。
//!
//! 列表查询不读取完整 `content_json`（详情才读取正文）；
//! FTS5 可用时使用全文检索，否则回退参数化 LIKE。

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::database::legacy::json_parser;

const NOTE_TYPES: [&str; 8] = [
    "quick",
    "document",
    "daily",
    "habit_log",
    "workout_review",
    "expense_note",
    "weekly_review",
    "monthly_review",
];

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn bool_value(object: &serde_json::Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}

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

/// 首次使用时种子化默认文件夹（仅当文件夹表为空时执行）。
pub fn seed_default_folders(connection: &Connection) -> Result<usize, String> {
    if !table_exists(connection, "note_folders") {
        return Ok(0);
    }
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM note_folders", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if count > 0 {
        return Ok(0);
    }
    let stamp = now();
    let defaults: [(&str, &str, &str); 6] = [
        ("工作", "briefcase", "#416b5c"),
        ("学习", "book", "#5975a4"),
        ("健身", "dumbbell", "#b06943"),
        ("生活", "home", "#887257"),
        ("财务", "wallet", "#8b654d"),
        ("项目", "folder", "#6c668f"),
    ];
    let mut seeded = 0;
    for (index, (name, icon, color)) in defaults.into_iter().enumerate() {
        connection
            .execute(
                "INSERT OR IGNORE INTO note_folders(
                   id, user_id, name, icon, color, sort_order, created_at, updated_at,
                   deleted_at, version, modified_by_device
                 ) VALUES(?1,'local',?2,?3,?4,?5,?6,?6,NULL,1,NULL)",
                params![id(), name, icon, color, index as i64, stamp],
            )
            .map_err(|error| error.to_string())?;
        seeded += 1;
    }
    Ok(seeded)
}

fn load_tags(connection: &Connection, note_id: &str) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(
            "SELECT t.id, t.name, t.color, t.created_at, t.updated_at
             FROM note_tags t
             JOIN note_tag_relations tr ON tr.tag_id = t.id
             WHERE tr.note_id = ?1 AND t.deleted_at IS NULL
             ORDER BY t.name",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([note_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "color": row.get::<_, String>(2)?,
                "createdAt": row.get::<_, String>(3)?,
                "updatedAt": row.get::<_, String>(4)?
            }))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn load_relations(connection: &Connection, note_id: &str) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, note_id, entity_type, entity_id, relation_type, created_at
             FROM note_relations WHERE note_id = ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([note_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "noteId": row.get::<_, String>(1)?,
                "entityType": row.get::<_, String>(2)?,
                "entityId": row.get::<_, String>(3)?,
                "relationType": row.get::<_, String>(4)?,
                "createdAt": row.get::<_, String>(5)?
            }))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn load_attachments(connection: &Connection, note_id: &str) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, note_id, file_name, original_name, mime_type, file_size, storage_path, created_at
             FROM note_attachments WHERE note_id = ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([note_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "noteId": row.get::<_, String>(1)?,
                "fileName": row.get::<_, String>(2)?,
                "originalName": row.get::<_, String>(3)?,
                "mimeType": row.get::<_, String>(4)?,
                "fileSize": row.get::<_, i64>(5)?,
                "storagePath": row.get::<_, Option<String>>(6)?,
                "createdAt": row.get::<_, String>(7)?
            }))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

const LIST_COLUMNS: &str = "id, title, note_type, folder_id, content_text, summary,
       is_pinned, is_favorite, is_archived, created_at, updated_at, deleted_at, version,
       ai_summary, embedding_status, last_ai_processed_at, ai_tags_json";

const FULL_COLUMNS: &str = "id, title, note_type, folder_id, content_json, content_html,
       content_text, content_markdown, summary, is_pinned, is_favorite, is_archived,
       created_at, updated_at, deleted_at, version, ai_summary, embedding_status,
       last_ai_processed_at, ai_tags_json";

fn note_dto_base(
    connection: &Connection,
    note_id: &str,
    title: Option<String>,
    note_type: String,
    folder_id: Option<String>,
    content_text: String,
    summary: String,
    is_pinned: bool,
    is_favorite: bool,
    is_archived: bool,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
    version: i64,
    ai_summary: Option<String>,
    embedding_status: Option<String>,
    last_ai_processed_at: Option<String>,
    ai_tags_json: Option<String>,
) -> Value {
    let tags = load_tags(connection, note_id).unwrap_or_default();
    let relations = load_relations(connection, note_id).unwrap_or_default();
    let attachments = load_attachments(connection, note_id).unwrap_or_default();
    let ai_tags = ai_tags_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null);
    json!({
        "id": note_id,
        "title": title,
        "noteType": note_type,
        "folderId": folder_id,
        "contentText": content_text,
        "summary": summary,
        "isPinned": is_pinned,
        "isFavorite": is_favorite,
        "isArchived": is_archived,
        "createdAt": created_at,
        "updatedAt": updated_at,
        "deletedAt": deleted_at,
        "version": version,
        "aiSummary": ai_summary,
        "embeddingStatus": embedding_status,
        "lastAiProcessedAt": last_ai_processed_at,
        "aiTags": ai_tags,
        "tags": tags,
        "relations": relations,
        "attachments": attachments
    })
}

fn note_list_from_row(connection: &Connection, row: &Row<'_>) -> rusqlite::Result<Value> {
    Ok(note_dto_base(
        connection,
        &row.get::<_, String>(0)?,
        row.get::<_, Option<String>>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, bool>(6)?,
        row.get::<_, bool>(7)?,
        row.get::<_, bool>(8)?,
        row.get::<_, String>(9)?,
        row.get::<_, String>(10)?,
        row.get::<_, Option<String>>(11)?,
        row.get::<_, i64>(12)?,
        row.get::<_, Option<String>>(13)?,
        row.get::<_, Option<String>>(14)?,
        row.get::<_, Option<String>>(15)?,
        row.get::<_, Option<String>>(16)?,
    ))
}

fn note_full_from_row(connection: &Connection, row: &Row<'_>) -> rusqlite::Result<Value> {
    let mut note = note_dto_base(
        connection,
        &row.get::<_, String>(0)?,
        row.get::<_, Option<String>>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, String>(6)?,
        row.get::<_, String>(8)?,
        row.get::<_, bool>(9)?,
        row.get::<_, bool>(10)?,
        row.get::<_, bool>(11)?,
        row.get::<_, String>(12)?,
        row.get::<_, String>(13)?,
        row.get::<_, Option<String>>(14)?,
        row.get::<_, i64>(15)?,
        row.get::<_, Option<String>>(16)?,
        row.get::<_, Option<String>>(17)?,
        row.get::<_, Option<String>>(18)?,
        row.get::<_, Option<String>>(19)?,
    );
    if let Some(object) = note.as_object_mut() {
        let content_json: String = row.get(4)?;
        let content_html: String = row.get(5)?;
        let content_markdown: String = row.get(7)?;
        object.insert(
            "contentJson".to_owned(),
            serde_json::from_str::<Value>(&content_json).unwrap_or(Value::Null),
        );
        object.insert("contentHtml".to_owned(), Value::String(content_html));
        object.insert(
            "contentMarkdown".to_owned(),
            Value::String(content_markdown),
        );
    }
    Ok(note)
}

/// 笔记列表（SQL 不选取 content_json 的权威列；DTO 只带摘要字段，详情才读取正文）。
pub fn list_notes(
    connection: &Connection,
    q: Option<&str>,
    scope: Option<&str>,
    folder_id: Option<&str>,
    tag_id: Option<&str>,
    note_type: Option<&str>,
    sort: Option<&str>,
    limit: usize,
) -> Result<Vec<Value>, String> {
    let scope = scope.unwrap_or("all");
    let mut conditions = Vec::<String>::new();
    let mut arguments = Vec::<Box<dyn rusqlite::ToSql>>::new();
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    conditions.push("t.user_id = ?".to_owned());
    arguments.push(Box::new(profile_id));

    conditions.push(match scope {
        "trash" => "t.deleted_at IS NOT NULL".to_owned(),
        _ => "t.deleted_at IS NULL".to_owned(),
    });
    if scope != "trash" && scope != "archived" {
        conditions.push("t.is_archived = 0".to_owned());
    } else if scope == "archived" {
        conditions.push("t.is_archived = 1".to_owned());
    }
    match scope {
        "favorite" => conditions.push("t.is_favorite = 1".to_owned()),
        "pinned" => conditions.push("t.is_pinned = 1".to_owned()),
        "quick" => conditions.push("t.note_type = 'quick'".to_owned()),
        _ => {}
    }
    if let Some(folder_id) = folder_id.filter(|value| !value.is_empty()) {
        conditions.push("t.folder_id = ?".to_owned());
        arguments.push(Box::new(folder_id.to_owned()));
    }
    if let Some(note_type) = note_type.filter(|value| !value.is_empty()) {
        conditions.push("t.note_type = ?".to_owned());
        arguments.push(Box::new(note_type.to_owned()));
    }
    if let Some(tag_id) = tag_id.filter(|value| !value.is_empty()) {
        conditions.push(
            "EXISTS (SELECT 1 FROM note_tag_relations tr WHERE tr.note_id = t.id AND tr.tag_id = ?)"
                .to_owned(),
        );
        arguments.push(Box::new(tag_id.to_owned()));
    }

    if let Some(term) = q.filter(|value| !value.is_empty()) {
        let fts_ids = fts_search(connection, term).map_err(|error| error.to_string())?;
        match &fts_ids {
            Some(ids) if !ids.is_empty() => {
                let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                conditions.push(format!("t.id IN ({placeholders})"));
                for value in ids {
                    arguments.push(Box::new(value.clone()));
                }
            }
            Some(_) => return Ok(Vec::new()),
            None => {
                conditions.push(
                    "(lower(t.title) LIKE ? OR lower(t.summary) LIKE ? OR lower(t.content_text) LIKE ?)"
                        .to_owned(),
                );
                let pattern = format!("%{}%", term.to_lowercase());
                for _ in 0..3 {
                    arguments.push(Box::new(pattern.clone()));
                }
            }
        }
    }

    let order = match sort {
        Some("created_asc") => "t.created_at ASC",
        Some("created_desc") => "t.created_at DESC",
        Some("title_asc") => "t.title ASC",
        Some("title_desc") => "t.title DESC",
        _ => "t.is_pinned DESC, t.updated_at DESC",
    };
    let limit = limit.clamp(1, 250) as i64;
    let sql = format!(
        "SELECT {LIST_COLUMNS} FROM notes t WHERE {} ORDER BY {order} LIMIT ?",
        conditions.join(" AND ")
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let mut bound: Vec<&dyn rusqlite::ToSql> = arguments
        .iter()
        .map(|value| value.as_ref() as &dyn rusqlite::ToSql)
        .collect();
    bound.push(&limit);
    let rows = statement
        .query_map(bound.as_slice(), |row| note_list_from_row(connection, row))
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

/// FTS5 搜索；不可用时返回 None。
fn fts_search(connection: &Connection, term: &str) -> Result<Option<Vec<String>>, String> {
    if !table_exists(connection, "notes_fts") {
        return Ok(None);
    }
    let query = format!("\"{}\"", term.replace('"', "\"\""));
    match connection.prepare("SELECT note_id FROM notes_fts WHERE notes_fts MATCH ?1") {
        Ok(mut statement) => {
            let rows = statement
                .query_map([query], |row| row.get::<_, String>(0))
                .map_err(|error| error.to_string())?;
            let ids = rows
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            Ok(Some(ids))
        }
        Err(_) => Ok(None),
    }
}

/// 笔记详情（含完整正文）。
pub fn get_note(connection: &Connection, note_id: &str) -> Result<Option<Value>, String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    let sql = format!("SELECT {FULL_COLUMNS} FROM notes WHERE id = ?1 AND user_id=?2");
    let value = connection
        .query_row(&sql, rusqlite::params![note_id, profile_id], |row| {
            note_full_from_row(connection, row)
        })
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(value)
}

/// 文件夹 + 标签（含使用次数）。
pub fn meta(connection: &Connection) -> Result<Value, String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, icon, color, sort_order, created_at, updated_at
             FROM note_folders WHERE deleted_at IS NULL AND user_id=?1 ORDER BY sort_order, name",
        )
        .map_err(|error| error.to_string())?;
    let folders = statement
        .query_map([profile_id.clone()], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "icon": row.get::<_, String>(2)?,
                "color": row.get::<_, String>(3)?,
                "sortOrder": row.get::<_, i64>(4)?,
                "createdAt": row.get::<_, String>(5)?,
                "updatedAt": row.get::<_, String>(6)?
            }))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let tags;
    {
        let mut statement = connection
            .prepare(
                "SELECT t.id, t.name, t.color, t.created_at, t.updated_at,
                        (SELECT COUNT(*) FROM note_tag_relations tr WHERE tr.tag_id = t.id) AS usage_count
                 FROM note_tags t WHERE t.deleted_at IS NULL AND t.user_id=?1 ORDER BY t.name",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([profile_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "color": row.get::<_, String>(2)?,
                    "createdAt": row.get::<_, String>(3)?,
                    "updatedAt": row.get::<_, String>(4)?,
                    "usageCount": row.get::<_, i64>(5)?
                }))
            })
            .map_err(|error| error.to_string())?;
        tags = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
    }
    Ok(json!({ "folders": folders, "tags": tags }))
}

/// 笔记版本列表（倒序，最多 20 条）。
pub fn list_revisions(connection: &Connection, note_id: &str) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, note_id, revision_version, title, content_json, content_html,
                    content_markdown, created_at
             FROM note_revisions WHERE note_id = ?1
             ORDER BY revision_version DESC LIMIT 20",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([note_id], |row| {
            let content_json: String = row.get(4)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "noteId": row.get::<_, String>(1)?,
                "version": row.get::<_, i64>(2)?,
                "title": row.get::<_, Option<String>>(3)?,
                "contentJson": serde_json::from_str::<Value>(&content_json).unwrap_or(Value::Null),
                "contentHtml": row.get::<_, String>(5)?,
                "contentMarkdown": row.get::<_, String>(6)?,
                "createdAt": row.get::<_, String>(7)?
            }))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

/// 完整笔记备份（含正文）。
pub fn backup(connection: &Connection) -> Result<Value, String> {
    let notes = list_notes(connection, None, Some("all"), None, None, None, None, 250)?;
    let mut full_notes = Vec::new();
    for note in notes {
        let note_id = note.get("id").and_then(Value::as_str).unwrap_or_default();
        if let Some(full) = get_note(connection, note_id)? {
            full_notes.push(full);
        }
    }
    let meta_value = meta(connection)?;
    let revisions = {
        let mut statement = connection
            .prepare(
                "SELECT id, note_id, revision_version, title, content_json, content_html,
                        content_markdown, created_at
                 FROM note_revisions ORDER BY note_id, revision_version",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                let content_json: String = row.get(4)?;
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "noteId": row.get::<_, String>(1)?,
                    "version": row.get::<_, i64>(2)?,
                    "title": row.get::<_, Option<String>>(3)?,
                    "contentJson": serde_json::from_str::<Value>(&content_json).unwrap_or(Value::Null),
                    "contentHtml": row.get::<_, String>(5)?,
                    "contentMarkdown": row.get::<_, String>(6)?,
                    "createdAt": row.get::<_, String>(7)?
                }))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    Ok(json!({
        "format": "lifetrace-notes",
        "version": 2,
        "createdAt": now(),
        "notes": full_notes,
        "folders": meta_value["folders"],
        "tags": meta_value["tags"],
        "revisions": revisions
    }))
}

fn save_revision(connection: &Connection, note: &Value) -> Result<(), String> {
    let note_id = note.get("id").and_then(Value::as_str).unwrap_or_default();
    let version = note.get("version").and_then(Value::as_i64).unwrap_or(1);
    let revision_id = id();
    connection
        .execute(
            "INSERT OR IGNORE INTO note_revisions(
               id, note_id, revision_version, title, content_json, content_html,
               content_markdown, created_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                revision_id,
                note_id,
                version,
                note.get("title").and_then(Value::as_str),
                note.get("contentJson")
                    .cloned()
                    .unwrap_or_else(|| json!({}))
                    .to_string(),
                note.get("contentHtml")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                note.get("contentMarkdown")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                now()
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn replace_tag_relations(
    connection: &Connection,
    note_id: &str,
    tag_ids: &[Value],
) -> Result<(), String> {
    connection
        .execute(
            "DELETE FROM note_tag_relations WHERE note_id = ?1",
            [note_id],
        )
        .map_err(|error| error.to_string())?;
    let stamp = now();
    for tag_id in tag_ids.iter().filter_map(Value::as_str) {
        connection
            .execute(
                "INSERT OR IGNORE INTO note_tag_relations(note_id, tag_id, created_at) VALUES(?1,?2,?3)",
                params![note_id, tag_id, stamp],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn replace_relations(
    connection: &Connection,
    note_id: &str,
    relations: &[Value],
) -> Result<(), String> {
    connection
        .execute("DELETE FROM note_relations WHERE note_id = ?1", [note_id])
        .map_err(|error| error.to_string())?;
    let stamp = now();
    for relation in relations {
        let relation_id = relation.get("id").and_then(Value::as_str).unwrap_or(&stamp);
        connection
            .execute(
                "INSERT OR IGNORE INTO note_relations(
                   id, note_id, entity_type, entity_id, relation_type, created_at
                 ) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    relation_id,
                    note_id,
                    relation
                        .get("entityType")
                        .and_then(Value::as_str)
                        .unwrap_or("project"),
                    relation
                        .get("entityId")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    relation
                        .get("relationType")
                        .and_then(Value::as_str)
                        .unwrap_or("reference"),
                    relation
                        .get("createdAt")
                        .and_then(Value::as_str)
                        .unwrap_or(&stamp)
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn resolve_tag_ids(connection: &Connection, tag_ids: &[Value]) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    for tag_id in tag_ids.iter().filter_map(Value::as_str) {
        let exists: bool = connection
            .query_row(
                "SELECT 1 FROM note_tags WHERE id=?1 AND deleted_at IS NULL",
                [tag_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .is_some();
        if exists {
            result.push(Value::String(tag_id.to_owned()));
        }
    }
    Ok(result)
}

/// 创建或更新笔记。
pub fn save_note(
    connection: &Connection,
    input: &Value,
    is_update: bool,
    create_revision: bool,
) -> Result<Value, String> {
    let owned = crate::database::profile::assign_active_owner(connection, input)?;
    let input = &owned;
    let object = json_parser::as_object(input, "笔记数据")?;
    let note_id = text(object, "id").unwrap_or_else(id);
    let existing = get_note(connection, &note_id)?;
    if is_update && existing.is_none() {
        return Err("笔记不存在".to_owned());
    }
    if existing.is_some() && create_revision {
        save_revision(connection, existing.as_ref().expect("checked"))?;
    }
    let note_type = text(object, "noteType").unwrap_or_else(|| "document".to_owned());
    if !NOTE_TYPES.contains(&note_type.as_str()) {
        return Err(format!("笔记类型不合法: {note_type}"));
    }
    let folder_id = text(object, "folderId");
    let version = existing
        .as_ref()
        .and_then(|value| value.get("version"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
        + 1;
    let created_at = text(object, "createdAt").unwrap_or_else(now);
    let updated_at = now();
    let deleted_at = object
        .get("deletedAt")
        .filter(|value| !value.is_null())
        .and_then(Value::as_str);
    let tag_ids = resolve_tag_ids(
        connection,
        object
            .get("tagIds")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    )?;
    let relations = object
        .get("relations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    connection
        .execute(
            "INSERT INTO notes(
               id, user_id, title, note_type, folder_id, content_json, content_html,
               content_text, content_markdown, summary, is_pinned, is_favorite, is_archived,
               ai_summary, ai_tags_json, embedding_status, last_ai_processed_at,
               created_at, updated_at, deleted_at, version, modified_by_device
             ) VALUES(?1,?21,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,NULL)
             ON CONFLICT(id) DO UPDATE SET
               title=excluded.title, note_type=excluded.note_type, folder_id=excluded.folder_id,
               content_json=excluded.content_json, content_html=excluded.content_html,
               content_text=excluded.content_text, content_markdown=excluded.content_markdown,
               summary=excluded.summary, is_pinned=excluded.is_pinned,
               is_favorite=excluded.is_favorite, is_archived=excluded.is_archived,
               ai_summary=excluded.ai_summary, ai_tags_json=excluded.ai_tags_json,
               embedding_status=excluded.embedding_status,
               last_ai_processed_at=excluded.last_ai_processed_at,
               updated_at=excluded.updated_at, deleted_at=excluded.deleted_at,
               version=excluded.version",
            params![
                note_id,
                text(object, "title"),
                note_type,
                folder_id,
                object
                    .get("contentJson")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "doc", "content": [] }))
                    .to_string(),
                text(object, "contentHtml").unwrap_or_default(),
                text(object, "contentText").unwrap_or_default(),
                text(object, "contentMarkdown").unwrap_or_default(),
                text(object, "summary").unwrap_or_default(),
                bool_value(object, "isPinned"),
                bool_value(object, "isFavorite"),
                bool_value(object, "isArchived"),
                text(object, "aiSummary"),
                object.get("aiTags").map(Value::to_string),
                text(object, "embeddingStatus"),
                text(object, "lastAiProcessedAt"),
                created_at,
                updated_at,
                deleted_at,
                version,
                text(object, "userId").ok_or_else(|| "笔记缺少当前资料归属".to_owned())?
            ],
        )
        .map_err(|error| error.to_string())?;
    replace_tag_relations(connection, &note_id, &tag_ids)?;
    replace_relations(connection, &note_id, &relations)?;
    refresh_fts_note(connection, &note_id).ok();
    Ok(get_note(connection, &note_id)?.ok_or_else(|| "笔记保存失败".to_owned())?)
}

/// 置入/移出回收站（软删除）。
pub fn set_deleted(connection: &Connection, note_id: &str, deleted: bool) -> Result<(), String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE notes SET deleted_at = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![if deleted { Some(&stamp) } else { None }, stamp, note_id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())?;
    refresh_fts_note(connection, note_id).ok();
    Ok(())
}

/// 物理删除笔记（关系、附件、版本、FTS 由级联或显式清理）。
pub fn delete_note(connection: &Connection, note_id: &str) -> Result<(), String> {
    connection
        .execute("DELETE FROM notes WHERE id = ?1", [note_id])
        .map_err(|error| error.to_string())?;
    if table_exists(connection, "notes_fts") {
        let _ = connection.execute("DELETE FROM notes_fts WHERE note_id = ?1", [note_id]);
    }
    Ok(())
}

/// 复制笔记。
pub fn duplicate_note(connection: &Connection, note_id: &str) -> Result<Value, String> {
    let existing = get_note(connection, note_id)?.ok_or_else(|| "笔记不存在".to_owned())?;
    let new_id = id();
    let title = existing
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("无标题笔记");
    let stamp = now();
    let mut copy = existing.clone();
    if let Some(object) = copy.as_object_mut() {
        object.insert("id".to_owned(), json!(new_id));
        object.insert("title".to_owned(), json!(format!("{title} · 副本")));
        object.insert("isPinned".to_owned(), json!(false));
        object.insert("createdAt".to_owned(), json!(stamp.clone()));
        object.insert("updatedAt".to_owned(), json!(stamp.clone()));
        object.insert("deletedAt".to_owned(), Value::Null);
        object.insert("version".to_owned(), json!(1));
    }
    let mut tag_ids = Vec::new();
    if let Some(tags) = copy.get("tags").and_then(Value::as_array) {
        for tag in tags {
            if let Some(tag_id) = tag.get("id").and_then(Value::as_str) {
                tag_ids.push(Value::String(tag_id.to_owned()));
            }
        }
    }
    let mut relations = Vec::new();
    if let Some(items) = copy.get("relations").and_then(Value::as_array) {
        for relation in items {
            let mut item = relation.clone();
            if let Some(object) = item.as_object_mut() {
                object.insert("id".to_owned(), json!(id()));
                object.insert("noteId".to_owned(), json!(new_id));
                object.insert("createdAt".to_owned(), json!(stamp));
            }
            relations.push(item);
        }
    }
    let mut input = copy.clone();
    if let Some(object) = input.as_object_mut() {
        object.insert("tagIds".to_owned(), Value::Array(tag_ids));
        object.insert("relations".to_owned(), Value::Array(relations));
        object.insert("attachments".to_owned(), Value::Array(Vec::new()));
    }
    save_note(connection, &input, false, false)?;
    if let Some(attachments) = existing.get("attachments").and_then(Value::as_array) {
        for attachment in attachments {
            let attachment_id = id();
            connection
                .execute(
                    "INSERT INTO note_attachments(
                       id, note_id, file_name, original_name, mime_type, file_size,
                       storage_path, created_at
                     ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        attachment_id,
                        new_id,
                        attachment
                            .get("fileName")
                            .and_then(Value::as_str)
                            .unwrap_or("file"),
                        attachment
                            .get("originalName")
                            .and_then(Value::as_str)
                            .unwrap_or("file"),
                        attachment
                            .get("mimeType")
                            .and_then(Value::as_str)
                            .unwrap_or("application/octet-stream"),
                        attachment
                            .get("fileSize")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        attachment.get("storagePath").and_then(Value::as_str),
                        attachment
                            .get("createdAt")
                            .and_then(Value::as_str)
                            .unwrap_or(&now())
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    get_note(connection, &new_id)?.ok_or_else(|| "复制笔记失败".to_owned())
}

/// 保存文件夹，返回 id。
pub fn save_folder(connection: &Connection, input: &Value) -> Result<String, String> {
    let owned = crate::database::profile::assign_active_owner(connection, input)?;
    let input = &owned;
    let object = json_parser::as_object(input, "文件夹数据")?;
    let folder_id = text(object, "id").unwrap_or_else(id);
    let name = text(object, "name").ok_or_else(|| "文件夹缺少 name".to_owned())?;
    let stamp = now();
    connection
        .execute(
            "INSERT INTO note_folders(
               id, user_id, name, icon, color, sort_order, created_at, updated_at,
               deleted_at, version, modified_by_device
             ) VALUES(?1,'local',?2,?3,?4,?5,?6,?7,NULL,1,NULL)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, icon=excluded.icon, color=excluded.color,
               sort_order=excluded.sort_order, updated_at=excluded.updated_at",
            params![
                folder_id,
                name,
                text(object, "icon").unwrap_or_else(|| "folder".to_owned()),
                text(object, "color").unwrap_or_else(|| "#5f7d70".to_owned()),
                object.get("sortOrder").and_then(Value::as_i64).unwrap_or(0),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp)
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(folder_id)
}

/// 删除文件夹：软删除并清空笔记引用。
pub fn delete_folder(connection: &Connection, folder_id: &str) -> Result<(), String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE note_folders SET deleted_at=?1, updated_at=?1 WHERE id=?2",
            params![stamp, folder_id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE notes SET folder_id=NULL, updated_at=?1 WHERE folder_id=?2",
            params![stamp, folder_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// 保存标签，返回 id。
pub fn save_tag(connection: &Connection, input: &Value) -> Result<String, String> {
    let owned = crate::database::profile::assign_active_owner(connection, input)?;
    let input = &owned;
    let object = json_parser::as_object(input, "标签数据")?;
    let tag_id = text(object, "id").unwrap_or_else(id);
    let name = text(object, "name").ok_or_else(|| "标签缺少 name".to_owned())?;
    let stamp = now();
    connection
        .execute(
            "INSERT INTO note_tags(
               id, user_id, name, color, created_at, updated_at, deleted_at, version,
               modified_by_device
             ) VALUES(?1,'local',?2,?3,?4,?5,NULL,1,NULL)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, color=excluded.color, updated_at=excluded.updated_at",
            params![
                tag_id,
                name,
                text(object, "color").unwrap_or_else(|| "#5f7d70".to_owned()),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp)
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(tag_id)
}

/// 删除标签：软删除并移除关系。
pub fn delete_tag(connection: &Connection, tag_id: &str) -> Result<(), String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE note_tags SET deleted_at=?1, updated_at=?1 WHERE id=?2",
            params![stamp, tag_id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute("DELETE FROM note_tag_relations WHERE tag_id = ?1", [tag_id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// 从版本恢复笔记内容。
pub fn restore_revision(connection: &Connection, revision_id: &str) -> Result<Value, String> {
    let revision: Option<(String, i64, Option<String>, String, String, String)> = connection
        .query_row(
            "SELECT note_id, revision_version, title, content_json, content_html, content_markdown
             FROM note_revisions WHERE id = ?1",
            [revision_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (note_id, _, title, content_json, content_html, content_markdown) =
        revision.ok_or_else(|| "历史版本不存在".to_owned())?;
    let note = get_note(connection, &note_id)?.ok_or_else(|| "笔记不存在".to_owned())?;
    save_revision(connection, &note)?;
    let stamp = now();
    connection
        .execute(
            "UPDATE notes SET title=?1, content_json=?2, content_html=?3, content_markdown=?4,
                    content_text=?5, updated_at=?6, version=version+1
             WHERE id=?7",
            params![
                title,
                content_json,
                content_html,
                content_markdown,
                content_markdown,
                stamp,
                note_id
            ],
        )
        .map_err(|error| error.to_string())?;
    refresh_fts_note(connection, &note_id).ok();
    get_note(connection, &note_id)?.ok_or_else(|| "笔记不存在".to_owned())
}

/// 记录附件元数据。
pub fn record_attachment(
    connection: &Connection,
    note_id: &str,
    file: &Value,
) -> Result<(), String> {
    let object = json_parser::as_object(file, "附件数据")?;
    let attachment_id = text(object, "id").unwrap_or_else(id);
    connection
        .execute(
            "INSERT OR IGNORE INTO note_attachments(
               id, note_id, file_name, original_name, mime_type, file_size, storage_path, created_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                attachment_id,
                note_id,
                text(object, "fileName").unwrap_or_else(|| "file".to_owned()),
                text(object, "originalName").unwrap_or_else(|| "file".to_owned()),
                text(object, "mimeType").unwrap_or_else(|| "application/octet-stream".to_owned()),
                object.get("fileSize").and_then(Value::as_i64).unwrap_or(0),
                text(object, "storagePath"),
                text(object, "createdAt").unwrap_or_else(now)
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// 删除附件元数据。
pub fn delete_attachment(connection: &Connection, attachment_id: &str) -> Result<(), String> {
    connection
        .execute(
            "DELETE FROM note_attachments WHERE id = ?1",
            [attachment_id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// 从 JSON 备份恢复（旧 lifetrace-notes 格式兼容）。
pub fn restore_backup(connection: &mut Connection, data: &Value) -> Result<(), String> {
    let object = json_parser::as_object(data, "备份数据")?;
    if object.get("format").and_then(Value::as_str) != Some("lifetrace-notes") {
        return Err("不支持的笔记备份格式".to_owned());
    }
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if table_exists(&transaction, "notes_fts") {
        let _ = transaction.execute("DELETE FROM notes_fts", []);
    }
    transaction
        .execute("DELETE FROM note_revisions", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM note_attachments", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM note_relations", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM note_tag_relations", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM notes", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM note_tags", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM note_folders", [])
        .map_err(|error| error.to_string())?;
    for folder in object
        .get("folders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        save_folder(&transaction, &folder)?;
    }
    for tag in object
        .get("tags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        save_tag(&transaction, &tag)?;
    }
    for revision in object
        .get("revisions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let note_id = revision
            .get("noteId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let version = revision.get("version").and_then(Value::as_i64).unwrap_or(1);
        transaction
            .execute(
                "INSERT OR IGNORE INTO note_revisions(
                   id, note_id, revision_version, title, content_json, content_html,
                   content_markdown, created_at
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    revision
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    note_id,
                    version,
                    revision.get("title").and_then(Value::as_str),
                    revision
                        .get("contentJson")
                        .cloned()
                        .unwrap_or_else(|| json!({}))
                        .to_string(),
                    revision
                        .get("contentHtml")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    revision
                        .get("contentMarkdown")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    revision
                        .get("createdAt")
                        .and_then(Value::as_str)
                        .unwrap_or(&now())
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    for note in object
        .get("notes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        save_note(&transaction, &note, false, false)?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(())
}

/// 重建单条笔记的 FTS 行（尽力而为）。
fn refresh_fts_note(connection: &Connection, note_id: &str) -> Result<(), String> {
    if !table_exists(connection, "notes_fts") {
        return Ok(());
    }
    let row: Option<(String, String, String)> = connection
        .query_row(
            "SELECT title, content_text, summary FROM notes WHERE id=?1",
            [note_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let _ = connection.execute("DELETE FROM notes_fts WHERE note_id=?1", [note_id]);
    if let Some((title, content_text, summary)) = row {
        let _ = connection.execute(
            "INSERT INTO notes_fts(title, content_text, summary, note_id) VALUES(?1,?2,?3,?4)",
            params![title, content_text, summary, note_id],
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn schema(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE note_folders(
                   id TEXT PRIMARY KEY, user_id TEXT, name TEXT, icon TEXT, color TEXT,
                   sort_order INTEGER, created_at TEXT, updated_at TEXT, deleted_at TEXT,
                   version INTEGER, modified_by_device TEXT
                 );
                 CREATE TABLE notes(
                   id TEXT PRIMARY KEY, user_id TEXT, title TEXT, note_type TEXT, folder_id TEXT,
                   content_json TEXT, content_html TEXT, content_text TEXT, content_markdown TEXT,
                   summary TEXT, is_pinned INTEGER, is_favorite INTEGER, is_archived INTEGER,
                   ai_summary TEXT, ai_tags_json TEXT, embedding_status TEXT,
                   last_ai_processed_at TEXT, created_at TEXT, updated_at TEXT, deleted_at TEXT,
                   version INTEGER, modified_by_device TEXT
                 );
                 CREATE TABLE note_tags(
                   id TEXT PRIMARY KEY, user_id TEXT, name TEXT, color TEXT, created_at TEXT,
                   updated_at TEXT, deleted_at TEXT, version INTEGER, modified_by_device TEXT
                 );
                 CREATE TABLE note_tag_relations(
                   note_id TEXT, tag_id TEXT, created_at TEXT, PRIMARY KEY(note_id, tag_id)
                 );
                 CREATE TABLE note_relations(
                   id TEXT PRIMARY KEY, note_id TEXT, entity_type TEXT, entity_id TEXT,
                   relation_type TEXT, created_at TEXT
                 );
                 CREATE TABLE note_attachments(
                   id TEXT PRIMARY KEY, note_id TEXT, file_name TEXT, original_name TEXT,
                   mime_type TEXT, file_size INTEGER, storage_path TEXT, created_at TEXT
                 );
                 CREATE TABLE note_revisions(
                   id TEXT PRIMARY KEY, note_id TEXT, revision_version INTEGER, title TEXT,
                   content_json TEXT, content_html TEXT, content_markdown TEXT, created_at TEXT
                 );",
            )
            .unwrap();
    }

    #[test]
    fn list_does_not_include_content_json() {
        let connection = Connection::open_in_memory().unwrap();
        schema(&connection);
        save_folder(&connection, &json!({"name": "工作"})).unwrap();
        save_note(
            &connection,
            &json!({
                "title": "会议", "noteType": "document", "contentJson": {"type": "doc"},
                "contentHtml": "<p>hi</p>", "contentText": "hi", "contentMarkdown": "hi",
                "summary": "摘要"
            }),
            false,
            false,
        )
        .unwrap();
        let list = list_notes(&connection, None, Some("all"), None, None, None, None, 20).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].get("contentJson").is_none());
        assert_eq!(list[0]["summary"], json!("摘要"));
        let detail = get_note(&connection, list[0]["id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        assert!(detail.get("contentJson").is_some());
    }

    #[test]
    fn update_bumps_version_and_keeps_attachments() {
        let connection = Connection::open_in_memory().unwrap();
        schema(&connection);
        let note = save_note(
            &connection,
            &json!({
                "title": "a", "noteType": "quick", "contentJson": {"type": "doc"},
                "contentHtml": "", "contentText": "", "contentMarkdown": "", "summary": ""
            }),
            false,
            false,
        )
        .unwrap();
        let note_id = note["id"].as_str().unwrap().to_owned();
        record_attachment(
            &connection,
            &note_id,
            &json!({"fileName": "x.pdf", "originalName": "x.pdf", "mimeType": "application/pdf", "fileSize": 3}),
        )
        .unwrap();
        let mut updated = note.clone();
        if let Some(object) = updated.as_object_mut() {
            object.insert("title".to_owned(), json!("b"));
            object.insert("tagIds".to_owned(), json!([]));
            object.insert("relations".to_owned(), json!([]));
        }
        let saved = save_note(&connection, &updated, true, false).unwrap();
        assert_eq!(saved["version"], json!(2));
        assert_eq!(saved["title"], json!("b"));
        assert_eq!(saved["attachments"].as_array().map(Vec::len), Some(1));
    }
}
