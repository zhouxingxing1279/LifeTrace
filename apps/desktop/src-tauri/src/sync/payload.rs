use chrono::{DateTime, Utc};
use lifetrace_contracts::registry::{describe, EntityOwnership, SyncMode};
use serde_json::{json, Map, Value};

fn stamp(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}

fn cents(value: Option<&Value>) -> Option<i64> {
    value
        .and_then(Value::as_f64)
        .map(|amount| (amount * 100.0).round() as i64)
}

fn text(value: Option<&Value>, fallback: &str) -> String {
    value.and_then(Value::as_str).unwrap_or(fallback).to_owned()
}

fn optional_string(value: Option<&Value>) -> Value {
    value
        .and_then(Value::as_str)
        .map(|v| json!(v))
        .unwrap_or(Value::Null)
}

fn common_meta(
    object: &Map<String, Value>,
    profile_id: &str,
    server_version: Option<&str>,
) -> Value {
    json!({
        "id": text(object.get("id"), ""),
        "userId": profile_id,
        "createdAt": stamp(object.get("createdAt")),
        "updatedAt": stamp(object.get("updatedAt")),
        "deletedAt": object.get("deletedAt").cloned().unwrap_or(Value::Null),
        "localVersion": object.get("version").and_then(Value::as_u64).unwrap_or(1),
        "serverVersion": server_version,
        "modifiedByDevice": object.get("modifiedByDevice").cloned().unwrap_or(Value::Null),
    })
}

pub fn is_syncable(entity_type: &str) -> bool {
    describe(entity_type).is_some_and(|descriptor| {
        descriptor.ownership == EntityOwnership::UserOwned
            && matches!(
                descriptor.sync_mode,
                SyncMode::Bidirectional | SyncMode::ClientToServer
            )
    })
}

