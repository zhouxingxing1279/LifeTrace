use std::time::Duration as StdDuration;

use chrono::{Datelike, Duration, NaiveDate, SecondsFormat, Utc, Weekday};
use lifetrace_cloud::{AppState, Config};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

const LEASE_NAME: &str = "execution-maintenance-v1";
const LEASE_SECONDS: i64 = 45;
const LOOP_SLEEP: StdDuration = StdDuration::from_secs(15);
const HORIZON_DAYS: i64 = 60;
const BATCH_LIMIT: i64 = 200;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    config.validate().map_err(|message| {
        eprintln!("[lifetrace-execution-worker] invalid configuration: {message}");
        message
    })?;
    let state = AppState::new(config);
    state.initialize().await?;
    if !state.database_enabled {
        return Err("execution worker requires DATABASE_URL".into());
    }

    let owner = Uuid::new_v4();
    println!("[lifetrace-execution-worker] started owner={owner}");
    loop {
        if acquire_lease(&state, owner).await? {
            let reminders = fire_due_reminders(&state).await?;
            let task_occurrences = materialize_task_occurrences(&state).await?;
            let calendar_occurrences = materialize_calendar_occurrences(&state).await?;
            if reminders + task_occurrences + calendar_occurrences > 0 {
                println!(
                    "[lifetrace-execution-worker] cycle reminders={reminders} task_occurrences={task_occurrences} calendar_occurrences={calendar_occurrences}"
                );
            }
        }
        tokio::time::sleep(LOOP_SLEEP).await;
    }
}

async fn acquire_lease(state: &AppState, owner: Uuid) -> Result<bool, sqlx::Error> {
    let acquired = sqlx::query_scalar::<_, bool>(
        r#"
        INSERT INTO execution_worker_leases(lease_name, owner_id, lease_until, heartbeat_at)
        VALUES($1, $2, now() + make_interval(secs => $3), now())
        ON CONFLICT(lease_name) DO UPDATE SET
          owner_id = EXCLUDED.owner_id,
          lease_until = EXCLUDED.lease_until,
          heartbeat_at = now(),
          acquired_at = CASE
            WHEN execution_worker_leases.owner_id = EXCLUDED.owner_id
              THEN execution_worker_leases.acquired_at
            ELSE now()
          END
        WHERE execution_worker_leases.lease_until <= now()
           OR execution_worker_leases.owner_id = EXCLUDED.owner_id
        RETURNING TRUE
        "#,
    )
    .bind(LEASE_NAME)
    .bind(owner)
    .bind(LEASE_SECONDS as f64)
    .fetch_optional(&state.pool)
    .await?;
    Ok(acquired.unwrap_or(false))
}

async fn fire_due_reminders(state: &AppState) -> Result<usize, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT user_id, entity_id, server_version, payload
        FROM sync_entities
        WHERE entity_type='execution.reminder'
          AND is_deleted=FALSE
          AND payload->>'status'='scheduled'
          AND COALESCE(NULLIF(payload->>'snoozedUntil',''), payload->>'triggerAt') IS NOT NULL
          AND COALESCE(NULLIF(payload->>'snoozedUntil',''), payload->>'triggerAt')::timestamptz <= now()
        ORDER BY COALESCE(NULLIF(payload->>'snoozedUntil',''), payload->>'triggerAt')::timestamptz
        LIMIT $1
        "#,
    )
    .bind(BATCH_LIMIT)
    .fetch_all(&state.pool)
    .await?;

    let mut count = 0;
    for row in rows {
        let user_id: Uuid = row.get("user_id");
        let entity_id: String = row.get("entity_id");
        let version: i64 = row.get("server_version");
        let mut payload: Value = row.get("payload");
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        payload["status"] = json!("fired");
        payload["lastFiredAt"] = json!(now);
        payload["snoozedUntil"] = Value::Null;
        touch_meta(&mut payload, &now, version + 1);
        if publish_existing(&state.pool, user_id, "execution.reminder", &entity_id, version, payload)
            .await?
        {
            count += 1;
        }
    }
    Ok(count)
}

