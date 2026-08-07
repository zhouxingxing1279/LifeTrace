use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProfile {
    pub id: String,
    pub display_name: String,
    pub cloud_user_id: Option<String>,
    pub cloud_binding_state: String,
    pub active: bool,
}

const LEGACY_PROFILE_TABLES: &[&str] = &[
    "finance_accounts",
    "transaction_categories",
    "transactions",
    "activities",
    "activity_logs",
    "daily_reviews",
    "note_folders",
    "note_tags",
    "notes",
    "english_learning_records",
    "english_highlights",
    "english_notes",
    "english_vocabulary",
    "english_ai_analysis",
    "workouts",
    "workout_imports",
    "training_notes",
];

fn legacy_profile_id(connection: &Connection) -> Result<String, String> {
    for table in LEGACY_PROFILE_TABLES {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [*table],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !exists {
            continue;
        }
        let sql = format!(
            "SELECT user_id FROM {table} WHERE user_id IS NOT NULL AND trim(user_id)<>'' ORDER BY rowid LIMIT 1"
        );
        let profile_id = connection
            .query_row(&sql, [], |row| row.get::<_, String>(0))
            .optional()
            .map_err(|error| error.to_string())?;
        if let Some(profile_id) = profile_id {
            return Ok(profile_id);
        }
    }
    Ok("local".to_owned())
}

pub fn active_profile_id(connection: &Connection) -> Result<String, String> {
    let state_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='app_profile_state')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !state_exists {
        return legacy_profile_id(connection);
    }
    connection
        .query_row(
            "SELECT active_profile_id FROM app_profile_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

pub fn ensure_active_profile(connection: &Connection) -> Result<String, String> {
    if let Ok(id) = active_profile_id(connection) {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    let stamp = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO local_profiles(id,display_name,cloud_binding_state,created_at,updated_at)
         VALUES(?1,'本机资料','local_only',?2,?2)",
            params![id, stamp],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO app_profile_state(singleton,active_profile_id,updated_at) VALUES(1,?1,?2)",
            params![id, stamp],
        )
        .map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn list(connection: &Connection) -> Result<Vec<LocalProfile>, String> {
    let active = active_profile_id(connection)?;
    let mut statement = connection.prepare(
        "SELECT id,display_name,cloud_user_id,cloud_binding_state FROM local_profiles ORDER BY created_at"
    ).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(LocalProfile {
                id: row.get(0)?,
                display_name: row.get(1)?,
                cloud_user_id: row.get(2)?,
                cloud_binding_state: row.get(3)?,
                active: row.get::<_, String>(0)? == active,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn create(connection: &Connection, display_name: &str) -> Result<LocalProfile, String> {
    let id = Uuid::new_v4().to_string();
    let stamp = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO local_profiles(id,display_name,cloud_binding_state,created_at,updated_at)
         VALUES(?1,?2,'local_only',?3,?3)",
            params![id, display_name.trim(), stamp],
        )
        .map_err(|error| error.to_string())?;
    Ok(LocalProfile {
        id,
        display_name: display_name.trim().to_owned(),
        cloud_user_id: None,
        cloud_binding_state: "local_only".to_owned(),
        active: false,
    })
}

pub fn set_active(connection: &Connection, profile_id: &str) -> Result<(), String> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM local_profiles WHERE id=?1)",
            [profile_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err("本地资料不存在".to_owned());
    }
    connection
        .execute(
            "UPDATE app_profile_state SET active_profile_id=?1,updated_at=?2 WHERE singleton=1",
            params![profile_id, Utc::now().to_rfc3339()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn bind_cloud_user(
    connection: &Connection,
    profile_id: &str,
    cloud_user_id: &str,
) -> Result<(), String> {
    let other: Option<String> = connection
        .query_row(
            "SELECT id FROM local_profiles WHERE cloud_user_id=?1 AND id<>?2",
            params![cloud_user_id, profile_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if other.is_some() {
        return Err("该云账号已绑定到其他本地资料".to_owned());
    }
    connection.execute(
        "UPDATE local_profiles SET cloud_user_id=?1,cloud_binding_state='bound',updated_at=?2 WHERE id=?3",
        params![cloud_user_id, Utc::now().to_rfc3339(), profile_id]
    ).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn mark_pending_choice(connection: &Connection, profile_id: &str) -> Result<(), String> {
    connection.execute(
        "UPDATE local_profiles SET cloud_binding_state='pending_choice',updated_at=?1 WHERE id=?2",
        params![Utc::now().to_rfc3339(), profile_id]
    ).map_err(|error| error.to_string())?;
    Ok(())
}

/// Return a DTO owned by the active local profile. Client-supplied `userId`
/// is never trusted for local persistence or sync isolation.
pub fn assign_active_owner(
    connection: &Connection,
    value: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut owned = value.clone();
    let object = owned
        .as_object_mut()
        .ok_or_else(|| "业务数据必须是 JSON 对象".to_owned())?;
    object.insert(
        "userId".to_owned(),
        serde_json::Value::String(active_profile_id(connection)?),
    );
    Ok(owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_data_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-{label}-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn profiles_isolate_repository_reads_and_reject_duplicate_cloud_binding() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(
            &mut connection,
            &MigrationContext::new(unique_data_dir("profile-isolation")),
            &all(),
        )
        .unwrap();
        let first = active_profile_id(&connection).unwrap();
        let second = create(&connection, "第二资料").unwrap();
        connection.execute(
            "INSERT INTO finance_accounts(id,user_id,name,account_type,opening_balance_cents,is_archived,created_at,updated_at,version)
             VALUES('profile-a-account',?1,'A','cash',0,0,?3,?3,1),
                   ('profile-b-account',?2,'B','cash',0,0,?3,?3,1)",
            params![first, second.id, Utc::now().to_rfc3339()],
        ).unwrap();
        let first_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM finance_accounts WHERE user_id=?1",
                [&first],
                |row| row.get(0),
            )
            .unwrap();
        let second_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM finance_accounts WHERE user_id=?1",
                [&second.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((first_count, second_count), (1, 1));

        bind_cloud_user(&connection, &first, "cloud-user").unwrap();
        assert!(bind_cloud_user(&connection, &second.id, "cloud-user").is_err());
    }

    #[test]
    fn client_supplied_user_id_is_overwritten_by_active_profile() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(
            &mut connection,
            &MigrationContext::new(unique_data_dir("profile-owner")),
            &all(),
        )
        .unwrap();
        let active = active_profile_id(&connection).unwrap();
        let owned = assign_active_owner(
            &connection,
            &serde_json::json!({"id":"x","userId":"attacker"}),
        )
        .unwrap();
        assert_eq!(owned["userId"], active);
    }
}