/// Convert the legacy/UI DTO to the published EPIC-02 entity contract. This
/// boundary also prevents credential/settings JSON from entering the outbox.
pub fn legacy_to_wire(
    entity_type: &str,
    value: &Value,
    profile_id: &str,
    server_version: Option<&str>,
) -> Result<Value, String> {
    if !is_syncable(entity_type) {
        return Err(format!("entity type is not client-syncable: {entity_type}"));
    }
    let object = value
        .as_object()
        .ok_or_else(|| "sync payload must be an object".to_owned())?;
    let meta = common_meta(object, profile_id, server_version);
    let payload = match entity_type {
        "finance.account" => json!({
            "meta": meta,
            "name": text(object.get("name"), "账户"),
            "accountType": text(object.get("type").or_else(|| object.get("accountType")), "cash"),
            "openingBalanceCents": object.get("openingBalanceCents").and_then(Value::as_i64)
                .or_else(|| cents(object.get("balance"))),
            "balanceAt": object.get("balanceAt").cloned().unwrap_or(Value::Null),
            "last4": optional_string(object.get("last4")),
            "color": text(object.get("color"), "#5f7d70"),
            "icon": text(object.get("icon"), ""),
            "isArchived": object.get("isArchived").and_then(Value::as_bool).unwrap_or(false),
            "currency": text(object.get("currency"), "CNY")
        }),
        "finance.category" => json!({
            "meta": meta,
            "name": text(object.get("name"), "未分类"),
            "categoryType": text(object.get("categoryType").or_else(|| object.get("type")), "expense"),
            "parentId": object.get("parentId").cloned().unwrap_or(Value::Null),
            "icon": object.get("icon").cloned().unwrap_or(Value::Null),
            "color": object.get("color").cloned().unwrap_or(Value::Null),
            "isSystem": object.get("isSystem").and_then(Value::as_bool).unwrap_or(false),
            "isArchived": object.get("isArchived").and_then(Value::as_bool).unwrap_or(false)
        }),
        "finance.transaction" => {
            let occurred = stamp(object.get("occurredAt").or_else(|| object.get("createdAt")));
            let local_date = object
                .get("localDate")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| occurred.get(0..10).unwrap_or("1970-01-01").to_owned());
            json!({
                "meta": meta,
                "transactionType": text(object.get("transactionType").or_else(|| object.get("type")), "expense"),
                "amountCents": object.get("amountCents").and_then(Value::as_i64)
                    .or_else(|| cents(object.get("amount"))).unwrap_or(0),
                "currency": text(object.get("currency"), "CNY"),
                "accountId": object.get("accountId").cloned().unwrap_or(Value::Null),
                "toAccountId": object.get("toAccountId").cloned().unwrap_or(Value::Null),
                "categoryId": object.get("categoryId").cloned().unwrap_or(Value::Null),
                "counterparty": object.get("counterparty").cloned().unwrap_or(Value::Null),
                "merchant": object.get("merchant").cloned().unwrap_or(Value::Null),
                "item": object.get("item").cloned().unwrap_or(Value::Null),
                "note": object.get("note").cloned().unwrap_or(Value::Null),
                "occurredAt": occurred,
                "localDate": local_date,
                "status": text(object.get("status"), "confirmed"),
                "sourceType": text(object.get("sourceType"), "manual"),
                "externalTransactionId": object.get("externalTransactionId").cloned().unwrap_or(Value::Null)
            })
        }
        "habit.activity" => json!({
            "meta": meta,
            "name": text(object.get("name"), "习惯"),
            "activityType": text(object.get("activityType").or_else(|| object.get("type")), "boolean"),
            "unit": text(object.get("unit"), ""),
            "minimumTarget": object.get("minimumTarget").cloned().unwrap_or(Value::Null),
            "normalTarget": object.get("normalTarget").cloned().unwrap_or(Value::Null),
            "targetPeriod": text(object.get("targetPeriod"), "daily"),
            "targetDays": object.get("targetDays").cloned().unwrap_or_else(|| json!([])),
            "icon": object.get("icon").cloned().unwrap_or(Value::Null),
            "color": object.get("color").cloned().unwrap_or(Value::Null),
            "scheduleType": object.get("scheduleType").cloned().unwrap_or(Value::Null),
            "startDate": object.get("startDate").cloned().unwrap_or(Value::Null),
            "checkinMethod": object.get("checkinMethod").cloned().unwrap_or(Value::Null),
            "syncSource": object.get("syncSource").cloned().unwrap_or(Value::Null),
            "description": object.get("description").cloned().unwrap_or(Value::Null),
            "isArchived": object.get("isArchived").and_then(Value::as_bool).unwrap_or(false)
        }),
        "habit.log" => {
            let created = stamp(object.get("createdAt").or_else(|| object.get("updatedAt")));
            json!({
                "meta": meta,
                "activityId": object.get("activityId").cloned().unwrap_or(Value::Null),
                "logDate": object.get("logDate").cloned().unwrap_or_else(|| json!(created.get(0..10).unwrap_or("1970-01-01"))),
                "value": object.get("value").cloned().unwrap_or(Value::Null),
                "status": object.get("status").cloned().unwrap_or(Value::Null),
                "note": object.get("note").cloned().unwrap_or(Value::Null),
                "metadata": object.get("metadata").cloned().unwrap_or(Value::Null)
            })
        }
        "review.daily" => json!({
            "meta": meta,
            "reviewDate": text(object.get("reviewDate"), Utc::now().date_naive().to_string().as_str()),
            "energy": object.get("energy").cloned().unwrap_or(Value::Null),
            "mood": object.get("mood").cloned().unwrap_or(Value::Null),
            "completionScore": object.get("completionScore").cloned().unwrap_or(Value::Null),
            "bestThing": object.get("bestThing").cloned().unwrap_or(Value::Null),
            "problem": object.get("problem").cloned().unwrap_or(Value::Null),
            "tomorrowPriority": object.get("tomorrowPriority").cloned().unwrap_or(Value::Null),
            "note": object.get("note").cloned().unwrap_or(Value::Null)
        }),
        "note.folder" => json!({
            "meta": meta, "name": text(object.get("name"), "文件夹"),
            "icon": text(object.get("icon"), ""), "color": text(object.get("color"), "#64748b"),
            "sortOrder": object.get("sortOrder").and_then(Value::as_i64).unwrap_or(0)
        }),
        "note.tag" => json!({
            "meta": meta, "name": text(object.get("name"), "标签"),
            "color": text(object.get("color"), "#64748b")
        }),
        "note.note" => json!({
            "meta": meta,
            "title": object.get("title").cloned().unwrap_or(Value::Null),
            "noteType": text(object.get("noteType"), "normal"),
            "folderId": object.get("folderId").cloned().unwrap_or(Value::Null),
            "contentJson": object.get("contentJson").cloned().unwrap_or_else(|| json!({"type":"doc","content":[]})),
            "contentHtml": text(object.get("contentHtml"), ""),
            "contentText": text(object.get("contentText"), ""),
            "contentMarkdown": text(object.get("contentMarkdown"), ""),
            "summary": text(object.get("summary"), ""),
            "isPinned": object.get("isPinned").and_then(Value::as_bool).unwrap_or(false),
            "isFavorite": object.get("isFavorite").and_then(Value::as_bool).unwrap_or(false),
            "isArchived": object.get("isArchived").and_then(Value::as_bool).unwrap_or(false),
            "aiSummary": object.get("aiSummary").cloned().unwrap_or(Value::Null),
            "aiTags": object.get("aiTags").cloned().unwrap_or(Value::Null),
            "embeddingStatus": object.get("embeddingStatus").cloned().unwrap_or(Value::Null),
            "lastAiProcessedAt": object.get("lastAiProcessedAt").cloned().unwrap_or(Value::Null)
        }),
        "workout.workout" => json!({
            "meta": meta,
            "source": text(object.get("source"), "manual"),
            "sourceId": object.get("sourceId").cloned().unwrap_or(Value::Null),
            "name": text(object.get("name"), "训练记录"),
            "occurredAt": stamp(object.get("occurredAt")),
            "localDate": object.get("localDate").cloned().unwrap_or_else(|| json!(stamp(object.get("occurredAt")).get(0..10).unwrap_or("1970-01-01"))),
            "durationSeconds": object.get("durationSeconds").and_then(Value::as_i64).unwrap_or(0),
            "exerciseCount": object.get("exerciseCount").and_then(Value::as_i64).unwrap_or(0),
            "setCount": object.get("setCount").and_then(Value::as_i64).unwrap_or(0),
            "plannedSetCount": object.get("plannedSetCount").cloned().unwrap_or(Value::Null),
            "volumeKg": object.get("volumeKg").cloned().unwrap_or(Value::Null),
            "caloriesKcal": object.get("caloriesKcal").cloned().unwrap_or(Value::Null),
            "status": object.get("status").cloned().unwrap_or(Value::Null)
        }),
        "workout.import" => json!({
            "meta": meta, "source": text(object.get("source"), "xunji"),
            "shareUrl": object.get("shareUrl").cloned().unwrap_or(Value::Null),
            "status": text(object.get("status"), "pending"),
            "parser": object.get("parser").cloned().unwrap_or(Value::Null),
            "parserVersion": object.get("parserVersion").cloned().unwrap_or(Value::Null),
            "error": object.get("error").cloned().unwrap_or(Value::Null),
            "workoutId": object.get("workoutId").or_else(|| object.get("workoutRecordId")).cloned().unwrap_or(Value::Null)
        }),
        "workout.training_note" => json!({
            "meta": meta, "title": text(object.get("title"), ""),
            "content": text(object.get("content"), ""),
            "workoutId": object.get("workoutId").or_else(|| object.get("workoutRecordId")).cloned().unwrap_or(Value::Null),
            "source": text(object.get("source"), "manual"),
            "noteDate": text(object.get("noteDate"), Utc::now().date_naive().to_string().as_str())
        }),
        // English and relation entities already use names close to the public
        // contract. Preserve fields while replacing ownership metadata.
        _ => {
            let mut copy = object.clone();
            copy.remove("id");
            copy.remove("userId");
            copy.remove("createdAt");
            copy.remove("updatedAt");
            copy.remove("deletedAt");
            copy.remove("version");
            copy.remove("modifiedByDevice");
            copy.insert("meta".to_owned(), meta);
            Value::Object(copy)
        }
    };
    Ok(payload)
}

