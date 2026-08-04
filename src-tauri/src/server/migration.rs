use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

const JSON_TABLES: [(&str, &str); 13] = [
    ("activities", "activities"),
    ("activity_logs", "activity_logs"),
    ("transactions", "transactions"),
    ("daily_reviews", "daily_reviews"),
    ("settings", "settings"),
    ("finance_accounts", "finance_accounts"),
    ("workout_history", "workout_history"),
    ("english_articles", "english_articles"),
    ("english_learning_records", "english_learning_records"),
    ("english_highlights", "english_highlights"),
    ("english_notes", "english_notes"),
    ("english_ai_analysis", "english_ai_analysis"),
    ("english_user_vocabulary", "english_vocabulary"),
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

fn has_column(connection: &Connection, table: &str, column: &str) -> bool {
    let mut statement = match connection.prepare(&format!("PRAGMA table_info({table})")) {
        Ok(statement) => statement,
        Err(_) => return false,
    };
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .map(|rows| {
            rows.flatten().any(|name| name == column)
        })
        .unwrap_or(false)
}

fn collect_sqlite_files(directory: &Path, depth: usize, result: &mut Vec<PathBuf>) {
    if depth > 7 {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sqlite_files(&path, depth + 1, result);
        } else if path.extension().and_then(|value| value.to_str()) == Some("sqlite")
            && path.file_name().and_then(|value| value.to_str()) != Some("metadata.sqlite")
            && entry.metadata().is_ok_and(|value| value.len() > 64 * 1024)
        {
            result.push(path);
        }
    }
}

fn candidates(data_dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(roaming) = data_dir.parent() {
        roots.push(roaming.join("LifeTrace").join("wrangler-state"));
        roots.push(roaming.join("lifetrace").join("wrangler-state"));
    }
    if let Ok(project) = std::env::current_dir() {
        roots.push(project.join(".wrangler").join("state"));
    }
    let mut files = Vec::new();
    for root in roots {
        collect_sqlite_files(&root, 0, &mut files);
    }
    files.sort_by_key(|path| fs::metadata(path).and_then(|value| value.modified()).ok());
    files.reverse();
    files
}

fn copy_json_table(
    source: &Connection,
    destination: &mut Connection,
    source_table: &str,
    destination_table: &str,
) -> Result<usize, String> {
    if !table_exists(source, source_table) {
        return Ok(0);
    }
    if !has_column(destination, destination_table, "data_json") {
        // 目标已是规范化真实列表：通过对应 Repository 导入。
        return match destination_table {
            "finance_accounts" | "transactions" => {
                crate::database::legacy::finance_d1::import_json_table(
                    source,
                    destination,
                    source_table,
                    destination_table,
                )
            }
            "activities" | "activity_logs" | "daily_reviews" => {
                crate::database::legacy::habits_d1::import_json_table(
                    source,
                    destination,
                    source_table,
                    destination_table,
                )
            }
            "english_articles"
            | "english_learning_records"
            | "english_highlights"
            | "english_notes"
            | "english_ai_analysis"
            | "english_vocabulary" => {
                crate::database::legacy::english_d1::import_json_table(
                    source,
                    destination,
                    source_table,
                    destination_table,
                )
            }
            _ => Ok(0),
        };
    }
    let mut statement = source
        .prepare(&format!(
            "SELECT id,data_json,updated_at FROM {source_table}"
        ))
        .map_err(|value| value.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|value| value.to_string())?;
    let mut copied = 0;
    for row in rows {
        let (id, data, updated_at) = row.map_err(|value| value.to_string())?;
        copied += destination
            .execute(
                &format!("INSERT OR IGNORE INTO {destination_table}(id,data_json,updated_at) VALUES(?1,?2,?3)"),
                params![id, data, updated_at],
            )
            .map_err(|value| value.to_string())?;
    }
    Ok(copied)
}

fn copy_json_query(
    source: &Connection,
    destination: &Connection,
    query: &str,
    destination_table: &str,
) -> Result<usize, String> {
    let mut statement = source.prepare(query).map_err(|value| value.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|value| value.to_string())?;
    let mut copied = 0;
    for row in rows {
        let (id, data, updated_at) = row.map_err(|value| value.to_string())?;
        copied += destination.execute(
            &format!("INSERT OR IGNORE INTO {destination_table}(id,data_json,updated_at) VALUES(?1,?2,?3)"),
            params![id, data, updated_at],
        ).map_err(|value| value.to_string())?;
    }
    Ok(copied)
}

