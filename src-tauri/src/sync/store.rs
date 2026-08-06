use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::registry::{EntityRef, EntityType};
use lifetrace_contracts::sync::v1::{
    ChangeOperation, PullResponseV1, SnapshotResponseV1, SyncChangeV1,
};
use lifetrace_contracts::{AtomicGroupId, ChangeId, ConflictId, Cursor, EntityId, ServerVersion};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use lifetrace_sync_client::{
    ApplyPageResult, ConflictResolution, FailureClass, LeasedChange, LocalProfileId,
    PersistedConflict, RetryPolicy, SyncError, SyncScope, SyncStatus, SyncStore,
};

use crate::database::repositories::{english, finance, habits, notes, workouts};

use super::outbox::{enqueue_upsert, MutationOrigin};
use super::payload::wire_to_legacy;

fn server_version_from_string(value: &str) -> Result<ServerVersion, SyncError> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(SqliteSyncStore::db_error)
}

#[derive(Clone)]
pub struct SqliteSyncStore {
    database: Arc<Mutex<Connection>>,
    device_id: Arc<Mutex<String>>,
}

impl SqliteSyncStore {
    pub fn new(database: Arc<Mutex<Connection>>, device_id: impl Into<String>) -> Self {
        Self {
            database,
            device_id: Arc::new(Mutex::new(device_id.into())),
        }
    }

    fn list_find(values: Vec<Value>, id: &str) -> Option<Value> {
        values
            .into_iter()
            .find(|value| value.get("id").and_then(Value::as_str) == Some(id))
    }