/// Convert a wire contract payload back to the legacy repository DTO.
pub fn wire_to_legacy(payload: &Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "wire payload must be an object".to_owned())?;
    let mut legacy = object.clone();
    let meta = legacy
        .remove("meta")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    for (wire, local) in [
        ("id", "id"),
        ("userId", "userId"),
        ("createdAt", "createdAt"),
        ("updatedAt", "updatedAt"),
        ("deletedAt", "deletedAt"),
        ("localVersion", "version"),
        ("modifiedByDevice", "modifiedByDevice"),
    ] {
        if let Some(value) = meta.get(wire) {
            legacy.insert(local.to_owned(), value.clone());
        }
    }
    for (wire, local) in [
        ("accountType", "type"),
        ("transactionType", "type"),
        ("activityType", "type"),
        ("openingBalanceCents", "openingBalanceCents"),
    ] {
        if let Some(value) = legacy.remove(wire) {
            legacy.insert(local.to_owned(), value);
        }
    }
    if let Some(amount_cents) = legacy.get("amountCents").and_then(Value::as_i64) {
        legacy.insert("amount".to_owned(), json!(amount_cents as f64 / 100.0));
    }
    if let Some(opening) = legacy.get("openingBalanceCents").and_then(Value::as_i64) {
        legacy.insert("balance".to_owned(), json!(opening as f64 / 100.0));
    }
    Ok(Value::Object(legacy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifetrace_contracts::domain::payload::EntityPayload;
    use lifetrace_contracts::EntityType;

    #[test]
    fn finance_transaction_payload_matches_contract() {
        let legacy = json!({
            "id":"t1","userId":"local-user","type":"expense","amount":12.34,
            "occurredAt":"2026-08-05T10:00:00Z","createdAt":"2026-08-05T10:00:00Z",
            "updatedAt":"2026-08-05T10:00:00Z"
        });
        let wire =
            legacy_to_wire(EntityType::FINANCE_TRANSACTION, &legacy, "profile-1", None).unwrap();
        let parsed = EntityPayload::try_from((
            &EntityType::new(EntityType::FINANCE_TRANSACTION),
            wire.clone().into(),
        ));
        assert!(parsed.is_ok(), "{parsed:?}: {wire}");
        assert_eq!(wire["amountCents"], 1234);
    }
}
