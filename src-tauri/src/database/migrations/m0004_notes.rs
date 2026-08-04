use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

use crate::database::legacy::json_parser;
use crate::database::migration_runner::{Migration, MigrationContext, MigrationError, MigrationReport};

const LEGACY_NOTES_TABLE: &str = "legacy_notes_v2_json_v1";
const LEGACY_FOLDERS_TABLE: &str = "legacy_note_folders_v2_json_v1";
const LEGACY_TAGS_TABLE: &str = "legacy_note_tags_v2_json_v1";
const LEGACY_REVISIONS_TABLE: &str = "legacy_note_revisions_v2_json_v1";

const NOTE_TYPES: [&str; 8] = [
    "quick", "document", "daily", "habit_log", "workout_review", "expense_note",
    "weekly_review", "monthly_review",
];

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

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
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

/// m0004：笔记 schema 规范化。
pub struct M0004Notes;

impl Migration for M0004Notes {
    fn version(&self) -> i64 {
        4
    }

    fn name(&self) -> &'static str {
        "notes-normalization"
    }

    fn checksum(&self) -> &'static str {
        "m0004-notes-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        rename_legacy_tables(transaction)?;
        create_normalized_tables(transaction)?;

        let legacy_folders = if table_exists(transaction, LEGACY_FOLDERS_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_FOLDERS_TABLE)?
        } else {
            Vec::new()
        };
        let legacy_tags = if table_exists(transaction, LEGACY_TAGS_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_TAGS_TABLE)?
        } else {
            Vec::new()
        };
        let legacy_notes = if table_exists(transaction, LEGACY_NOTES_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_NOTES_TABLE)?
        } else {
            Vec::new()
        };
        let legacy_revisions = if table_exists(transaction, LEGACY_REVISIONS_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_REVISIONS_TABLE)?
        } else {
            Vec::new()
        };

        let mut folder_count = 0usize;
        for value in &legacy_folders {
            insert_folder(transaction, value)?;
            folder_count += 1;
        }

        // 标签按 (user_id, name) 去重，映射到规范 id。
        let mut tag_ids = HashMap::<(String, String), String>::new();
        let mut tag_count = 0usize;
        for value in &legacy_tags {
            let object = json_parser::as_object(value, "标签记录")?;
            let id = json_parser::string_field(object, "id")
                .filter(|id| !id.is_empty())
                .ok_or_else(|| format!("标签缺少 id: {}", value))?;
            let name = text(object, "name")
                .ok_or_else(|| format!("标签 {id} 缺少 name"))?;
            let user = json_parser::string_field(object, "userId")
                .filter(|value| !value.is_empty())
                .unwrap_or("local");
            let key = (user.to_owned(), name.clone());
            if let Some(existing) = tag_ids.get(&key) {
                let message = format!("标签「{name}」重复（{existing} 与 {id}），统一使用 {existing}");
                crate::database::migration_runner::record_issue(
                    transaction,
                    context,
                    "note_tags",
                    Some(id),
                    "warning",
                    &message,
                    Some(&value.to_string()),
                )?;
                continue;
            }
            insert_tag(transaction, value)?;
            tag_ids.insert(key, id.to_owned());
            tag_count += 1;
        }

        let mut note_count = 0usize;
        let mut relation_count = 0usize;
        let mut attachment_count = 0usize;
        let mut tag_relation_count = 0usize;
        for value in &legacy_notes {
            let (relations, attachments) =
                insert_note(transaction, value, context, &tag_ids)?;
            relation_count += relations;
            attachment_count += attachments;
            tag_relation_count += value
                .get("tags")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            note_count += 1;
        }

        let mut revision_count = 0usize;
        for value in &legacy_revisions {
            insert_revision(transaction, value)?;
            revision_count += 1;
        }

        // FTS5 尽力而为：失败不阻断主迁移。
        let fts_ok = build_fts(transaction)?;
        if !fts_ok {
            crate::database::migration_runner::record_issue(
                transaction,
                context,
                "notes_fts",
                None,
                "warning",
                "FTS5 不可用，笔记搜索回退到参数化 LIKE",
                None,
            )?;
        }

        validate_notes(
            transaction,
            &legacy_folders,
            &legacy_tags,
            &legacy_notes,
            &legacy_revisions,
            fts_ok,
        )?;

        let mut report = MigrationReport::default();
        report.migrated = folder_count + tag_count + note_count + revision_count;
        report.metrics.insert("note_folders".to_owned(), folder_count as i64);
        report.metrics.insert("note_tags".to_owned(), tag_count as i64);
        report.metrics.insert("notes".to_owned(), note_count as i64);
        report
            .metrics
            .insert("note_tag_relations".to_owned(), tag_relation_count as i64);
        report
            .metrics
            .insert("note_relations".to_owned(), relation_count as i64);
        report
            .metrics
            .insert("note_attachments".to_owned(), attachment_count as i64);
        report
            .metrics
            .insert("note_revisions".to_owned(), revision_count as i64);
        report.metrics.insert("notes_fts".to_owned(), fts_ok as i64);
        Ok(report)
    }
}