async fn materialize_task_occurrences(state: &AppState) -> Result<usize, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT rule.user_id, rule.entity_id AS rule_id, rule.payload AS rule_payload,
               task.entity_id AS subject_id, task.payload AS subject_payload
        FROM sync_entities rule
        JOIN sync_entities task
          ON task.user_id=rule.user_id
         AND task.entity_type='execution.task'
         AND task.is_deleted=FALSE
         AND task.payload->>'recurrenceRuleId'=rule.entity_id
        WHERE rule.entity_type='execution.recurrence_rule'
          AND rule.is_deleted=FALSE
        ORDER BY rule.user_id, rule.entity_id
        LIMIT $1
        "#,
    )
    .bind(BATCH_LIMIT)
    .fetch_all(&state.pool)
    .await?;

    let mut created = 0;
    for row in rows {
        let user_id: Uuid = row.get("user_id");
        let task_id: String = row.get("subject_id");
        let rule: Value = row.get("rule_payload");
        let task: Value = row.get("subject_payload");
        let Some(anchor) = task_anchor(&task) else { continue };
        for date in occurrence_dates(anchor, &rule, HORIZON_DAYS) {
            if occurrence_exists(&state.pool, user_id, "execution.task_occurrence", "taskId", &task_id, date).await? {
                continue;
            }
            if max_occurrences_reached(&state.pool, user_id, "execution.task_occurrence", "taskId", &task_id, &rule).await? {
                break;
            }
            let payload = task_occurrence_payload(user_id, &task_id, &task, anchor, date);
            let entity_id = deterministic_id(user_id, "task_occurrence", &task_id, date);
            if publish_new(&state.pool, user_id, "execution.task_occurrence", &entity_id, payload).await? {
                created += 1;
            }
        }
    }
    Ok(created)
}

async fn materialize_calendar_occurrences(state: &AppState) -> Result<usize, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT rule.user_id, rule.payload AS rule_payload,
               event.entity_id AS subject_id, event.payload AS subject_payload
        FROM sync_entities rule
        JOIN sync_entities event
          ON event.user_id=rule.user_id
         AND event.entity_type='execution.calendar_event'
         AND event.is_deleted=FALSE
         AND event.payload->>'recurrenceRuleId'=rule.entity_id
        WHERE rule.entity_type='execution.recurrence_rule'
          AND rule.is_deleted=FALSE
        ORDER BY rule.user_id, rule.entity_id
        LIMIT $1
        "#,
    )
    .bind(BATCH_LIMIT)
    .fetch_all(&state.pool)
    .await?;

    let mut created = 0;
    for row in rows {
        let user_id: Uuid = row.get("user_id");
        let event_id: String = row.get("subject_id");
        let rule: Value = row.get("rule_payload");
        let event: Value = row.get("subject_payload");
        let Some(anchor) = event_anchor(&event) else { continue };
        for date in occurrence_dates(anchor, &rule, HORIZON_DAYS) {
            if occurrence_exists(&state.pool, user_id, "execution.calendar_occurrence", "eventId", &event_id, date).await? {
                continue;
            }
            if max_occurrences_reached(&state.pool, user_id, "execution.calendar_occurrence", "eventId", &event_id, &rule).await? {
                break;
            }
            let payload = calendar_occurrence_payload(user_id, &event_id, &event, anchor, date);
            let entity_id = deterministic_id(user_id, "calendar_occurrence", &event_id, date);
            if publish_new(&state.pool, user_id, "execution.calendar_occurrence", &entity_id, payload).await? {
                created += 1;
            }
        }
    }
    Ok(created)
}

fn occurrence_dates(anchor: NaiveDate, rule: &Value, horizon_days: i64) -> Vec<NaiveDate> {
    let today = Utc::now().date_naive();
    let until = json_date(rule.get("untilAt"));
    let interval = rule.get("intervalValue").and_then(Value::as_i64).unwrap_or(1).max(1);
    let frequency = rule.get("frequency").and_then(Value::as_str).unwrap_or_default();
    let weekdays: Vec<u32> = rule
        .get("weekdays")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_u64).map(|value| value as u32).collect())
        .unwrap_or_default();
    let month_day = rule.get("monthDay").and_then(Value::as_u64).unwrap_or(anchor.day() as u64) as u32;
    (0..horizon_days)
        .map(|offset| today + Duration::days(offset))
        .filter(|candidate| *candidate >= anchor)
        .filter(|candidate| until.is_none_or(|limit| *candidate <= limit))
        .filter(|candidate| match frequency {
            "daily" => candidate.signed_duration_since(anchor).num_days() % interval == 0,
            "weekly" => {
                let weekday = weekday_number(candidate.weekday());
                let anchor_week = anchor - Duration::days((weekday_number(anchor.weekday()) - 1) as i64);
                let candidate_week = *candidate - Duration::days((weekday - 1) as i64);
                weekdays.contains(&weekday)
                    && candidate_week.signed_duration_since(anchor_week).num_days() / 7 % interval == 0
            }
            "monthly" => {
                let months = (candidate.year() - anchor.year()) as i64 * 12
                    + candidate.month() as i64
                    - anchor.month() as i64;
                months >= 0 && months % interval == 0 && candidate.day() == month_day
            }
            _ => false,
        })
        .collect()
}

fn weekday_number(day: Weekday) -> u32 {
    day.num_days_from_monday() + 1
}