fn migrate_notes(source: &Connection, destination: &mut Connection) -> Result<usize, String> {
    if !table_exists(source, "notes") {
        return Ok(0);
    }
    if has_column(destination, "notes", "content_json") {
        // 目标已是规范化真实列表：直接导入 D1 真实列数据。
        return crate::database::legacy::notes_d1::import_d1_notes(source, destination);
    }
    let mut copied = 0;
    copied += copy_json_query(
        source,
        destination,
        "SELECT id,json_object(
          'id',id,'title',title,'noteType',note_type,'folderId',folder_id,
          'contentJson',json(content_json),'contentHtml',content_html,'contentText',content_text,
          'contentMarkdown',content_markdown,'summary',summary,'isPinned',json(CASE is_pinned WHEN 1 THEN 'true' ELSE 'false' END),
          'isFavorite',json(CASE is_favorite WHEN 1 THEN 'true' ELSE 'false' END),
          'isArchived',json(CASE is_archived WHEN 1 THEN 'true' ELSE 'false' END),
          'createdAt',created_at,'updatedAt',updated_at,'deletedAt',deleted_at,'version',version,
          'tags',json(COALESCE((SELECT json_group_array(json_object('id',t.id,'name',t.name,'color',t.color,'createdAt',t.created_at,'updatedAt',t.updated_at)) FROM note_tags t JOIN note_tag_relations tr ON tr.tag_id=t.id WHERE tr.note_id=notes.id),'[]')),
          'relations',json(COALESCE((SELECT json_group_array(json_object('id',r.id,'noteId',r.note_id,'entityType',r.entity_type,'entityId',r.entity_id,'relationType',r.relation_type,'createdAt',r.created_at)) FROM note_relations r WHERE r.note_id=notes.id),'[]')),
          'attachments',json(COALESCE((SELECT json_group_array(json_object('id',a.id,'noteId',a.note_id,'fileName',a.file_name,'originalName',a.original_name,'mimeType',a.mime_type,'fileSize',a.file_size,'storagePath',a.storage_path,'createdAt',a.created_at)) FROM note_attachments a WHERE a.note_id=notes.id),'[]'))
        ),updated_at FROM notes",
        "notes_v2",
    )?;
    if table_exists(source, "note_folders") {
        copied += copy_json_query(
            source,
            destination,
            "SELECT id,json_object('id',id,'name',name,'icon',icon,'color',color,'sortOrder',sort_order,'createdAt',created_at,'updatedAt',updated_at),updated_at FROM note_folders",
            "note_folders_v2",
        )?;
    }
    if table_exists(source, "note_tags") {
        copied += copy_json_query(
            source,
            destination,
            "SELECT id,json_object('id',id,'name',name,'color',color,'createdAt',created_at,'updatedAt',updated_at),updated_at FROM note_tags",
            "note_tags_v2",
        )?;
    }
    if table_exists(source, "note_revisions") {
        copied += copy_json_query(
            source,
            destination,
            "SELECT id,json_object('id',id,'noteId',note_id,'version',version,'title',title,'contentJson',json(content_json),'contentHtml',content_html,'contentMarkdown',content_markdown,'createdAt',created_at),created_at FROM note_revisions",
            "note_revisions_v2",
        )?;
    }
    Ok(copied)
}

pub fn migrate_once(destination: &mut Connection, data_dir: &Path) -> Result<usize, String> {
    destination
        .execute(
            "CREATE TABLE IF NOT EXISTS app_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL)",
            [],
        )
        .map_err(|value| value.to_string())?;
    let checked: Option<String> = destination
        .query_row(
            "SELECT value FROM app_meta WHERE key='legacy_d1_migration'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|value| value.to_string())?;
    if checked.is_some() {
        return Ok(0);
    }
    let mut copied = 0;
    for path in candidates(data_dir) {
        let Ok(source) = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) else {
            continue;
        };
        if !JSON_TABLES
            .iter()
            .any(|(table, _)| table_exists(&source, table))
            && !table_exists(&source, "notes")
        {
            continue;
        }
        for (source_table, destination_table) in JSON_TABLES {
            if source_table == "finance_accounts" || source_table == "transactions" {
                continue;
            }
        copied += copy_json_table(&source, destination, source_table, destination_table)?;
        }
        // 财务：账户必须先于交易导入。
        copied += copy_json_table(&source, destination, "finance_accounts", "finance_accounts")?;
        copied += copy_json_table(&source, destination, "transactions", "transactions")?;
        copied += migrate_notes(&source, destination)?;
        if copied > 0 {
            break;
        }
    }
    destination
        .execute(
            "INSERT OR REPLACE INTO app_meta(key,value) VALUES('legacy_d1_migration',?1)",
            [format!("{}:{copied}", chrono::Utc::now().to_rfc3339())],
        )
        .map_err(|value| value.to_string())?;
    Ok(copied)
}