fn rename_legacy_tables(connection: &Connection) -> Result<(), MigrationError> {
    for (source, legacy) in [
        ("notes_v2", LEGACY_NOTES_TABLE),
        ("note_folders_v2", LEGACY_FOLDERS_TABLE),
        ("note_tags_v2", LEGACY_TAGS_TABLE),
        ("note_revisions_v2", LEGACY_REVISIONS_TABLE),
    ] {
        if table_exists(connection, source) && !table_exists(connection, legacy) {
            connection
                .execute(&format!("ALTER TABLE {source} RENAME TO {legacy}"), [])
                .map_err(|error| MigrationError {
                    version: 4,
                    message: format!("重命名 {source} 失败: {error}"),
                })?;
        }
    }
    Ok(())
}

fn create_normalized_tables(connection: &Connection) -> Result<(), MigrationError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS note_folders (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               name TEXT NOT NULL,
               icon TEXT NOT NULL DEFAULT 'folder',
               color TEXT NOT NULL DEFAULT '#5f7d70',
               sort_order INTEGER NOT NULL DEFAULT 0,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE TABLE IF NOT EXISTS notes (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               title TEXT,
               note_type TEXT NOT NULL DEFAULT 'document',
               folder_id TEXT REFERENCES note_folders(id),
               content_json TEXT NOT NULL,
               content_html TEXT NOT NULL DEFAULT '',
               content_text TEXT NOT NULL DEFAULT '',
               content_markdown TEXT NOT NULL DEFAULT '',
               summary TEXT NOT NULL DEFAULT '',
               is_pinned INTEGER NOT NULL DEFAULT 0 CHECK (is_pinned IN (0,1)),
               is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0,1)),
               is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
               ai_summary TEXT,
               ai_tags_json TEXT,
               embedding_status TEXT,
               last_ai_processed_at TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_notes_folder
               ON notes(folder_id, deleted_at);
             CREATE INDEX IF NOT EXISTS idx_notes_updated
               ON notes(updated_at DESC);
             CREATE TABLE IF NOT EXISTS note_tags (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               name TEXT NOT NULL,
               color TEXT NOT NULL DEFAULT '#5f7d70',
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE UNIQUE INDEX IF NOT EXISTS uq_note_tags_name
               ON note_tags(user_id, name) WHERE deleted_at IS NULL;
             CREATE TABLE IF NOT EXISTS note_tag_relations (
               note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
               tag_id TEXT NOT NULL REFERENCES note_tags(id) ON DELETE CASCADE,
               created_at TEXT NOT NULL,
               PRIMARY KEY (note_id, tag_id)
             );
             CREATE TABLE IF NOT EXISTS note_relations (
               id TEXT PRIMARY KEY,
               note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
               entity_type TEXT NOT NULL,
               entity_id TEXT NOT NULL,
               relation_type TEXT NOT NULL DEFAULT 'reference',
               created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_note_relations_note
               ON note_relations(note_id);
             CREATE TABLE IF NOT EXISTS note_attachments (
               id TEXT PRIMARY KEY,
               note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
               file_name TEXT NOT NULL,
               original_name TEXT NOT NULL,
               mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
               file_size INTEGER NOT NULL DEFAULT 0,
               storage_path TEXT,
               created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_note_attachments_note
               ON note_attachments(note_id);
             CREATE TABLE IF NOT EXISTS note_revisions (
               id TEXT PRIMARY KEY,
               note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
               revision_version INTEGER NOT NULL,
               title TEXT,
               content_json TEXT NOT NULL,
               content_html TEXT NOT NULL DEFAULT '',
               content_markdown TEXT NOT NULL DEFAULT '',
               created_at TEXT NOT NULL,
               UNIQUE(note_id, revision_version)
             );",
        )
        .map_err(|error| MigrationError {
            version: 4,
            message: format!("创建笔记规范化表失败: {error}"),
        })
}

fn insert_folder(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "文件夹记录")?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| format!("文件夹缺少 id: {}", value))?;
    let name = text(object, "name").ok_or_else(|| format!("文件夹 {id} 缺少 name"))?;
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
            rusqlite::params![
                id,
                name,
                text(object, "icon").unwrap_or_else(|| "folder".to_owned()),
                text(object, "color").unwrap_or_else(|| "#5f7d70".to_owned()),
                object.get("sortOrder").and_then(Value::as_i64).unwrap_or(0),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp)
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn insert_tag(connection: &Connection, value: &Value) -> Result<(), String> {
    let object = json_parser::as_object(value, "标签记录")?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| format!("标签缺少 id: {}", value))?;
    let name = text(object, "name").ok_or_else(|| format!("标签 {id} 缺少 name"))?;
    let stamp = now();
    connection
        .execute(
            "INSERT INTO note_tags(
               id, user_id, name, color, created_at, updated_at, deleted_at, version,
               modified_by_device
             ) VALUES(?1,'local',?2,?3,?4,?5,NULL,1,NULL)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, color=excluded.color, updated_at=excluded.updated_at",
            rusqlite::params![
                id,
                name,
                text(object, "color").unwrap_or_else(|| "#5f7d70".to_owned()),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or(stamp)
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn insert_note(
    connection: &Transaction,
    value: &Value,
    context: &MigrationContext,
    tag_ids: &HashMap<(String, String), String>,
) -> Result<(usize, usize), MigrationError> {
    let object = json_parser::as_object(value, "笔记记录")?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 4, message: format!("笔记缺少 id: {}", value) })?;
    let note_type = json_parser::string_field(object, "noteType")
        .unwrap_or("document");
    if !NOTE_TYPES.contains(&note_type) {
        return Err(MigrationError {
            version: 4,
            message: format!("笔记 {id} 类型不合法: {note_type}"),
        });
    }
    let folder_id = json_parser::string_field(object, "folderId");
    let resolved_folder = folder_id
        .filter(|value| !value.is_empty())
        .filter(|folder_id| {
            connection
                .query_row(
                    "SELECT 1 FROM note_folders WHERE id=?1 AND deleted_at IS NULL",
                    [folder_id],
                    |_| Ok(()),
                )
                .optional()
                .ok()
                .flatten()
                .is_some()
        });
    if folder_id.is_some() && resolved_folder.is_none() {
        let _ = crate::database::migration_runner::record_issue(
            connection,
            context,
            "notes",
            Some(id),
            "warning",
            &format!("笔记 {id} 引用的文件夹 {folder_id:?} 不存在，folder_id 置空"),
            Some(&value.to_string()),
        );
    }
    let content_json = object
        .get("contentJson")
        .cloned()
        .unwrap_or_else(|| json!({ "type": "doc", "content": [] }));
    let version = object.get("version").and_then(Value::as_i64).unwrap_or(1).max(1);
    let stamp = now();
    connection
        .execute(
            "INSERT INTO notes(
               id, user_id, title, note_type, folder_id, content_json, content_html,
               content_text, content_markdown, summary, is_pinned, is_favorite, is_archived,
               ai_summary, ai_tags_json, embedding_status, last_ai_processed_at,
               created_at, updated_at, deleted_at, version, modified_by_device
             ) VALUES(?1,'local',?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,NULL)
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
            rusqlite::params![
                id,
                text(object, "title"),
                note_type,
                resolved_folder,
                content_json.to_string(),
                text(object, "contentHtml").unwrap_or_default(),
                text(object, "contentText").unwrap_or_default(),
                text(object, "contentMarkdown").unwrap_or_default(),
                text(object, "summary").unwrap_or_default(),
                bool_value(object, "isPinned"),
                bool_value(object, "isFavorite"),
                bool_value(object, "isArchived"),
                text(object, "aiSummary"),
                object
                    .get("aiTags")
                    .map(Value::to_string),
                text(object, "embeddingStatus"),
                text(object, "lastAiProcessedAt"),
                text(object, "createdAt").unwrap_or_else(|| stamp.clone()),
                text(object, "updatedAt").unwrap_or_else(|| stamp.clone()),
                object
                    .get("deletedAt")
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str),
                version
            ],
        )
        .map_err(|error| MigrationError { version: 4, message: error.to_string() })?;

        // 标签关系（按规范 tag id）。
        let tags = object.get("tags").and_then(Value::as_array).cloned().unwrap_or_default();
        for tag in tags {
            let Some(tag_id) = tag.get("id").and_then(Value::as_str) else {
                continue;
            };
            let tag_name = tag.get("name").and_then(Value::as_str).unwrap_or_default();
            let canonical = tag_ids
                .iter()
                .find(|((_, name), _)| name == tag_name)
                .map(|(_, id)| id.clone())
                .unwrap_or_else(|| tag_id.to_owned());
            connection
                .execute(
                    "INSERT OR IGNORE INTO note_tag_relations(note_id, tag_id, created_at)
                     VALUES(?1,?2,?3)",
                    rusqlite::params![id, canonical, now()],
                )
                .map_err(|error| MigrationError { version: 4, message: error.to_string() })?;
        }

        // 业务关系。
        let relations = object.get("relations").and_then(Value::as_array).cloned().unwrap_or_default();
        for relation in &relations {
            let Some(relation_id) = relation.get("id").and_then(Value::as_str) else {
                continue;
            };
            connection
                .execute(
                    "INSERT OR IGNORE INTO note_relations(
                       id, note_id, entity_type, entity_id, relation_type, created_at
                     ) VALUES(?1,?2,?3,?4,?5,?6)",
                    rusqlite::params![
                        relation_id,
                        id,
                        relation.get("entityType").and_then(Value::as_str).unwrap_or("project"),
                        relation.get("entityId").and_then(Value::as_str).unwrap_or(""),
                        relation.get("relationType").and_then(Value::as_str).unwrap_or("reference"),
                        relation.get("createdAt").and_then(Value::as_str).unwrap_or(&now())
                    ],
                )
                .map_err(|error| MigrationError { version: 4, message: error.to_string() })?;
        }

        // 附件元数据（当前内嵌在笔记 JSON，独立成表以保留数据）。
        let attachments = object
            .get("attachments")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for attachment in &attachments {
            let Some(attachment_id) = attachment.get("id").and_then(Value::as_str) else {
                continue;
            };
            connection
                .execute(
                    "INSERT OR IGNORE INTO note_attachments(
                       id, note_id, file_name, original_name, mime_type, file_size,
                       storage_path, created_at
                     ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    rusqlite::params![
                        attachment_id,
                        id,
                        attachment.get("fileName").and_then(Value::as_str).unwrap_or("file"),
                        attachment.get("originalName").and_then(Value::as_str).unwrap_or("file"),
                        attachment.get("mimeType").and_then(Value::as_str).unwrap_or("application/octet-stream"),
                        attachment.get("fileSize").and_then(Value::as_i64).unwrap_or(0),
                        attachment.get("storagePath").and_then(Value::as_str),
                        attachment.get("createdAt").and_then(Value::as_str).unwrap_or(&now())
                    ],
                )
                .map_err(|error| MigrationError { version: 4, message: error.to_string() })?;
        }
        Ok((relations.len(), attachments.len()))
}