fn json_date(value: Option<&Value>) -> Option<NaiveDate> {
    let text = value?.as_str()?;
    NaiveDate::parse_from_str(text.get(..10)?, "%Y-%m-%d").ok()
}

fn task_anchor(task: &Value) -> Option<NaiveDate> {
    json_date(task.get("scheduledStartAt"))
        .or_else(|| json_date(task.get("dueAt")))
        .or_else(|| json_date(task.get("meta").and_then(|meta| meta.get("createdAt"))))
}

fn event_anchor(event: &Value) -> Option<NaiveDate> {
    json_date(event.get("startLocalDate"))
        .or_else(|| json_date(event.get("startAt")))
        .or_else(|| json_date(event.get("meta").and_then(|meta| meta.get("createdAt"))))
}

fn task_occurrence_payload(user_id: Uuid, task_id: &str, task: &Value, anchor: NaiveDate, date: NaiveDate) -> Value {
    let delta = date.signed_duration_since(anchor).num_days();
    let scheduled_start = shifted_timestamp(task.get("scheduledStartAt"), delta);
    let scheduled_end = shifted_timestamp(task.get("scheduledEndAt"), delta);
    let due_at = shifted_timestamp(task.get("dueAt"), delta)
        .or_else(|| (!scheduled_start.is_some()).then(|| format!("{date}T23:59:00Z")));
    json!({
        "meta": server_meta(user_id, deterministic_id(user_id, "task_occurrence", task_id, date)),
        "taskId": task_id,
        "occurrenceKey": date.to_string(),
        "scheduledStartAt": scheduled_start,
        "scheduledEndAt": scheduled_end,
        "dueAt": due_at,
        "status": "pending",
        "titleOverride": null,
        "descriptionOverride": null,
        "completedAt": null,
        "skippedAt": null
    })
}

fn calendar_occurrence_payload(user_id: Uuid, event_id: &str, event: &Value, anchor: NaiveDate, date: NaiveDate) -> Value {
    let is_all_day = event.get("isAllDay").and_then(Value::as_bool).unwrap_or(false);
    let delta = date.signed_duration_since(anchor).num_days();
    let (start_at, end_at, start_local_date, end_local_date) = if is_all_day {
        let span = json_date(event.get("endLocalDate"))
            .map(|end| end.signed_duration_since(anchor).num_days().max(0))
            .unwrap_or(0);
        (None, None, Some(date.to_string()), Some((date + Duration::days(span)).to_string()))
    } else {
        (
            shifted_timestamp(event.get("startAt"), delta),
            shifted_timestamp(event.get("endAt"), delta),
            None,
            None,
        )
    };
    json!({
        "meta": server_meta(user_id, deterministic_id(user_id, "calendar_occurrence", event_id, date)),
        "eventId": event_id,
        "occurrenceKey": date.to_string(),
        "isAllDay": is_all_day,
        "startAt": start_at,
        "endAt": end_at,
        "startLocalDate": start_local_date,
        "endLocalDate": end_local_date,
        "status": "scheduled",
        "titleOverride": null,
        "descriptionOverride": null
    })
}

fn shifted_timestamp(value: Option<&Value>, days: i64) -> Option<String> {
    let raw = value?.as_str()?;
    let parsed = chrono::DateTime::parse_from_rfc3339(raw).ok()?;
    Some((parsed + Duration::days(days)).to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn server_meta(user_id: Uuid, entity_id: String) -> Value {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    json!({
        "id": entity_id,
        "userId": user_id.to_string(),
        "createdAt": now,
        "updatedAt": now,
        "deletedAt": null,
        "localVersion": 1,
        "serverVersion": null,
        "modifiedByDevice": null
    })
}

fn touch_meta(payload: &mut Value, now: &str, server_version: i64) {
    let Some(meta) = payload.get_mut("meta").and_then(Value::as_object_mut) else { return };
    meta.insert("updatedAt".to_owned(), json!(now));
    meta.insert("serverVersion".to_owned(), json!(server_version.to_string()));
    let next_local = meta.get("localVersion").and_then(Value::as_u64).unwrap_or(0) + 1;
    meta.insert("localVersion".to_owned(), json!(next_local));
    meta.insert("modifiedByDevice".to_owned(), Value::Null);
}

fn deterministic_id(user_id: Uuid, kind: &str, subject_id: &str, date: NaiveDate) -> String {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("lifetrace:{user_id}:{kind}:{subject_id}:{date}").as_bytes(),
    )
    .to_string()
}

async fn occurrence_exists(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    entity_type: &str,
    subject_field: &str,
    subject_id: &str,
    date: NaiveDate,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sync_entities WHERE user_id=$1 AND entity_type=$2 AND is_deleted=FALSE AND payload->>$3=$4 AND payload->>'occurrenceKey'=$5)",
    )
    .bind(user_id)
    .bind(entity_type)
    .bind(subject_field)
    .bind(subject_id)
    .bind(date.to_string())
    .fetch_one(pool)
    .await
}

