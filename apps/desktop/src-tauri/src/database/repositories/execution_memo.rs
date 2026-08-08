use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoRecord {
    pub id: String,
    pub user_id: String,
    pub content: String,
    pub plain_text: String,
    pub is_pinned: bool,
    pub status: String,
    pub archived_at: Option<String>,
    pub context: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MemoWrite {
    pub id: Option<String>,
    pub user_id: String,
    pub content: String,
    pub plain_text: String,
    pub is_pinned: bool,
    pub status: String,
    pub archived_at: Option<String>,
    pub context: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MemoFilter {
    pub query: Option<String>,
    pub status: Option<String>,
    pub pinned: Option<bool>,
    pub tag: Option<String>,
}

const COLUMNS: &str = "id,user_id,content,plain_text,is_pinned,status,archived_at,context,version,created_at,updated_at";

fn now() -> String { Utc::now().to_rfc3339() }

fn base_from_row(row: &Row<'_>) -> rusqlite::Result<MemoRecord> {
    Ok(MemoRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        content: row.get(2)?,
        plain_text: row.get(3)?,
        is_pinned: row.get::<_, i64>(4)? != 0,
        status: row.get(5)?,
        archived_at: row.get(6)?,
        context: row.get(7)?,
        version: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        tags: Vec::new(),
    })
}

fn tags_for(connection: &Connection, user_id: &str, memo_id: &str) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(
            "SELECT t.name FROM execution_memo_tags t
             JOIN execution_memo_tag_relations r ON r.tag_id=t.id
             WHERE r.memo_id=?1 AND t.user_id=?2 AND t.deleted_at IS NULL
             ORDER BY t.normalized_name ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![memo_id, user_id], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|error| error.to_string())
}

fn hydrate(connection: &Connection, mut memo: MemoRecord) -> Result<MemoRecord, String> {
    memo.tags = tags_for(connection, &memo.user_id, &memo.id)?;
    Ok(memo)
}

pub fn get(connection: &Connection, user_id: &str, id: &str) -> Result<Option<MemoRecord>, String> {
    let sql = format!("SELECT {COLUMNS} FROM execution_memos WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL");
    let memo = connection
        .query_row(&sql, params![id, user_id], base_from_row)
        .optional()
        .map_err(|error| error.to_string())?;
    memo.map(|memo| hydrate(connection, memo)).transpose()
}

pub fn list(connection: &Connection, user_id: &str, filter: &MemoFilter) -> Result<Vec<MemoRecord>, String> {
    let sql = format!(
        "SELECT DISTINCT {prefix} FROM execution_memos m
         LEFT JOIN execution_memo_tag_relations r ON r.memo_id=m.id
         LEFT JOIN execution_memo_tags t ON t.id=r.tag_id AND t.deleted_at IS NULL
         WHERE m.user_id=?1 AND m.deleted_at IS NULL
           AND (?2 IS NULL OR m.status=?2)
           AND (?3 IS NULL OR m.is_pinned=?3)
           AND (?4 IS NULL OR lower(m.plain_text) LIKE ?4 OR lower(COALESCE(m.context,'')) LIKE ?4 OR lower(COALESCE(t.name,'')) LIKE ?4)
           AND (?5 IS NULL OR lower(COALESCE(t.normalized_name,''))=?5)
         ORDER BY m.is_pinned DESC,m.updated_at DESC",
        prefix = COLUMNS.split(',').map(|column| format!("m.{column}")).collect::<Vec<_>>().join(",")
    );
    let search = filter.query.as_ref().map(|value| format!("%{}%", value.to_lowercase()));
    let pinned = filter.pinned.map(|value| if value { 1_i64 } else { 0_i64 });
    let tag = filter.tag.as_ref().map(|value| value.to_lowercase());
    let mut statement = connection.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![user_id, filter.status.as_deref(), pinned, search, tag], base_from_row)
        .map_err(|error| error.to_string())?;
    let base = rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|error| error.to_string())?;
    base.into_iter().map(|memo| hydrate(connection, memo)).collect()
}

fn ensure_tag(connection: &Connection, user_id: &str, name: &str) -> Result<String, String> {
    let normalized = name.to_lowercase();
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM execution_memo_tags WHERE user_id=?1 AND normalized_name=?2 AND deleted_at IS NULL",
            params![user_id, normalized],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    let stamp = now();
    connection
        .execute(
            "INSERT INTO execution_memo_tags(id,user_id,name,normalized_name,created_at,updated_at,version)
             VALUES(?1,?2,?3,?4,?5,?5,1)",
            params![id, user_id, name, normalized, stamp],
        )
        .map_err(|error| error.to_string())?;
    Ok(id)
}

