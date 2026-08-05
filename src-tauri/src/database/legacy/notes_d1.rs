//! 旧 D1 笔记数据（真实列）直接导入规范化表。

use rusqlite::{params, Connection, OptionalExtension};

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

/// 从旧 D1 库导入笔记相关真实列表，返回导入条数。
pub fn import_d1_notes(source: &Connection, destination: &mut Connection) -> Result<usize, String> {
    let transaction = destination
        .transaction()
        .map_err(|error| error.to_string())?;
    let mut imported = 0usize;

    if table_exists(source, "note_folders") {
        let mut statement = source
            .prepare(
                "SELECT id, name, icon, color, sort_order, created_at, updated_at
                 FROM note_folders",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (id, name, icon, color, sort_order, created_at, updated_at) =
                row.map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO note_folders(
                       id, user_id, name, icon, color, sort_order, created_at, updated_at,
                       deleted_at, version, modified_by_device
                     ) VALUES(?1,'local',?2,?3,?4,?5,?6,?7,NULL,1,NULL)",
                    params![id, name, icon, color, sort_order, created_at, updated_at],
                )
                .map_err(|error| error.to_string())?;
            imported += 1;
        }
    }

    if table_exists(source, "note_tags") {
        let mut statement = source
            .prepare("SELECT id, name, color, created_at, updated_at FROM note_tags")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (id, name, color, created_at, updated_at) =
                row.map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO note_tags(
                       id, user_id, name, color, created_at, updated_at, deleted_at, version,
                       modified_by_device
                     ) VALUES(?1,'local',?2,?3,?4,?5,NULL,1,NULL)",
                    params![id, name, color, created_at, updated_at],
                )
                .map_err(|error| error.to_string())?;
            imported += 1;
        }
    }

    if table_exists(source, "notes") {
        let mut note_ids = std::collections::HashSet::new();
        let mut statement = source
            .prepare(
                "SELECT id, title, note_type, folder_id, content_json, content_html,
                        content_text, content_markdown, summary, is_pinned, is_favorite,
                        is_archived, created_at, updated_at, deleted_at, version
                 FROM notes",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, bool>(9)?,
                    row.get::<_, bool>(10)?,
                    row.get::<_, bool>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, i64>(15)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (
                id,
                title,
                note_type,
                folder_id,
                content_json,
                content_html,
                content_text,
                content_markdown,
                summary,
                is_pinned,
                is_favorite,
                is_archived,
                created_at,
                updated_at,
                deleted_at,
                version,
            ) = row.map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO notes(
                       id, user_id, title, note_type, folder_id, content_json, content_html,
                       content_text, content_markdown, summary, is_pinned, is_favorite,
                       is_archived, created_at, updated_at, deleted_at, version,
                       modified_by_device
                     ) VALUES(?1,'local',?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,NULL)",
                    params![
                        id,
                        title,
                        note_type,
                        folder_id,
                        content_json,
                        content_html,
                        content_text,
                        content_markdown,
                        summary,
                        is_pinned,
                        is_favorite,
                        is_archived,
                        created_at,
                        updated_at,
                        deleted_at,
                        version.max(1)
                    ],
                )
                .map_err(|error| error.to_string())?;
            note_ids.insert(id);
            imported += 1;
        }

        if table_exists(source, "note_tag_relations") {
            let mut statement = source
                .prepare("SELECT note_id, tag_id FROM note_tag_relations")
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| error.to_string())?;
            let stamp = chrono::Utc::now().to_rfc3339();
            for row in rows {
                let (note_id, tag_id) = row.map_err(|error| error.to_string())?;
                if note_ids.contains(&note_id) {
                    transaction
                        .execute(
                            "INSERT OR IGNORE INTO note_tag_relations(note_id, tag_id, created_at)
                             VALUES(?1,?2,?3)",
                            params![note_id, tag_id, stamp],
                        )
                        .map_err(|error| error.to_string())?;
                    imported += 1;
                }
            }
        }

        if table_exists(source, "note_relations") {
            let mut statement = source
                .prepare(
                    "SELECT id, note_id, entity_type, entity_id, relation_type, created_at
                     FROM note_relations",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            for row in rows {
                let (id, note_id, entity_type, entity_id, relation_type, created_at) =
                    row.map_err(|error| error.to_string())?;
                if note_ids.contains(&note_id) {
                    transaction
                        .execute(
                            "INSERT OR IGNORE INTO note_relations(
                               id, note_id, entity_type, entity_id, relation_type, created_at
                             ) VALUES(?1,?2,?3,?4,?5,?6)",
                            params![
                                id,
                                note_id,
                                entity_type,
                                entity_id,
                                relation_type,
                                created_at
                            ],
                        )
                        .map_err(|error| error.to_string())?;
                    imported += 1;
                }
            }
        }

        if table_exists(source, "note_attachments") {
            let mut statement = source
                .prepare(
                    "SELECT id, note_id, file_name, original_name, mime_type, file_size,
                            storage_path, created_at
                     FROM note_attachments",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            for row in rows {
                let (
                    id,
                    note_id,
                    file_name,
                    original_name,
                    mime_type,
                    file_size,
                    storage_path,
                    created_at,
                ) = row.map_err(|error| error.to_string())?;
                if note_ids.contains(&note_id) {
                    transaction
                        .execute(
                            "INSERT OR IGNORE INTO note_attachments(
                               id, note_id, file_name, original_name, mime_type, file_size,
                               storage_path, created_at
                             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                            params![
                                id,
                                note_id,
                                file_name,
                                original_name,
                                mime_type,
                                file_size,
                                storage_path,
                                created_at
                            ],
                        )
                        .map_err(|error| error.to_string())?;
                    imported += 1;
                }
            }
        }

        if table_exists(source, "note_revisions") {
            let mut statement = source
                .prepare(
                    "SELECT id, note_id, version, title, content_json, content_html,
                            content_markdown, created_at
                     FROM note_revisions",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            for row in rows {
                let (
                    id,
                    note_id,
                    version,
                    title,
                    content_json,
                    content_html,
                    content_markdown,
                    created_at,
                ) = row.map_err(|error| error.to_string())?;
                if note_ids.contains(&note_id) {
                    transaction
                        .execute(
                            "INSERT OR IGNORE INTO note_revisions(
                               id, note_id, revision_version, title, content_json, content_html,
                               content_markdown, created_at
                             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                            params![
                                id,
                                note_id,
                                version,
                                title,
                                content_json,
                                content_html,
                                content_markdown,
                                created_at
                            ],
                        )
                        .map_err(|error| error.to_string())?;
                    imported += 1;
                }
            }
        }
    }

    transaction.commit().map_err(|error| error.to_string())?;
    Ok(imported)
}