fn insert_revision(connection: &Connection, value: &Value) -> Result<(), MigrationError> {
    let object = json_parser::as_object(value, "版本记录")?;
    let id = json_parser::string_field(object, "id")
        .ok_or_else(|| MigrationError { version: 4, message: format!("版本缺少 id: {}", value) })?;
    let note_id = json_parser::string_field(object, "noteId")
        .ok_or_else(|| MigrationError { version: 4, message: format!("版本 {id} 缺少 noteId") })?;
    let parent_exists: bool = connection
        .query_row(
            "SELECT 1 FROM notes WHERE id=?1",
            [note_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(|error| MigrationError { version: 4, message: error.to_string() })?
        .is_some();
    if !parent_exists {
        return Err(MigrationError {
            version: 4,
            message: format!("版本 {id} 引用的笔记 {note_id} 不存在，无法建立外键"),
        });
    }
    let revision_version = object
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| MigrationError { version: 4, message: format!("版本 {id} 缺少 version") })?;
    connection
        .execute(
            "INSERT OR IGNORE INTO note_revisions(
               id, note_id, revision_version, title, content_json, content_html,
               content_markdown, created_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                id,
                note_id,
                revision_version,
                text(object, "title"),
                object
                    .get("contentJson")
                    .cloned()
                    .unwrap_or_else(|| json!({}))
                    .to_string(),
                text(object, "contentHtml").unwrap_or_default(),
                text(object, "contentMarkdown").unwrap_or_default(),
                text(object, "createdAt").unwrap_or_else(now)
            ],
        )
        .map(|_| ())
        .map_err(|error| MigrationError { version: 4, message: error.to_string() })
}