    fn load_local_entity(
        connection: &Connection,
        profile: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<Value>, SyncError> {
        let value = match entity_type {
            "finance.account" => Self::list_find(finance::list_accounts(connection).map_err(Self::db_error)?, entity_id),
            "finance.transaction" => Self::list_find(finance::list_transactions(connection).map_err(Self::db_error)?, entity_id),
            "habit.activity" => Self::list_find(habits::list_activities(connection).map_err(Self::db_error)?, entity_id),
            "habit.log" => Self::list_find(habits::list_activity_logs(connection).map_err(Self::db_error)?, entity_id),
            "review.daily" => Self::list_find(habits::list_daily_reviews(connection).map_err(Self::db_error)?, entity_id),
            "workout.workout" => workouts::get_workout(connection, entity_id).map_err(Self::db_error)?,
            "workout.import" => workouts::get_import(connection, entity_id).map_err(Self::db_error)?,
            "note.note" => notes::get_note(connection, entity_id).map_err(Self::db_error)?,
            "english.learning_record" => english::get(connection, "records", entity_id).map_err(Self::db_error)?,
            "english.highlight" => english::get(connection, "highlights", entity_id).map_err(Self::db_error)?,
            "english.note" => english::get(connection, "notes", entity_id).map_err(Self::db_error)?,
            "english.vocabulary" => english::get(connection, "vocabulary", entity_id).map_err(Self::db_error)?,
            "note.folder" => connection.query_row(
                "SELECT id,user_id,name,icon,color,sort_order,created_at,updated_at FROM note_folders WHERE id=?1 AND user_id=?2",
                params![entity_id,profile], |row| Ok(json!({
                    "id":row.get::<_,String>(0)?,"userId":row.get::<_,String>(1)?,"name":row.get::<_,String>(2)?,
                    "icon":row.get::<_,String>(3)?,"color":row.get::<_,String>(4)?,"sortOrder":row.get::<_,i64>(5)?,
                    "createdAt":row.get::<_,String>(6)?,"updatedAt":row.get::<_,String>(7)?
                }))
            ).optional().map_err(Self::db_error)?,
            "note.tag" => connection.query_row(
                "SELECT id,user_id,name,color,created_at,updated_at FROM note_tags WHERE id=?1 AND user_id=?2",
                params![entity_id,profile], |row| Ok(json!({
                    "id":row.get::<_,String>(0)?,"userId":row.get::<_,String>(1)?,"name":row.get::<_,String>(2)?,
                    "color":row.get::<_,String>(3)?,"createdAt":row.get::<_,String>(4)?,"updatedAt":row.get::<_,String>(5)?
                }))
            ).optional().map_err(Self::db_error)?,
            "workout.training_note" => connection.query_row(
                "SELECT id,user_id,title,content,workout_id,source,note_date,created_at,updated_at FROM training_notes WHERE id=?1 AND user_id=?2",
                params![entity_id,profile], |row| Ok(json!({
                    "id":row.get::<_,String>(0)?,"userId":row.get::<_,String>(1)?,"title":row.get::<_,String>(2)?,
                    "content":row.get::<_,String>(3)?,"workoutId":row.get::<_,Option<String>>(4)?,"source":row.get::<_,String>(5)?,
                    "noteDate":row.get::<_,String>(6)?,"createdAt":row.get::<_,String>(7)?,"updatedAt":row.get::<_,String>(8)?
                }))
            ).optional().map_err(Self::db_error)?,
            "workout.exercise" => connection.query_row(
                "SELECT e.id,w.user_id,e.workout_id,e.name,e.sort_order,e.planned_sets,e.completed_sets,w.created_at,w.updated_at
                 FROM workout_exercises e JOIN workouts w ON w.id=e.workout_id WHERE e.id=?1 AND w.user_id=?2",
                params![entity_id,profile], |row| Ok(json!({
                    "id":row.get::<_,String>(0)?,"userId":row.get::<_,String>(1)?,"workoutId":row.get::<_,String>(2)?,
                    "name":row.get::<_,String>(3)?,"sortOrder":row.get::<_,i64>(4)?,"plannedSets":row.get::<_,i64>(5)?,
                    "completedSets":row.get::<_,i64>(6)?,"createdAt":row.get::<_,String>(7)?,"updatedAt":row.get::<_,String>(8)?
                }))
            ).optional().map_err(Self::db_error)?,
            "workout.set" => connection.query_row(
                "SELECT s.id,w.user_id,s.exercise_id,s.set_number,s.weight_kg,s.reps,s.completed,w.created_at,w.updated_at
                 FROM workout_sets s JOIN workout_exercises e ON e.id=s.exercise_id JOIN workouts w ON w.id=e.workout_id
                 WHERE s.id=?1 AND w.user_id=?2",
                params![entity_id,profile], |row| Ok(json!({
                    "id":row.get::<_,String>(0)?,"userId":row.get::<_,String>(1)?,"exerciseId":row.get::<_,String>(2)?,
                    "setNumber":row.get::<_,i64>(3)?,"weightKg":row.get::<_,Option<f64>>(4)?,"reps":row.get::<_,Option<i64>>(5)?,
                    "completed":row.get::<_,bool>(6)?,"createdAt":row.get::<_,String>(7)?,"updatedAt":row.get::<_,String>(8)?
                }))
            ).optional().map_err(Self::db_error)?,
            "note.tag_relation" => {
                let (note_id, tag_id) = entity_id.split_once(':').unwrap_or(("", ""));
                connection.query_row(
                    "SELECT n.user_id,r.note_id,r.tag_id,n.created_at,n.updated_at FROM note_tag_relations r JOIN notes n ON n.id=r.note_id WHERE r.note_id=?1 AND r.tag_id=?2 AND n.user_id=?3",
                    params![note_id,tag_id,profile], |row| Ok(json!({
                        "id":entity_id,"userId":row.get::<_,String>(0)?,"noteId":row.get::<_,String>(1)?,"tagId":row.get::<_,String>(2)?,
                        "createdAt":row.get::<_,String>(3)?,"updatedAt":row.get::<_,String>(4)?
                    }))
                ).optional().map_err(Self::db_error)?
            }
            _ => connection.query_row(
                "SELECT payload_json FROM sync_materialized_entities WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3 AND deleted_at IS NULL",
                params![profile,entity_type,entity_id], |row| row.get::<_,String>(0),
            ).optional().map_err(Self::db_error)?.map(|raw| serde_json::from_str(&raw)).transpose().map_err(Self::db_error)?,
        };
        Ok(value)
    }

    fn db_error(error: impl std::fmt::Display) -> SyncError {
        SyncError::new(
            "SYNC_SQLITE",
            error.to_string(),
            lifetrace_sync_client::FailureClass::Permanent,
        )
    }

    fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, SyncError> {
        DateTime::parse_from_rfc3339(value)
            .map(|value| value.with_timezone(&Utc))
            .map_err(Self::db_error)
    }

    fn upsert_metadata(
        connection: &Connection,
        profile: &str,
        entity_type: &str,
        entity_id: &str,
        server_version: &str,
        cursor: &str,
    ) -> Result<(), SyncError> {
        connection.execute(
            "INSERT INTO sync_metadata(profile_id,entity_type,entity_id,server_version,last_server_cursor,last_synced_at)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(profile_id,entity_type,entity_id) DO UPDATE SET
               server_version=excluded.server_version,last_server_cursor=excluded.last_server_cursor,
               last_synced_at=excluded.last_synced_at",
            params![profile, entity_type, entity_id, server_version, cursor, Utc::now().to_rfc3339()],
        ).map_err(Self::db_error)?;
        Ok(())
    }

    fn active_change_exists(
        connection: &Connection,
        profile: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<bool, SyncError> {
        connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sync_outbox WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3
             AND status IN ('pending','leased','blocked','conflict'))",
            params![profile, entity_type, entity_id], |row| row.get(0),
        ).map_err(Self::db_error)
    }