async fn max_occurrences_reached(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    entity_type: &str,
    subject_field: &str,
    subject_id: &str,
    rule: &Value,
) -> Result<bool, sqlx::Error> {
    let Some(max) = rule.get("maxOccurrences").and_then(Value::as_i64) else { return Ok(false) };
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sync_entities WHERE user_id=$1 AND entity_type=$2 AND is_deleted=FALSE AND payload->>$3=$4",
    )
    .bind(user_id)
    .bind(entity_type)
    .bind(subject_field)
    .bind(subject_id)
    .fetch_one(pool)
    .await?;
    Ok(count >= max.max(1))
}

async fn publish_new(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    entity_type: &str,
    entity_id: &str,
    payload: Value,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sync_entities WHERE user_id=$1 AND entity_type=$2 AND entity_id=$3 AND is_deleted=FALSE)",
    )
    .bind(user_id)
    .bind(entity_type)
    .bind(entity_id)
    .fetch_one(&mut *tx)
    .await?;
    if exists {
        tx.rollback().await?;
        return Ok(false);
    }
    publish_entity(&mut tx, user_id, entity_type, entity_id, 1, payload, true).await?;
    tx.commit().await?;
    Ok(true)
}

async fn publish_existing(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    entity_type: &str,
    entity_id: &str,
    expected_version: i64,
    payload: Value,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let locked = sqlx::query_scalar::<_, i64>(
        "SELECT server_version FROM sync_entities WHERE user_id=$1 AND entity_type=$2 AND entity_id=$3 AND is_deleted=FALSE FOR UPDATE",
    )
    .bind(user_id)
    .bind(entity_type)
    .bind(entity_id)
    .fetch_optional(&mut *tx)
    .await?;
    if locked != Some(expected_version) {
        tx.rollback().await?;
        return Ok(false);
    }
    publish_entity(&mut tx, user_id, entity_type, entity_id, expected_version + 1, payload, false).await?;
    tx.commit().await?;
    Ok(true)
}

async fn publish_entity(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    entity_type: &str,
    entity_id: &str,
    server_version: i64,
    payload: Value,
    insert: bool,
) -> Result<(), sqlx::Error> {
    let bytes = serde_json::to_vec(&payload).expect("JSON payload must serialize");
    let payload_hash = Sha256::digest(bytes).to_vec();
    let now = Utc::now();
    let cursor = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sync_change_log(user_id,entity_type,entity_id,operation,entity_schema_version,server_version,payload,payload_hash,server_modified_at) VALUES($1,$2,$3,'upsert',1,$4,$5,$6,$7) RETURNING cursor",
    )
    .bind(user_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(server_version)
    .bind(payload.clone())
    .bind(payload_hash.clone())
    .bind(now)
    .fetch_one(&mut **tx)
    .await?;

    if insert {
        sqlx::query(
            "INSERT INTO sync_entities(user_id,entity_type,entity_id,entity_schema_version,server_version,payload,payload_hash,is_deleted,deleted_at,origin_device_id,created_at,server_modified_at,client_modified_at,last_cursor) VALUES($1,$2,$3,1,$4,$5,$6,FALSE,NULL,NULL,$7,$7,NULL,$8) ON CONFLICT(user_id,entity_type,entity_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(server_version)
        .bind(payload)
        .bind(payload_hash)
        .bind(now)
        .bind(cursor)
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE sync_entities SET server_version=$4,payload=$5,payload_hash=$6,server_modified_at=$7,client_modified_at=NULL,last_cursor=$8 WHERE user_id=$1 AND entity_type=$2 AND entity_id=$3",
        )
        .bind(user_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(server_version)
        .bind(payload)
        .bind(payload_hash)
        .bind(now)
        .bind(cursor)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurrence_matching_is_deterministic() {
        let anchor = NaiveDate::from_ymd_opt(2026, 8, 3).unwrap(); // Monday
        let weekly = json!({"frequency":"weekly","intervalValue":1,"weekdays":[1,3],"untilAt":null});
        let dates = occurrence_dates(anchor, &weekly, 14);
        assert!(dates.iter().all(|date| matches!(date.weekday(), Weekday::Mon | Weekday::Wed)));
        assert_eq!(dates.len(), 4);
    }

    #[test]
    fn occurrence_ids_are_stable() {
        let user = Uuid::nil();
        let day = NaiveDate::from_ymd_opt(2026, 8, 14).unwrap();
        assert_eq!(
            deterministic_id(user, "task_occurrence", "task-1", day),
            deterministic_id(user, "task_occurrence", "task-1", day)
        );
    }
}