/// FTS5 尽力构建；不可用时返回 false。
fn build_fts(connection: &Connection) -> Result<bool, MigrationError> {
    let create = connection.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts
           USING fts5(title, content_text, summary, note_id UNINDEXED);",
    );
    if create.is_err() {
        return Ok(false);
    }
    let fill = connection.execute(
        "INSERT INTO notes_fts(rowid, title, content_text, summary, note_id)
         SELECT rowid, title, content_text, summary, id FROM notes",
        [],
    );
    match fill {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn validate_notes(
    connection: &Connection,
    legacy_folders: &[Value],
    legacy_tags: &[Value],
    legacy_notes: &[Value],
    legacy_revisions: &[Value],
    fts_ok: bool,
) -> Result<(), MigrationError> {
    let counts: (i64, i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM note_folders),
               (SELECT COUNT(*) FROM note_tags),
               (SELECT COUNT(*) FROM notes),
               (SELECT COUNT(*) FROM note_revisions)",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(|error| MigrationError { version: 4, message: error.to_string() })?;
    if counts.0 != legacy_folders.len() as i64 {
        return Err(MigrationError { version: 4, message: format!("文件夹数量不一致: 旧 {}，新 {}", legacy_folders.len(), counts.0) });
    }
    if counts.1 != legacy_tags.len() as i64 {
        return Err(MigrationError { version: 4, message: format!("标签数量不一致: 旧 {}，新 {}", legacy_tags.len(), counts.1) });
    }
    if counts.2 != legacy_notes.len() as i64 {
        return Err(MigrationError { version: 4, message: format!("笔记数量不一致: 旧 {}，新 {}", legacy_notes.len(), counts.2) });
    }
    if counts.3 != legacy_revisions.len() as i64 {
        return Err(MigrationError { version: 4, message: format!("版本数量不一致: 旧 {}，新 {}", legacy_revisions.len(), counts.3) });
    }
    if fts_ok {
        let fts_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM notes_fts",
                [],
                |row| row.get(0),
            )
            .map_err(|error| MigrationError { version: 4, message: error.to_string() })?;
        if fts_count != counts.2 {
            return Err(MigrationError { version: 4, message: format!("FTS 数量不一致: 笔记 {}，FTS {fts_count}", counts.2) });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::{M0001Framework, M0002Finance, M0003HabitsReviews};
    use crate::database::migration_runner::run;
    use rusqlite::Connection;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-notes-{label}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn seed_legacy_json(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE notes_v2(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE note_folders_v2(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE note_tags_v2(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE note_revisions_v2(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO note_folders_v2 VALUES('f1', ?1, '2026-01-01T00:00:00Z')",
                rusqlite::params![json!({
                    "id": "f1", "name": "工作", "icon": "briefcase", "color": "#416b5c",
                    "sortOrder": 0, "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-01T00:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO note_tags_v2 VALUES('t1', ?1, '2026-01-01T00:00:00Z')",
                rusqlite::params![json!({
                    "id": "t1", "name": "重要", "color": "#ff0000",
                    "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO notes_v2 VALUES('n1', ?1, '2026-01-02T00:00:00Z')",
                rusqlite::params![json!({
                    "id": "n1", "title": "会议记录", "noteType": "document", "folderId": "f1",
                    "contentJson": {"type": "doc", "content": []},
                    "contentHtml": "<p>hi</p>", "contentText": "hi", "contentMarkdown": "hi",
                    "summary": "摘要", "isPinned": false, "isFavorite": true, "isArchived": false,
                    "createdAt": "2026-01-02T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z",
                    "deletedAt": null, "version": 2,
                    "tags": [{"id": "t1", "name": "重要", "color": "#ff0000"}],
                    "relations": [{"id": "r1", "noteId": "n1", "entityType": "habit",
                        "entityId": "piano", "relationType": "reference",
                        "createdAt": "2026-01-02T00:00:00Z"}],
                    "attachments": [{"id": "a1", "noteId": "n1", "fileName": "x.pdf",
                        "originalName": "x.pdf", "mimeType": "application/pdf", "fileSize": 10,
                        "storagePath": "attachments/a1.pdf", "createdAt": "2026-01-02T00:00:00Z"}]
                })
                .to_string()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO note_revisions_v2 VALUES('rv1', ?1, '2026-01-02T00:00:00Z')",
                rusqlite::params![json!({
                    "id": "rv1", "noteId": "n1", "version": 1, "title": "会议记录",
                    "contentJson": {"type": "doc", "content": []},
                    "contentHtml": "<p>v1</p>", "contentMarkdown": "v1",
                    "createdAt": "2026-01-01T00:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
    }

    #[test]
    fn migrates_notes_with_relations_and_attachments() {
        let directory = temp_dir("migrate");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        seed_legacy_json(&connection);
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(M0001Framework),
            Box::new(M0002Finance),
            Box::new(M0003HabitsReviews),
            Box::new(M0004Notes),
        ];
        run(&mut connection, &context, &migrations).unwrap();

        let note_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(note_count, 1);
        let relation_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM note_relations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(relation_count, 1);
        let attachment_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM note_attachments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(attachment_count, 1);
        let tag_relation_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM note_tag_relations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(tag_relation_count, 1);
        let version: i64 = connection
            .query_row("SELECT version FROM notes WHERE id='n1'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);

        fs::remove_dir_all(&directory).ok();
    }
}