    fn persist_pull_conflict(
        connection: &Connection,
        profile: &str,
        change: &lifetrace_contracts::sync::v1::ServerChangeV1,
    ) -> Result<(), SyncError> {
        let local_payload: Option<String> = connection.query_row(
            "SELECT payload_json FROM sync_outbox WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3
             AND status IN ('pending','leased','blocked','conflict') ORDER BY created_at DESC LIMIT 1",
            params![profile, change.entity_type.as_str(), change.entity_id.as_str()], |row| row.get(0),
        ).optional().map_err(Self::db_error)?.flatten();
        let conflict_id = format!(
            "pull-{}-{}-{}",
            change.cursor, change.entity_type, change.entity_id
        );
        connection.execute(
            "INSERT INTO sync_conflicts(
               conflict_id,profile_id,change_id,entity_type,entity_id,conflict_type,
               base_server_version,server_version,local_payload_json,remote_payload_json,
               server_deleted,status,created_at
             ) VALUES(?1,?2,NULL,?3,?4,'remote_changed_while_local_pending','0',?5,?6,?7,?8,'unresolved',?9)
             ON CONFLICT(conflict_id) DO NOTHING",
            params![
                conflict_id, profile, change.entity_type.as_str(), change.entity_id.as_str(),
                change.server_version.as_str(), local_payload,
                change.payload.as_ref().map(|value| value.0.to_string()),
                change.operation.as_str() == ChangeOperation::DELETE,
                Utc::now().to_rfc3339(),
            ],
        ).map_err(Self::db_error)?;
        connection.execute(
            "UPDATE sync_outbox SET status='conflict',updated_at=?1
             WHERE profile_id=?2 AND entity_type=?3 AND entity_id=?4 AND status IN ('pending','leased','blocked')",
            params![Utc::now().to_rfc3339(), profile, change.entity_type.as_str(), change.entity_id.as_str()],
        ).map_err(Self::db_error)?;
        Ok(())
    }

    fn apply_server_change(
        connection: &Connection,
        profile: &str,
        change: &lifetrace_contracts::sync::v1::ServerChangeV1,
    ) -> Result<(), SyncError> {
        if change.operation.as_str() == ChangeOperation::DELETE {
            Self::apply_delete(
                connection,
                profile,
                change.entity_type.as_str(),
                change.entity_id.as_str(),
            )?;
        } else if let Some(payload) = &change.payload {
            Self::apply_upsert(connection, profile, change.entity_type.as_str(), &payload.0)?;
        }
        Self::upsert_metadata(
            connection,
            profile,
            change.entity_type.as_str(),
            change.entity_id.as_str(),
            change.server_version.as_str(),
            change.cursor.as_str(),
        )?;
        Ok(())
    }

    fn force_owner(mut value: Value, profile: &str) -> Value {
        if let Some(object) = value.as_object_mut() {
            object.insert("userId".to_owned(), json!(profile));
            object.insert("updatedAt".to_owned(), json!(Utc::now().to_rfc3339()));
        }
        value
    }

    fn apply_upsert(
        connection: &Connection,
        profile: &str,
        entity_type: &str,
        payload: &Value,
    ) -> Result<(), SyncError> {
        let legacy = Self::force_owner(wire_to_legacy(payload).map_err(Self::db_error)?, profile);
        let result = match entity_type {
            "finance.account" => finance::save_account(connection, &legacy),
            "finance.transaction" => finance::save_transaction(connection, &legacy),
            "habit.activity" => habits::save_activity(connection, &legacy),
            "habit.log" => habits::save_activity_log(connection, &legacy),
            "review.daily" => habits::save_daily_review(connection, &legacy),
            "workout.workout" => workouts::save_workout(connection, &legacy),
            "workout.import" => workouts::save_import(connection, &legacy),
            "workout.training_note" => workouts::save_training_note(connection, &legacy),
            "note.note" => notes::save_note(connection, &legacy, true, false).map(|_| ()),
            "note.folder" => notes::save_folder(connection, &legacy).map(|_| ()),
            "note.tag" => notes::save_tag(connection, &legacy).map(|_| ()),
            "english.learning_record" => english::put(connection, "records", &legacy),
            "english.highlight" => english::put(connection, "highlights", &legacy),
            "english.note" => english::put(connection, "notes", &legacy),
            "english.vocabulary" => english::put(connection, "vocabulary", &legacy),
            _ => {
                let entity_id = payload
                    .get("meta")
                    .and_then(|v| v.get("id"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| Self::db_error("wire payload missing meta.id"))?;
                connection.execute(
                    "INSERT INTO sync_materialized_entities(profile_id,entity_type,entity_id,payload_json,deleted_at,updated_at)
                     VALUES(?1,?2,?3,?4,NULL,?5)
                     ON CONFLICT(profile_id,entity_type,entity_id) DO UPDATE SET
                       payload_json=excluded.payload_json,deleted_at=NULL,updated_at=excluded.updated_at",
                    params![profile, entity_type, entity_id, payload.to_string(), Utc::now().to_rfc3339()],
                ).map(|_| ()).map_err(|error| error.to_string())
            }
        };
        result.map_err(Self::db_error)
    }

    fn apply_delete(
        connection: &Connection,
        profile: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), SyncError> {
        let result = match entity_type {
            "finance.account" => finance::delete_account(connection, entity_id),
            "finance.transaction" => finance::delete_transaction(connection, entity_id),
            "workout.workout" => workouts::delete_workout(connection, entity_id),
            "note.note" => notes::set_deleted(connection, entity_id, true),
            "note.folder" => notes::delete_folder(connection, entity_id),
            "note.tag" => notes::delete_tag(connection, entity_id),
            "english.learning_record" => english::remove(connection, "records", entity_id).map(|_| ()),
            "english.highlight" => english::remove(connection, "highlights", entity_id).map(|_| ()),
            "english.note" => english::remove(connection, "notes", entity_id).map(|_| ()),
            "english.vocabulary" => english::remove(connection, "vocabulary", entity_id).map(|_| ()),
            "habit.activity" => connection.execute(
                "UPDATE activities SET deleted_at=?1,updated_at=?1,version=version+1 WHERE id=?2 AND user_id=?3",
                params![Utc::now().to_rfc3339(), entity_id, profile]
            ).map(|_| ()).map_err(|error| error.to_string()),
            "habit.log" => connection.execute(
                "UPDATE activity_logs SET deleted_at=?1,updated_at=?1,version=version+1 WHERE id=?2 AND user_id=?3",
                params![Utc::now().to_rfc3339(), entity_id, profile]
            ).map(|_| ()).map_err(|error| error.to_string()),
            "review.daily" => connection.execute(
                "UPDATE daily_reviews SET deleted_at=?1,updated_at=?1,version=version+1 WHERE id=?2 AND user_id=?3",
                params![Utc::now().to_rfc3339(), entity_id, profile]
            ).map(|_| ()).map_err(|error| error.to_string()),
            _ => connection.execute(
                "INSERT INTO sync_materialized_entities(profile_id,entity_type,entity_id,payload_json,deleted_at,updated_at)
                 VALUES(?1,?2,?3,NULL,?4,?4)
                 ON CONFLICT(profile_id,entity_type,entity_id) DO UPDATE SET payload_json=NULL,deleted_at=?4,updated_at=?4",
                params![profile, entity_type, entity_id, Utc::now().to_rfc3339()]
            ).map(|_| ()).map_err(|error| error.to_string()),
        };
        result.map_err(Self::db_error)
    }
}