fn replace_tags(connection: &Connection, user_id: &str, memo_id: &str, tags: &[String]) -> Result<(), String> {
    connection
        .execute("DELETE FROM execution_memo_tag_relations WHERE memo_id=?1", [memo_id])
        .map_err(|error| error.to_string())?;
    for tag in tags {
        let tag_id = ensure_tag(connection, user_id, tag)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO execution_memo_tag_relations(memo_id,tag_id,created_at) VALUES(?1,?2,?3)",
                params![memo_id, tag_id, now()],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn save(connection: &Connection, input: &MemoWrite) -> Result<MemoRecord, String> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    if get(connection, &input.user_id, &id)?.is_some() {
        let changed = connection
            .execute(
                "UPDATE execution_memos SET content=?1,plain_text=?2,is_pinned=?3,status=?4,archived_at=?5,context=?6,
                 updated_at=?7,version=version+1 WHERE id=?8 AND user_id=?9 AND deleted_at IS NULL",
                params![input.content,input.plain_text,if input.is_pinned {1_i64}else{0_i64},input.status,input.archived_at,
                    input.context,stamp,id,input.user_id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 { return Err("备忘录更新失败".to_owned()); }
    } else {
        connection
            .execute(
                "INSERT INTO execution_memos(id,user_id,content,plain_text,is_pinned,status,archived_at,context,created_at,updated_at,version)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,1)",
                params![id,input.user_id,input.content,input.plain_text,if input.is_pinned {1_i64}else{0_i64},input.status,
                    input.archived_at,input.context,stamp],
            )
            .map_err(|error| error.to_string())?;
    }
    replace_tags(connection, &input.user_id, &id, &input.tags)?;
    get(connection, &input.user_id, &id)?.ok_or_else(|| "备忘录保存后读取失败".to_owned())
}

pub fn soft_delete(connection: &Connection, user_id: &str, id: &str) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_memos SET deleted_at=?1,updated_at=?1,version=version+1 WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp,id,user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn find_conversion_target(
    connection: &Connection,
    user_id: &str,
    memo_id: &str,
    target_type: &str,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT target_id FROM execution_entity_links
             WHERE user_id=?1 AND source_type='memo' AND source_id=?2 AND relation_type='converted_to'
               AND target_type=?3 AND deleted_at IS NULL ORDER BY created_at ASC LIMIT 1",
            params![user_id,memo_id,target_type],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn create_conversion_links(
    connection: &Connection,
    user_id: &str,
    memo_id: &str,
    target_type: &str,
    target_id: &str,
) -> Result<(), String> {
    let stamp = now();
    connection.execute(
        "INSERT OR IGNORE INTO execution_entity_links(id,user_id,source_type,source_id,relation_type,target_type,target_id,created_at,updated_at,version)
         VALUES(?1,?2,'memo',?3,'converted_to',?4,?5,?6,?6,1)",
        params![Uuid::new_v4().to_string(),user_id,memo_id,target_type,target_id,stamp],
    ).map_err(|error| error.to_string())?;
    connection.execute(
        "INSERT OR IGNORE INTO execution_entity_links(id,user_id,source_type,source_id,relation_type,target_type,target_id,created_at,updated_at,version)
         VALUES(?1,?2,?3,?4,'derived_from','memo',?5,?6,?6,1)",
        params![Uuid::new_v4().to_string(),user_id,target_type,target_id,memo_id,stamp],
    ).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn db() -> (Connection,String) {
        let unique=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("lifetrace-memo-repo-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut connection=Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection,&MigrationContext::new(dir),&all()).unwrap();
        let user_id=crate::database::profile::active_profile_id(&connection).unwrap();
        (connection,user_id)
    }

    #[test]
    fn tags_and_search_round_trip() {
        let (connection,user_id)=db();
        let memo=save(&connection,&MemoWrite{id:None,user_id:user_id.clone(),content:"Buy milk".to_owned(),plain_text:"Buy milk".to_owned(),
            is_pinned:true,status:"active".to_owned(),archived_at:None,context:Some("home".to_owned()),tags:vec!["Errand".to_owned()] }).unwrap();
        assert_eq!(memo.tags,vec!["Errand"]);
        let result=list(&connection,&user_id,&MemoFilter{query:Some("errand".to_owned()),status:Some("active".to_owned()),..Default::default()}).unwrap();
        assert_eq!(result.len(),1);
        assert_eq!(result[0].id,memo.id);
    }
}
