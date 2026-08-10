//! Obsidian-style wiki-link index for notes.
//!
//! `note_links` is derived data. The note body remains the source of truth and this index can be
//! rebuilt at any time from `content_markdown`, which keeps sync/backup semantics simple.

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedWikiLink {
    pub target_title: String,
    pub alias: Option<String>,
}

fn now() -> String {
    Utc::now().to_rfc3339()
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

/// Parse `[[Note title]]` and `[[Note title|Alias]]` from Markdown/plain text.
///
/// Repeated links with the same title and alias are folded into one graph edge. This keeps the
/// index relationship-oriented rather than occurrence-oriented.
pub fn parse_wiki_links(input: &str) -> Vec<ParsedWikiLink> {
    let mut result = Vec::new();
    let mut seen = HashSet::<(String, String)>::new();
    let mut cursor = 0usize;

    while cursor < input.len() {
        let Some(relative_start) = input[cursor..].find("[[") else {
            break;
        };
        let start = cursor + relative_start + 2;
        let Some(relative_end) = input[start..].find("]]" ) else {
            break;
        };
        let end = start + relative_end;
        cursor = end + 2;

        let raw = input[start..end].trim();
        if raw.is_empty() || raw.contains('\n') || raw.contains('\r') {
            continue;
        }
        let (target, alias) = match raw.split_once('|') {
            Some((target, alias)) => (target.trim(), Some(alias.trim())),
            None => (raw, None),
        };
        if target.is_empty() {
            continue;
        }
        let alias = alias.filter(|value| !value.is_empty()).map(str::to_owned);
        let key = (
            target.to_lowercase(),
            alias.as_deref().unwrap_or_default().to_lowercase(),
        );
        if seen.insert(key) {
            result.push(ParsedWikiLink {
                target_title: target.to_owned(),
                alias,
            });
        }
    }

    result
}

fn resolve_target_id(
    connection: &Connection,
    user_id: &str,
    source_note_id: &str,
    target_title: &str,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT id
             FROM notes
             WHERE user_id=?1 AND deleted_at IS NULL AND id<>?2
               AND trim(title)=?3 COLLATE NOCASE
             ORDER BY updated_at DESC, id
             LIMIT 1",
            params![user_id, source_note_id, target_title],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn target_is_available(
    connection: &Connection,
    user_id: &str,
    target_note_id: &str,
) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT 1 FROM notes WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL",
            params![target_note_id, user_id],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(|error| error.to_string())
}

fn sync_note_fields(
    connection: &Connection,
    note_id: &str,
    user_id: &str,
    title: Option<&str>,
    markdown: &str,
) -> Result<usize, String> {
    if !table_exists(connection, "note_links") {
        return Ok(0);
    }

    // Preserve already-resolved targets across target-note renames. The source text may still
    // contain the old title until the user edits it, but the graph edge remains stable by id.
    let mut previous_targets = HashMap::<String, String>::new();
    {
        let mut statement = connection
            .prepare(
                "SELECT target_title, target_note_id
                 FROM note_links
                 WHERE source_note_id=?1 AND target_note_id IS NOT NULL",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([note_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (target_title, target_id) = row.map_err(|error| error.to_string())?;
            previous_targets.insert(target_title.to_lowercase(), target_id);
        }
    }

    connection
        .execute("DELETE FROM note_links WHERE source_note_id=?1", [note_id])
        .map_err(|error| error.to_string())?;

    let stamp = now();
    let parsed = parse_wiki_links(markdown);
    for link in &parsed {
        let mut target_note_id = resolve_target_id(
            connection,
            user_id,
            note_id,
            &link.target_title,
        )?;
        if target_note_id.is_none() {
            if let Some(previous_id) = previous_targets.get(&link.target_title.to_lowercase()) {
                if target_is_available(connection, user_id, previous_id)? {
                    target_note_id = Some(previous_id.clone());
                }
            }
        }
        connection
            .execute(
                "INSERT INTO note_links(
                   id, source_note_id, target_note_id, target_title, alias, created_at, updated_at
                 ) VALUES(?1,?2,?3,?4,?5,?6,?6)",
                params![
                    Uuid::new_v4().to_string(),
                    note_id,
                    target_note_id,
                    link.target_title,
                    link.alias,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }

    // A newly created/renamed note may satisfy links that were previously unresolved.
    if let Some(title) = title.map(str::trim).filter(|value| !value.is_empty()) {
        connection
            .execute(
                "UPDATE note_links
                 SET target_note_id=?1, updated_at=?2
                 WHERE target_note_id IS NULL
                   AND trim(target_title)=?3 COLLATE NOCASE
                   AND source_note_id<>?1
                   AND source_note_id IN (
                     SELECT id FROM notes WHERE user_id=?4 AND deleted_at IS NULL
                   )",
                params![note_id, stamp, title, user_id],
            )
            .map_err(|error| error.to_string())?;
    }

    Ok(parsed.len())
}

/// Re-index one saved note from its Markdown body.
pub fn sync_note_links(connection: &Connection, note: &Value) -> Result<usize, String> {
    let note_id = note
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "笔记缺少 id，无法建立双链索引".to_owned())?;
    let user_id = crate::database::profile::active_profile_id(connection)?;
    let title = note.get("title").and_then(Value::as_str);
    let markdown = note
        .get("contentMarkdown")
        .and_then(Value::as_str)
        .or_else(|| note.get("contentText").and_then(Value::as_str))
        .unwrap_or_default();
    sync_note_fields(connection, note_id, &user_id, title, markdown)
}

/// Rebuild the complete derived index. Used by migrations and backup restore.
pub fn rebuild_all(connection: &Connection) -> Result<usize, String> {
    if !table_exists(connection, "note_links") {
        return Ok(0);
    }
    let notes = {
        let mut statement = connection
            .prepare(
                "SELECT id, user_id, title, content_markdown, content_text
                 FROM notes WHERE deleted_at IS NULL",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };

    connection
        .execute("DELETE FROM note_links", [])
        .map_err(|error| error.to_string())?;
    let mut total = 0usize;
    for (note_id, user_id, title, markdown, text) in notes {
        let source = if markdown.trim().is_empty() { &text } else { &markdown };
        total += sync_note_fields(
            connection,
            &note_id,
            &user_id,
            title.as_deref(),
            source,
        )?;
    }
    Ok(total)
}

/// Attach outgoing wiki links and backlinks to a full note DTO.
pub fn enrich_note(connection: &Connection, mut note: Value) -> Result<Value, String> {
    let note_id = note
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "笔记缺少 id，无法读取双链".to_owned())?
        .to_owned();
    if !table_exists(connection, "note_links") {
        if let Some(object) = note.as_object_mut() {
            object.insert("wikiLinks".to_owned(), Value::Array(Vec::new()));
            object.insert("backlinks".to_owned(), Value::Array(Vec::new()));
        }
        return Ok(note);
    }

    let outgoing = {
        let mut statement = connection
            .prepare(
                "SELECT l.id, n.id, l.target_title,
                        COALESCE(NULLIF(trim(n.title),''), l.target_title) AS display_title,
                        l.alias, l.created_at
                 FROM note_links l
                 LEFT JOIN notes n ON n.id=l.target_note_id AND n.deleted_at IS NULL
                 WHERE l.source_note_id=?1
                 ORDER BY display_title COLLATE NOCASE",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([&note_id], |row| {
                let target_note_id = row.get::<_, Option<String>>(1)?;
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "targetNoteId": target_note_id,
                    "targetTitle": row.get::<_, String>(2)?,
                    "displayTitle": row.get::<_, String>(3)?,
                    "alias": row.get::<_, Option<String>>(4)?,
                    "resolved": target_note_id.is_some(),
                    "createdAt": row.get::<_, String>(5)?
                }))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };

    let backlinks = {
        let mut statement = connection
            .prepare(
                "SELECT l.id, l.source_note_id,
                        COALESCE(NULLIF(trim(s.title),''), NULLIF(trim(s.summary),''), '无标题笔记'),
                        s.summary, l.alias, l.created_at
                 FROM note_links l
                 JOIN notes s ON s.id=l.source_note_id
                 WHERE l.target_note_id=?1 AND s.deleted_at IS NULL
                 ORDER BY s.updated_at DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([&note_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "sourceNoteId": row.get::<_, String>(1)?,
                    "sourceTitle": row.get::<_, String>(2)?,
                    "sourceSummary": row.get::<_, String>(3)?,
                    "alias": row.get::<_, Option<String>>(4)?,
                    "createdAt": row.get::<_, String>(5)?
                }))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };

    if let Some(object) = note.as_object_mut() {
        object.insert("wikiLinks".to_owned(), Value::Array(outgoing));
        object.insert("backlinks".to_owned(), Value::Array(backlinks));
    }
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_and_aliased_links() {
        assert_eq!(
            parse_wiki_links("See [[Tube MPC]] and [[Set Membership Filter|SMF]]."),
            vec![
                ParsedWikiLink {
                    target_title: "Tube MPC".to_owned(),
                    alias: None,
                },
                ParsedWikiLink {
                    target_title: "Set Membership Filter".to_owned(),
                    alias: Some("SMF".to_owned()),
                },
            ]
        );
    }

    #[test]
    fn folds_duplicate_edges_and_ignores_invalid_links() {
        assert_eq!(
            parse_wiki_links("[[MPC]] [[mpc]] [[]] [[broken"),
            vec![ParsedWikiLink {
                target_title: "MPC".to_owned(),
                alias: None,
            }]
        );
    }
}