#[async_trait]
impl SyncStore for SqliteSyncStore {
    async fn profile_is_cloud_bound(&self, profile: &LocalProfileId) -> Result<bool, SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        connection.query_row(
            "SELECT cloud_user_id IS NOT NULL AND cloud_binding_state='bound' FROM local_profiles WHERE id=?1",
            [profile.as_str()], |row| row.get(0),
        ).map_err(Self::db_error)
    }

    async fn lease_pending(
        &self,
        profile: &LocalProfileId,
        owner: &str,
        limit: usize,
        lease_seconds: u64,
    ) -> Result<Vec<LeasedChange>, SyncError> {
        let mut connection = self.database.lock().map_err(Self::db_error)?;
        let tx = connection.transaction().map_err(Self::db_error)?;
        let now = Utc::now();
        tx.execute(
            "UPDATE sync_outbox SET status='pending',lease_owner=NULL,lease_expires_at=NULL
             WHERE profile_id=?1 AND status='leased' AND lease_expires_at<?2",
            params![profile.as_str(), now.to_rfc3339()],
        )
        .map_err(Self::db_error)?;
        let mut statement = tx
            .prepare(
                "WITH first_batch AS (
               SELECT change_id,atomic_group_id FROM sync_outbox
                WHERE profile_id=?1 AND status='pending'
                  AND (next_attempt_at IS NULL OR next_attempt_at<=?2)
                ORDER BY created_at LIMIT ?3
             )
             SELECT change_id,entity_type,entity_id,operation,base_server_version,
                    entity_schema_version,payload_json,dependencies_json,atomic_group_id,updated_at
             FROM sync_outbox
             WHERE profile_id=?1 AND status='pending'
               AND (next_attempt_at IS NULL OR next_attempt_at<=?2)
               AND (
                 change_id IN (SELECT change_id FROM first_batch)
                 OR atomic_group_id IN (
                   SELECT atomic_group_id FROM first_batch WHERE atomic_group_id IS NOT NULL
                 )
               )
             ORDER BY created_at",
            )
            .map_err(Self::db_error)?;
        let rows = statement
            .query_map(
                params![profile.as_str(), now.to_rfc3339(), limit as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                },
            )
            .map_err(Self::db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Self::db_error)?;
        drop(statement);
        let lease_expires = (now + Duration::seconds(lease_seconds as i64)).to_rfc3339();
        let mut result = Vec::new();
        for (
            change_id,
            entity_type,
            entity_id,
            mut operation,
            base,
            schema,
            payload,
            dependencies,
            atomic,
            updated_at,
        ) in rows
        {
            tx.execute(
                "UPDATE sync_outbox SET status='leased',lease_owner=?1,lease_expires_at=?2,updated_at=?3 WHERE change_id=?4 AND status='pending'",
                params![owner, lease_expires, now.to_rfc3339(), change_id],
            ).map_err(Self::db_error)?;
            let payload_value = if operation == ChangeOperation::UPSERT && payload.is_none() {
                match Self::load_local_entity(&tx, profile.as_str(), &entity_type, &entity_id) {
                    Ok(Some(legacy)) => {
                        let wire = super::payload::legacy_to_wire(
                            &entity_type,
                            &legacy,
                            profile.as_str(),
                            Some(&base),
                        )
                        .map_err(Self::db_error)?;
                        tx.execute(
                            "UPDATE sync_outbox SET payload_json=?1,updated_at=?2 WHERE change_id=?3",
                            params![wire.to_string(), now.to_rfc3339(), change_id],
                        )
                        .map_err(Self::db_error)?;
                        Some(wire)
                    }
                    Ok(None) => {
                        // 本地实体已不存在（硬删除/档案切换等导致 outbox 过期）：
                        // 从未上传过则丢弃；已上传过则转为删除，保持云端一致，
                        // 避免一条过期变更永久阻塞 Push/Pull。
                        if base == "0" {
                            tx.execute(
                                "DELETE FROM sync_outbox WHERE change_id=?1",
                                [&change_id],
                            )
                            .map_err(Self::db_error)?;
                            continue;
                        }
                        tx.execute(
                            "UPDATE sync_outbox SET operation='delete', payload_json=NULL, updated_at=?1
                             WHERE change_id=?2",
                            params![now.to_rfc3339(), change_id],
                        )
                        .map_err(Self::db_error)?;
                        operation = ChangeOperation::DELETE.to_owned();
                        None
                    }
                    Err(error) => return Err(error),
                }
            } else {
                payload
                    .as_ref()
                    .map(|raw| serde_json::from_str::<Value>(raw))
                    .transpose()
                    .map_err(Self::db_error)?
            };
            let deps: Vec<EntityRef> =
                serde_json::from_str(&dependencies).map_err(Self::db_error)?;
            result.push(LeasedChange {
                change: SyncChangeV1 {
                    change_id: ChangeId::new(change_id),
                    entity_type: EntityType::new(entity_type),
                    entity_id: EntityId::new(entity_id),
                    operation: ChangeOperation::new(operation),
                    base_server_version: server_version_from_string(&base)?,
                    entity_schema_version: schema.max(1) as u32,
                    client_modified_at: Self::parse_timestamp(&updated_at)?,
                    payload: payload_value.clone().map(JsonValue),
                    atomic_group_id: atomic.map(AtomicGroupId::new),
                    dependencies: deps,
                },
                local_payload_json: payload_value,
            });
        }
        tx.commit().map_err(Self::db_error)?;
        Ok(result)
    }

    async fn release_lease(
        &self,
        change_ids: &[String],
        error: Option<&SyncError>,
    ) -> Result<(), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        let now = Utc::now();
        let policy = RetryPolicy::default();
        for id in change_ids {
            let retry_count: i64 = connection
                .query_row(
                    "SELECT retry_count FROM sync_outbox WHERE change_id=?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(Self::db_error)?
                .unwrap_or(0);
            let next_attempt = error.and_then(|value| {
                let delay = match value.class {
                    FailureClass::RateLimited {
                        retry_after_seconds,
                    } => std::time::Duration::from_secs(retry_after_seconds.unwrap_or(2)),
                    FailureClass::Offline | FailureClass::Transient => {
                        policy.delay((retry_count + 1).max(1) as u32, 0)
                    }
                    _ => return None,
                };
                Duration::from_std(delay)
                    .ok()
                    .map(|value| (now + value).to_rfc3339())
            });
            connection.execute(
                "UPDATE sync_outbox SET status='pending',retry_count=retry_count+1,next_attempt_at=?1,
                 lease_owner=NULL,lease_expires_at=NULL,last_error_code=?2,last_error_message=?3,updated_at=?4
                 WHERE change_id=?5",
                params![next_attempt, error.map(|e| e.code.as_str()), error.map(|e| e.message.as_str()), now.to_rfc3339(), id],
            ).map_err(Self::db_error)?;
        }
        Ok(())
    }

    async fn mark_confirmed(
        &self,
        change_id: &str,
        server_version: &str,
        cursor: &str,
    ) -> Result<(), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        let row: Option<(String, String, String)> = connection
            .query_row(
                "SELECT profile_id,entity_type,entity_id FROM sync_outbox WHERE change_id=?1",
                [change_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(Self::db_error)?;
        if let Some((profile, entity_type, entity_id)) = row {
            connection.execute(
                "UPDATE sync_outbox SET status='confirmed',lease_owner=NULL,lease_expires_at=NULL,updated_at=?1 WHERE change_id=?2",
                params![Utc::now().to_rfc3339(), change_id],
            ).map_err(Self::db_error)?;
            Self::upsert_metadata(
                &connection,
                &profile,
                &entity_type,
                &entity_id,
                server_version,
                cursor,
            )?;
        }
        Ok(())
    }

    async fn mark_blocked(
        &self,
        change_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        connection.execute(
            "UPDATE sync_outbox SET status='blocked',last_error_code=?1,last_error_message=?2,lease_owner=NULL,lease_expires_at=NULL,updated_at=?3 WHERE change_id=?4",
            params![code,message,Utc::now().to_rfc3339(),change_id],
        ).map_err(Self::db_error)?;
        Ok(())
    }

    async fn mark_dead_letter(
        &self,
        change_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        connection.execute(
            "UPDATE sync_outbox SET status='dead_letter',last_error_code=?1,last_error_message=?2,lease_owner=NULL,lease_expires_at=NULL,updated_at=?3 WHERE change_id=?4",
            params![code,message,Utc::now().to_rfc3339(),change_id],
        ).map_err(Self::db_error)?;
        Ok(())
    }

    async fn persist_conflict(&self, conflict: PersistedConflict) -> Result<(), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        let profile: String = connection
            .query_row(
                "SELECT profile_id FROM sync_outbox WHERE change_id=?1",
                [conflict
                    .change_id
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("")],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db_error)?
            .unwrap_or_default();
        connection.execute(
            "INSERT INTO sync_conflicts(conflict_id,profile_id,change_id,entity_type,entity_id,conflict_type,
             base_server_version,server_version,local_payload_json,remote_payload_json,server_deleted,status,created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'unresolved',?12)
             ON CONFLICT(conflict_id) DO UPDATE SET remote_payload_json=excluded.remote_payload_json,server_version=excluded.server_version",
            params![
                conflict.conflict_id.as_str(), profile, conflict.change_id.as_ref().map(|v| v.as_str()),
                conflict.entity_type.as_str(), conflict.entity_id.as_str(), conflict.kind,
                conflict.base_version.as_str(), conflict.server_version.as_str(),
                conflict.local_payload.map(|v| v.0.to_string()), conflict.remote_payload.map(|v| v.0.to_string()),
                conflict.server_deleted, Utc::now().to_rfc3339(),
            ],
        ).map_err(Self::db_error)?;
        if let Some(change_id) = conflict.change_id {
            connection
                .execute(
                    "UPDATE sync_outbox SET status='conflict' WHERE change_id=?1",
                    [change_id.as_str()],
                )
                .map_err(Self::db_error)?;
        }
        Ok(())
    }

    async fn cursor(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
    ) -> Result<Option<Cursor>, SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        let raw: Option<String> = connection
            .query_row(
                "SELECT cursor FROM sync_state WHERE profile_id=?1 AND scope_key=?2",
                params![profile.as_str(), scope.key],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db_error)?
            .flatten();
        Ok(raw.map(Cursor::new))
    }

    async fn apply_pull_page(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
        response: &PullResponseV1,
    ) -> Result<ApplyPageResult, SyncError> {
        let mut connection = self.database.lock().map_err(Self::db_error)?;
        let tx = connection.transaction().map_err(Self::db_error)?;
        tx.execute(
            "UPDATE sync_context SET origin='remote' WHERE singleton=1",
            [],
        )
        .map_err(Self::db_error)?;
        let device_id = self.device_id.lock().map_err(Self::db_error)?.clone();
        let mut result = ApplyPageResult::default();
        for change in &response.changes {
            let pending = Self::active_change_exists(
                &tx,
                profile.as_str(),
                change.entity_type.as_str(),
                change.entity_id.as_str(),
            )?;
            let same_device = change
                .origin_device_id
                .as_ref()
                .is_some_and(|value| value.as_str() == device_id);
            if pending && !same_device {
                Self::persist_pull_conflict(&tx, profile.as_str(), change)?;
                result.conflicts += 1;
                continue;
            }
            Self::apply_server_change(&tx, profile.as_str(), change)?;
            result.applied += 1;
            if pending && same_device {
                tx.execute(
                    "UPDATE sync_outbox SET status='confirmed',lease_owner=NULL,lease_expires_at=NULL,updated_at=?1
                     WHERE profile_id=?2 AND entity_type=?3 AND entity_id=?4 AND status IN ('pending','leased','blocked')",
                    params![Utc::now().to_rfc3339(),profile.as_str(),change.entity_type.as_str(),change.entity_id.as_str()],
                ).map_err(Self::db_error)?;
                result.confirmed_local += 1;
            }
        }
        tx.execute(
            "UPDATE sync_context SET origin='local' WHERE singleton=1",
            [],
        )
        .map_err(Self::db_error)?;
        tx.execute(
            "INSERT INTO sync_state(profile_id,scope_key,cursor,phase,updated_at) VALUES(?1,?2,?3,'pulling',?4)
             ON CONFLICT(profile_id,scope_key) DO UPDATE SET cursor=excluded.cursor,updated_at=excluded.updated_at",
            params![profile.as_str(),scope.key,response.next_cursor.as_str(),Utc::now().to_rfc3339()],
        ).map_err(Self::db_error)?;
        tx.commit().map_err(Self::db_error)?;
        Ok(result)
    }

    async fn begin_snapshot(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
    ) -> Result<(), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        connection
            .execute(
                "DELETE FROM sync_snapshot_staging WHERE profile_id=?1 AND scope_key=?2",
                params![profile.as_str(), scope.key],
            )
            .map_err(Self::db_error)?;
        connection.execute(
            "INSERT INTO sync_state(profile_id,scope_key,snapshot_in_progress,phase,updated_at)
             VALUES(?1,?2,1,'initializing_snapshot',?3)
             ON CONFLICT(profile_id,scope_key) DO UPDATE SET snapshot_in_progress=1,snapshot_id=NULL,
             snapshot_page_token=NULL,snapshot_cursor=NULL,phase='initializing_snapshot',updated_at=excluded.updated_at",
            params![profile.as_str(),scope.key,Utc::now().to_rfc3339()],
        ).map_err(Self::db_error)?;
        Ok(())
    }

    async fn stage_snapshot_page(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
        response: &SnapshotResponseV1,
    ) -> Result<(), SyncError> {
        let mut connection = self.database.lock().map_err(Self::db_error)?;
        let tx = connection.transaction().map_err(Self::db_error)?;
        for item in &response.items {
            tx.execute(
                "INSERT INTO sync_snapshot_staging(profile_id,scope_key,entity_type,entity_id,server_version,payload_json)
                 VALUES(?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(profile_id,scope_key,entity_type,entity_id) DO UPDATE SET
                   server_version=excluded.server_version,payload_json=excluded.payload_json",
                params![profile.as_str(),scope.key,item.entity_type.as_str(),item.entity_id.as_str(),item.server_version.as_str(),item.payload.0.to_string()],
            ).map_err(Self::db_error)?;
        }
        tx.execute(
            "UPDATE sync_state SET snapshot_id=?1,snapshot_page_token=?2,snapshot_cursor=?3,snapshot_in_progress=?4,updated_at=?5
             WHERE profile_id=?6 AND scope_key=?7",
            params![response.snapshot_id.as_str(),response.next_page_token,response.snapshot_cursor.as_str(),!response.completed,Utc::now().to_rfc3339(),profile.as_str(),scope.key],
        ).map_err(Self::db_error)?;
        tx.commit().map_err(Self::db_error)?;
        Ok(())
    }

    async fn finalize_snapshot(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
        snapshot_cursor: &Cursor,
    ) -> Result<(), SyncError> {
        let mut connection = self.database.lock().map_err(Self::db_error)?;
        let tx = connection.transaction().map_err(Self::db_error)?;
        tx.execute(
            "UPDATE sync_context SET origin='remote' WHERE singleton=1",
            [],
        )
        .map_err(Self::db_error)?;
        let mut statement = tx.prepare(
            "SELECT entity_type,entity_id,server_version,payload_json FROM sync_snapshot_staging
             WHERE profile_id=?1 AND scope_key=?2 ORDER BY entity_type,entity_id"
        ).map_err(Self::db_error)?;
        let rows = statement
            .query_map(params![profile.as_str(), scope.key], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(Self::db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Self::db_error)?;
        drop(statement);
        for (entity_type, entity_id, server_version, payload_raw) in rows {
            if Self::active_change_exists(&tx, profile.as_str(), &entity_type, &entity_id)? {
                let fake = lifetrace_contracts::sync::v1::ServerChangeV1 {
                    cursor: snapshot_cursor.clone(),
                    entity_type: EntityType::new(&entity_type),
                    entity_id: EntityId::new(&entity_id),
                    operation: ChangeOperation::new(ChangeOperation::UPSERT),
                    server_version: server_version_from_string(&server_version)?,
                    server_modified_at: Utc::now(),
                    payload: Some(JsonValue(
                        serde_json::from_str(&payload_raw).map_err(Self::db_error)?,
                    )),
                    tombstone: None,
                    origin_device_id: None,
                };
                Self::persist_pull_conflict(&tx, profile.as_str(), &fake)?;
            } else {
                let payload: Value = serde_json::from_str(&payload_raw).map_err(Self::db_error)?;
                Self::apply_upsert(&tx, profile.as_str(), &entity_type, &payload)?;
                Self::upsert_metadata(
                    &tx,
                    profile.as_str(),
                    &entity_type,
                    &entity_id,
                    &server_version,
                    snapshot_cursor.as_str(),
                )?;
            }
        }
        tx.execute(
            "UPDATE sync_context SET origin='local' WHERE singleton=1",
            [],
        )
        .map_err(Self::db_error)?;
        tx.execute(
            "DELETE FROM sync_snapshot_staging WHERE profile_id=?1 AND scope_key=?2",
            params![profile.as_str(), scope.key],
        )
        .map_err(Self::db_error)?;
        tx.execute(
            "UPDATE sync_state SET cursor=?1,snapshot_in_progress=0,snapshot_id=NULL,snapshot_page_token=NULL,
             snapshot_cursor=NULL,phase='idle',updated_at=?2 WHERE profile_id=?3 AND scope_key=?4",
            params![snapshot_cursor.as_str(),Utc::now().to_rfc3339(),profile.as_str(),scope.key],
        ).map_err(Self::db_error)?;
        tx.commit().map_err(Self::db_error)?;
        Ok(())
    }

    async fn snapshot_resume(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
    ) -> Result<(Option<String>, Option<String>), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        connection.query_row(
            "SELECT snapshot_id,snapshot_page_token FROM sync_state WHERE profile_id=?1 AND scope_key=?2 AND snapshot_in_progress=1",
            params![profile.as_str(),scope.key], |row| Ok((row.get(0)?,row.get(1)?)),
        ).optional().map_err(Self::db_error).map(|value| value.unwrap_or((None,None)))
    }

    async fn counts(&self, profile: &LocalProfileId) -> Result<(u64, u64), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        connection.query_row(
            "SELECT
             (SELECT COUNT(*) FROM sync_outbox WHERE profile_id=?1 AND status IN ('pending','leased','blocked')),
             (SELECT COUNT(*) FROM sync_conflicts WHERE profile_id=?1 AND status='unresolved')",
            [profile.as_str()], |row| Ok((row.get::<_,i64>(0)?.max(0) as u64,row.get::<_,i64>(1)?.max(0) as u64)),
        ).map_err(Self::db_error)
    }

    async fn set_status(
        &self,
        profile: &LocalProfileId,
        status: SyncStatus,
    ) -> Result<(), SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        connection.execute(
            "INSERT INTO sync_state(profile_id,scope_key,phase,pending_count,conflict_count,last_success_at,next_retry_at,last_error_code,last_error_message,updated_at)
             VALUES(?1,'all',?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(profile_id,scope_key) DO UPDATE SET phase=excluded.phase,pending_count=excluded.pending_count,
             conflict_count=excluded.conflict_count,last_success_at=COALESCE(excluded.last_success_at,sync_state.last_success_at),
             next_retry_at=excluded.next_retry_at,last_error_code=excluded.last_error_code,last_error_message=excluded.last_error_message,
             updated_at=excluded.updated_at",
            params![profile.as_str(), serde_json::to_value(&status.phase).map_err(Self::db_error)?.as_str().unwrap_or("error"),
                status.pending_count as i64,status.conflict_count as i64,status.last_success_at,status.next_retry_at,
                status.last_error_code,status.last_error_message,Utc::now().to_rfc3339()],
        ).map_err(Self::db_error)?;
        Ok(())
    }

    async fn list_conflicts(
        &self,
        profile: &LocalProfileId,
    ) -> Result<Vec<PersistedConflict>, SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        let mut statement = connection.prepare(
            "SELECT conflict_id,change_id,entity_type,entity_id,base_server_version,server_version,
             local_payload_json,remote_payload_json,server_deleted,conflict_type
             FROM sync_conflicts WHERE profile_id=?1 AND status='unresolved' ORDER BY created_at"
        ).map_err(Self::db_error)?;
        let rows = statement
            .query_map([profile.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .map_err(Self::db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Self::db_error)?;
        rows.into_iter()
            .map(|row| {
                Ok(PersistedConflict {
                    conflict_id: ConflictId::new(row.0),
                    change_id: row.1.map(ChangeId::new),
                    entity_type: EntityType::new(row.2),
                    entity_id: EntityId::new(row.3),
                    base_version: server_version_from_string(&row.4)?,
                    server_version: server_version_from_string(&row.5)?,
                    local_payload: row
                        .6
                        .map(|v| serde_json::from_str(&v))
                        .transpose()
                        .map_err(Self::db_error)?
                        .map(JsonValue),
                    remote_payload: row
                        .7
                        .map(|v| serde_json::from_str(&v))
                        .transpose()
                        .map_err(Self::db_error)?
                        .map(JsonValue),
                    server_deleted: row.8,
                    kind: row.9,
                })
            })
            .collect()
    }

    async fn resolve_conflict(
        &self,
        profile: &LocalProfileId,
        conflict_id: &str,
        resolution: ConflictResolution,
    ) -> Result<(), SyncError> {
        let mut connection = self.database.lock().map_err(Self::db_error)?;
        let tx = connection.transaction().map_err(Self::db_error)?;
        let row: (String,String,String,Option<String>,Option<String>,String,bool) = tx.query_row(
            "SELECT entity_type,entity_id,server_version,local_payload_json,remote_payload_json,conflict_type,server_deleted
             FROM sync_conflicts WHERE conflict_id=?1 AND profile_id=?2 AND status='unresolved'",
            params![conflict_id,profile.as_str()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)),
        ).map_err(Self::db_error)?;
        match resolution {
            ConflictResolution::AcceptRemote => {
                tx.execute(
                    "UPDATE sync_context SET origin='remote' WHERE singleton=1",
                    [],
                )
                .map_err(Self::db_error)?;
                if row.6 {
                    Self::apply_delete(&tx, profile.as_str(), &row.0, &row.1)?;
                } else if let Some(raw) = &row.4 {
                    Self::apply_upsert(
                        &tx,
                        profile.as_str(),
                        &row.0,
                        &serde_json::from_str(raw).map_err(Self::db_error)?,
                    )?;
                }
                tx.execute("DELETE FROM sync_outbox WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3 AND status='conflict'", params![profile.as_str(),row.0,row.1]).map_err(Self::db_error)?;
                tx.execute(
                    "UPDATE sync_context SET origin='local' WHERE singleton=1",
                    [],
                )
                .map_err(Self::db_error)?;
            }
            ConflictResolution::KeepLocal => {
                let raw = row
                    .3
                    .as_ref()
                    .ok_or_else(|| Self::db_error("conflict has no local payload"))?;
                let legacy = wire_to_legacy(&serde_json::from_str(raw).map_err(Self::db_error)?)
                    .map_err(Self::db_error)?;
                tx.execute("DELETE FROM sync_outbox WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3 AND status='conflict'", params![profile.as_str(),row.0,row.1]).map_err(Self::db_error)?;
                enqueue_upsert(&tx, &row.0, &legacy, None, MutationOrigin::Local)
                    .map_err(Self::db_error)?;
                tx.execute(
                    "UPDATE sync_outbox SET base_server_version=?1 WHERE profile_id=?2 AND entity_type=?3 AND entity_id=?4 AND status='pending'",
                    params![row.2,profile.as_str(),row.0,row.1]
                ).map_err(Self::db_error)?;
            }
            ConflictResolution::Discard => {
                tx.execute("DELETE FROM sync_outbox WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3 AND status='conflict'", params![profile.as_str(),row.0,row.1]).map_err(Self::db_error)?;
            }
        }
        tx.execute(
            "UPDATE sync_conflicts SET status=?1,resolution=?2,resolved_at=?3 WHERE conflict_id=?4",
            params![
                if resolution == ConflictResolution::Discard {
                    "discarded"
                } else {
                    "resolved"
                },
                format!("{resolution:?}"),
                Utc::now().to_rfc3339(),
                conflict_id
            ],
        )
        .map_err(Self::db_error)?;
        tx.commit().map_err(Self::db_error)?;
        Ok(())
    }

    async fn entity_has_pending_change(
        &self,
        profile: &LocalProfileId,
        entity_type: &EntityType,
        entity_id: &EntityId,
    ) -> Result<bool, SyncError> {
        let connection = self.database.lock().map_err(Self::db_error)?;
        Self::active_change_exists(
            &connection,
            profile.as_str(),
            entity_type.as_str(),
            entity_id.as_str(),
        )
    }
}
