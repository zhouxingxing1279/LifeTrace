//! Pure BeeCount-to-LifeTrace protocol boundary helpers.
//!
//! Keeping these rules free of HTTP/database concerns makes the migration
//! importer and live sync facade use exactly the same ID, amount and LWW
//! semantics.

use chrono::{DateTime, Duration, Utc};
use lifetrace_contracts::domain::payload::EntityPayload;
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::{EntityType, UserId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Number, Value};

pub const ENTITY_ID_PREFIX: &str = "beecount:";
pub const NATIVE_WIRE_ID_PREFIX: &str = "lifetrace:";
pub const MAX_CLIENT_CLOCK_AHEAD_SECONDS: i64 = 5;
pub const USER_GLOBAL_LEDGER_SENTINEL: &str = "__user_global__";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeeCountScope {
    User,
    Ledger,
}

impl BeeCountScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Ledger => "ledger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeeCountEntityKind {
    Ledger,
    Account,
    Category,
    Transaction,
    Tag,
    Budget,
    ExchangeRateOverride,
}

impl BeeCountEntityKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ledger" => Some(Self::Ledger),
            "account" => Some(Self::Account),
            "category" => Some(Self::Category),
            "transaction" => Some(Self::Transaction),
            "tag" => Some(Self::Tag),
            "budget" => Some(Self::Budget),
            "exchange_rate_override" => Some(Self::ExchangeRateOverride),
            _ => None,
        }
    }

    pub fn lifetrace_entity_type(self) -> &'static str {
        match self {
            Self::Ledger => "finance.ledger",
            Self::Account => "finance.account",
            Self::Category => "finance.category",
            Self::Transaction => "finance.transaction",
            Self::Tag => "finance.tag",
            Self::Budget => "finance.budget",
            Self::ExchangeRateOverride => "user.preference",
        }
    }

    pub fn scope(self) -> BeeCountScope {
        match self {
            Self::Account | Self::Category | Self::Tag | Self::ExchangeRateOverride => {
                BeeCountScope::User
            }
            Self::Ledger | Self::Transaction | Self::Budget => BeeCountScope::Ledger,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BeeCountBoundaryError {
    #[error("amount is not a valid decimal number")]
    InvalidAmount,
    #[error("amount exceeds the signed 64-bit cents range")]
    AmountOutOfRange,
    #[error("unsupported BeeCount entity type: {0}")]
    UnsupportedEntityType(String),
    #[error("invalid BeeCount payload: {0}")]
    InvalidPayload(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct BeeCountSyncChangeIn {
    pub ledger_id: Option<String>,
    pub entity_type: String,
    pub entity_sync_id: String,
    pub action: String,
    #[serde(default)]
    pub payload: Value,
    pub updated_at: DateTime<Utc>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BeeCountSyncPushRequest {
    pub device_id: String,
    pub changes: Vec<BeeCountSyncChangeIn>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BeeCountConflictSample {
    pub reason: String,
    #[serde(rename = "ledgerId")]
    pub ledger_id: Option<String>,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(rename = "entitySyncId")]
    pub entity_sync_id: String,
    #[serde(rename = "existingChangeId")]
    pub existing_change_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BeeCountSyncPushResponse {
    pub accepted: usize,
    pub rejected: usize,
    pub conflict_count: usize,
    pub conflict_samples: Vec<BeeCountConflictSample>,
    pub server_cursor: i64,
    pub server_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BeeCountSyncChangeOut {
    pub change_id: i64,
    pub ledger_id: String,
    pub entity_type: String,
    pub entity_sync_id: String,
    pub action: String,
    pub payload: Value,
    pub updated_at: DateTime<Utc>,
    pub updated_by_device_id: Option<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BeeCountSyncPullResponse {
    pub changes: Vec<BeeCountSyncChangeOut>,
    pub server_cursor: i64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BeeCountSyncLedgerOut {
    pub ledger_id: String,
    pub path: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub size: i64,
    pub metadata: Value,
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BeeCountSyncFullResponse {
    pub ledger_id: String,
    pub snapshot: Option<BeeCountSyncChangeOut>,
    pub latest_cursor: i64,
}

/// Namespace a BeeCount sync id without double-prefixing values that have
/// already passed through the compatibility boundary.
pub fn lifetrace_entity_id(beecount_sync_id: &str) -> String {
    if let Some(native_id) = beecount_sync_id.strip_prefix(NATIVE_WIRE_ID_PREFIX) {
        native_id.to_owned()
    } else if beecount_sync_id.starts_with(ENTITY_ID_PREFIX) {
        beecount_sync_id.to_owned()
    } else {
        format!("{ENTITY_ID_PREFIX}{beecount_sync_id}")
    }
}

pub fn beecount_sync_id(lifetrace_entity_id: &str) -> &str {
    lifetrace_entity_id
        .strip_prefix(ENTITY_ID_PREFIX)
        .unwrap_or(lifetrace_entity_id)
}

/// Encode a canonical LifeTrace id for BeeCount without colliding with a
/// legacy BeeCount sync id. Imported ids lose their storage-only `beecount:`
/// prefix; native ids gain a reversible wire-only `lifetrace:` prefix.
pub fn beecount_wire_id(lifetrace_entity_id: &str) -> String {
    match lifetrace_entity_id.strip_prefix(ENTITY_ID_PREFIX) {
        Some(legacy_id) => legacy_id.to_owned(),
        None => format!("{NATIVE_WIRE_ID_PREFIX}{lifetrace_entity_id}"),
    }
}

/// Convert a JSON decimal representation into cents using round-half-away-
/// from-zero. This avoids binary floating point differences between the live
/// facade and the SQLite importer and supports exponent notation.
pub fn decimal_amount_to_cents(value: &str) -> Result<i64, BeeCountBoundaryError> {
    let raw = value.trim();
    if raw.is_empty() {
        return Err(BeeCountBoundaryError::InvalidAmount);
    }
    let (negative, unsigned) = match raw.as_bytes()[0] {
        b'-' => (true, &raw[1..]),
        b'+' => (false, &raw[1..]),
        _ => (false, raw),
    };
    if unsigned.is_empty() {
        return Err(BeeCountBoundaryError::InvalidAmount);
    }

    let mut exponent_parts = unsigned.split(['e', 'E']);
    let mantissa = exponent_parts
        .next()
        .ok_or(BeeCountBoundaryError::InvalidAmount)?;
    let exponent = match exponent_parts.next() {
        Some(value) => value
            .parse::<i32>()
            .map_err(|_| BeeCountBoundaryError::InvalidAmount)?,
        None => 0,
    };
    if exponent_parts.next().is_some() || exponent.unsigned_abs() > 1000 {
        return Err(BeeCountBoundaryError::InvalidAmount);
    }

    let mut decimal_parts = mantissa.split('.');
    let whole = decimal_parts
        .next()
        .ok_or(BeeCountBoundaryError::InvalidAmount)?;
    let fraction = decimal_parts.next().unwrap_or("");
    if decimal_parts.next().is_some()
        || (whole.is_empty() && fraction.is_empty())
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(BeeCountBoundaryError::InvalidAmount);
    }

    let digits = format!("{whole}{fraction}");
    let magnitude = digits
        .parse::<i128>()
        .map_err(|_| BeeCountBoundaryError::AmountOutOfRange)?;
    let decimal_places = i32::try_from(fraction.len())
        .map_err(|_| BeeCountBoundaryError::AmountOutOfRange)?
        - exponent;
    let cents_power = 2_i32 - decimal_places;

    let cents_magnitude = if cents_power >= 0 {
        let factor = checked_pow10(cents_power as u32)?;
        magnitude
            .checked_mul(factor)
            .ok_or(BeeCountBoundaryError::AmountOutOfRange)?
    } else {
        let divisor = checked_pow10(cents_power.unsigned_abs())?;
        let quotient = magnitude / divisor;
        let remainder = magnitude % divisor;
        if remainder
            .checked_mul(2)
            .ok_or(BeeCountBoundaryError::AmountOutOfRange)?
            >= divisor
        {
            quotient
                .checked_add(1)
                .ok_or(BeeCountBoundaryError::AmountOutOfRange)?
        } else {
            quotient
        }
    };

    let signed = if negative {
        cents_magnitude
            .checked_neg()
            .ok_or(BeeCountBoundaryError::AmountOutOfRange)?
    } else {
        cents_magnitude
    };
    i64::try_from(signed).map_err(|_| BeeCountBoundaryError::AmountOutOfRange)
}

fn checked_pow10(power: u32) -> Result<i128, BeeCountBoundaryError> {
    if power > 38 {
        return Err(BeeCountBoundaryError::AmountOutOfRange);
    }
    10_i128
        .checked_pow(power)
        .ok_or(BeeCountBoundaryError::AmountOutOfRange)
}

pub fn clamp_client_updated_at(
    client_updated_at: DateTime<Utc>,
    server_now: DateTime<Utc>,
) -> DateTime<Utc> {
    client_updated_at.min(server_now + Duration::seconds(MAX_CLIENT_CLOCK_AHEAD_SECONDS))
}

/// BeeCount's deterministic LWW tie-break. Equal timestamp and equal device
/// is not a new winner, making replay idempotent before the LifeTrace change-id
/// check is reached.
pub fn incoming_clock_wins(
    incoming_updated_at: DateTime<Utc>,
    incoming_device_id: &str,
    current_updated_at: DateTime<Utc>,
    current_device_id: &str,
) -> bool {
    incoming_updated_at > current_updated_at
        || (incoming_updated_at == current_updated_at && incoming_device_id > current_device_id)
}

pub fn canonical_payload(
    kind: BeeCountEntityKind,
    sync_id: &str,
    ledger_id: Option<&str>,
    raw_payload: &Value,
    user_id: &UserId,
    device_id: &str,
    updated_at: DateTime<Utc>,
    current_payload: Option<&Value>,
    next_local_version: u64,
) -> Result<JsonValue, BeeCountBoundaryError> {
    let raw = raw_payload.as_object().ok_or_else(|| {
        BeeCountBoundaryError::InvalidPayload("payload must be a JSON object".to_owned())
    })?;
    let entity_id = lifetrace_entity_id(sync_id);
    let created_at = current_payload
        .and_then(|value| value.pointer("/meta/createdAt"))
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or(updated_at);
    let meta = json!({
        "id": entity_id,
        "userId": user_id.as_str(),
        "createdAt": created_at,
        "updatedAt": updated_at,
        "deletedAt": null,
        "localVersion": next_local_version,
        "serverVersion": null,
        "modifiedByDevice": device_id,
    });
    let ledger_entity_id = lifetrace_entity_id(ledger_id.unwrap_or(USER_GLOBAL_LEDGER_SENTINEL));
    let currency = string(raw, &["currency", "currencyCode"])
        .unwrap_or("CNY")
        .to_uppercase();

    let mut canonical = match kind {
        BeeCountEntityKind::Ledger => json!({
            "meta": meta,
            "name": string(raw, &["ledgerName", "name"]).unwrap_or("BeeCount"),
            "currency": currency,
            "ledgerType": "personal",
            "monthStartDay": integer(raw, "monthStartDay").unwrap_or(1),
            "sortOrder": integer(raw, "sortOrder").unwrap_or(0),
            "isArchived": boolean(raw, "isArchived").unwrap_or(false),
        }),
        BeeCountEntityKind::Account => json!({
            "meta": meta,
            "name": string(raw, &["name"]).unwrap_or("Account"),
            "accountType": string(raw, &["type", "accountType"]).unwrap_or("other"),
            "openingBalanceCents": optional_amount_cents(raw.get("initialBalance"))?,
            "balanceAt": null,
            "last4": string(raw, &["cardLastFour"]),
            "color": string(raw, &["color"]).unwrap_or("#F59E0B"),
            "icon": string(raw, &["icon"]).unwrap_or("wallet"),
            "isArchived": false,
            "currency": currency,
            "ledgerId": ledger_entity_id,
            "sortOrder": integer(raw, "sortOrder").unwrap_or(0),
            "isHidden": boolean(raw, "hidden").unwrap_or(false),
            "note": raw.get("note").cloned().unwrap_or(Value::Null),
        }),
        BeeCountEntityKind::Category => json!({
            "meta": meta,
            "name": string(raw, &["name"]).unwrap_or("Category"),
            "categoryType": string(raw, &["kind", "categoryType"]).unwrap_or("expense"),
            "parentId": string(raw, &["parentSyncId"]).map(lifetrace_entity_id),
            "icon": raw.get("icon").cloned().unwrap_or(Value::Null),
            "color": raw.get("color").cloned().unwrap_or(Value::Null),
            "isSystem": false,
            "isArchived": false,
            "level": integer(raw, "level").unwrap_or(1),
            "sortOrder": integer(raw, "sortOrder").unwrap_or(0),
            "iconType": string(raw, &["iconType"]).unwrap_or("material"),
            "customIconPath": raw.get("customIconPath").cloned().unwrap_or(Value::Null),
            "communityIconId": raw.get("communityIconId").cloned().unwrap_or(Value::Null),
            "iconCloudFileId": raw.get("iconCloudFileId").cloned().unwrap_or(Value::Null),
            "iconCloudSha256": raw.get("iconCloudSha256").cloned().unwrap_or(Value::Null),
            "parentName": raw.get("parentName").cloned().unwrap_or(Value::Null),
        }),
        BeeCountEntityKind::Transaction => {
            let happened_at = string(raw, &["happenedAt", "occurredAt"]).ok_or_else(|| {
                BeeCountBoundaryError::InvalidPayload(
                    "transaction happenedAt is required".to_owned(),
                )
            })?;
            let local_date = happened_at.get(0..10).ok_or_else(|| {
                BeeCountBoundaryError::InvalidPayload(
                    "transaction happenedAt must start with YYYY-MM-DD".to_owned(),
                )
            })?;
            json!({
                "meta": meta,
                "transactionType": string(raw, &["type", "transactionType"]).unwrap_or("expense"),
                "amountCents": required_amount_cents(raw.get("amount"), "transaction amount")?,
                "currency": currency,
                "accountId": optional_namespaced_id(raw, &["accountId", "fromAccountId"]),
                "toAccountId": optional_namespaced_id(raw, &["toAccountId"]),
                "categoryId": optional_namespaced_id(raw, &["categoryId"]),
                "counterparty": null,
                "merchant": null,
                "item": null,
                "note": raw.get("note").cloned().unwrap_or(Value::Null),
                "occurredAt": happened_at,
                "localDate": local_date,
                "status": "confirmed",
                "sourceType": "beecount-cloud",
                "externalTransactionId": sync_id,
                "ledgerId": ledger_entity_id,
                "excludeFromStats": boolean(raw, "excludeFromStats").unwrap_or(false),
                "excludeFromBudget": boolean(raw, "excludeFromBudget").unwrap_or(false),
                "nativeAmountCents": optional_amount_cents(raw.get("nativeAmount"))?,
                "tagIds": namespaced_ids(raw.get("tagIds")),
                "attachments": raw.get("attachments").cloned().unwrap_or_else(|| json!([])),
                "categoryName": raw.get("categoryName").cloned().unwrap_or(Value::Null),
                "categoryKind": raw.get("categoryKind").cloned().unwrap_or(Value::Null),
                "accountName": raw.get("accountName").cloned().unwrap_or(Value::Null),
                "fromAccountName": raw.get("fromAccountName").cloned().unwrap_or(Value::Null),
                "toAccountName": raw.get("toAccountName").cloned().unwrap_or(Value::Null),
                "createdByUserId": raw.get("createdByUserId").cloned()
                    .unwrap_or_else(|| Value::String(user_id.as_str().to_owned())),
                "updatedByUserId": raw.get("updatedByUserId").cloned()
                    .unwrap_or_else(|| Value::String(user_id.as_str().to_owned())),
            })
        }
        BeeCountEntityKind::Tag => json!({
            "meta": meta,
            "ledgerId": ledger_entity_id,
            "name": string(raw, &["name"]).unwrap_or("Tag"),
            "color": raw.get("color").cloned().unwrap_or(Value::Null),
            "sortOrder": integer(raw, "sortOrder").unwrap_or(0),
            "isArchived": false,
        }),
        BeeCountEntityKind::Budget => json!({
            "meta": meta,
            "ledgerId": ledger_entity_id,
            "budgetType": string(raw, &["type", "budgetType"]).unwrap_or("total"),
            "categoryId": optional_namespaced_id(raw, &["categoryId"]),
            "amountCents": required_amount_cents(raw.get("amount"), "budget amount")?,
            "currency": currency,
            "period": string(raw, &["period"]).unwrap_or("monthly"),
            "startDay": integer(raw, "startDay").unwrap_or(1),
            "enabled": boolean(raw, "enabled").unwrap_or(true),
        }),
        BeeCountEntityKind::ExchangeRateOverride => json!({
            "meta": meta,
            "preferenceKey": format!("beecount.exchange_rate_override.{sync_id}"),
            "value": raw_payload,
        }),
    };
    let object = canonical
        .as_object_mut()
        .expect("canonical payload is object");
    object.insert("beecountPayload".to_owned(), raw_payload.clone());
    object.insert(
        "beecountEntityType".to_owned(),
        Value::String(kind.as_beecount_type().to_owned()),
    );
    object.insert(
        "beecountLedgerId".to_owned(),
        ledger_id.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );

    let entity_type = EntityType::new(kind.lifetrace_entity_type());
    EntityPayload::try_from((&entity_type, JsonValue(canonical.clone())))
        .map_err(BeeCountBoundaryError::InvalidPayload)?;
    Ok(JsonValue(canonical))
}

pub fn beecount_payload(
    kind: BeeCountEntityKind,
    sync_id: &str,
    canonical: &Value,
) -> Result<Value, BeeCountBoundaryError> {
    let row = canonical.as_object().ok_or_else(|| {
        BeeCountBoundaryError::InvalidPayload("canonical payload must be an object".to_owned())
    })?;
    let mut output = row
        .get("beecountPayload")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    output.insert("syncId".to_owned(), Value::String(sync_id.to_owned()));
    match kind {
        BeeCountEntityKind::Ledger => {
            copy(&mut output, "ledgerName", row.get("name"));
            copy(&mut output, "currency", row.get("currency"));
            copy(&mut output, "monthStartDay", row.get("monthStartDay"));
        }
        BeeCountEntityKind::Account => {
            copy(&mut output, "name", row.get("name"));
            copy(&mut output, "type", row.get("accountType"));
            copy(&mut output, "currency", row.get("currency"));
            cents_into(
                &mut output,
                "initialBalance",
                row.get("openingBalanceCents"),
            )?;
            copy(&mut output, "hidden", row.get("isHidden"));
            copy(&mut output, "note", row.get("note"));
            copy(&mut output, "sortOrder", row.get("sortOrder"));
        }
        BeeCountEntityKind::Category => {
            copy(&mut output, "name", row.get("name"));
            copy(&mut output, "kind", row.get("categoryType"));
            copy(&mut output, "icon", row.get("icon"));
            copy(&mut output, "level", row.get("level"));
            copy(&mut output, "sortOrder", row.get("sortOrder"));
            copy(&mut output, "iconType", row.get("iconType"));
            copy(&mut output, "customIconPath", row.get("customIconPath"));
            copy(&mut output, "communityIconId", row.get("communityIconId"));
            copy(&mut output, "iconCloudFileId", row.get("iconCloudFileId"));
            copy(&mut output, "iconCloudSha256", row.get("iconCloudSha256"));
            namespaced_into(&mut output, "parentSyncId", row.get("parentId"));
        }
        BeeCountEntityKind::Transaction => {
            copy(&mut output, "type", row.get("transactionType"));
            cents_into(&mut output, "amount", row.get("amountCents"))?;
            copy(&mut output, "currencyCode", row.get("currency"));
            copy(&mut output, "happenedAt", row.get("occurredAt"));
            copy(&mut output, "note", row.get("note"));
            namespaced_into(&mut output, "accountId", row.get("accountId"));
            namespaced_into(&mut output, "fromAccountId", row.get("accountId"));
            namespaced_into(&mut output, "toAccountId", row.get("toAccountId"));
            namespaced_into(&mut output, "categoryId", row.get("categoryId"));
            copy(&mut output, "excludeFromStats", row.get("excludeFromStats"));
            copy(
                &mut output,
                "excludeFromBudget",
                row.get("excludeFromBudget"),
            );
            cents_into(&mut output, "nativeAmount", row.get("nativeAmountCents"))?;
            ids_into(&mut output, "tagIds", row.get("tagIds"));
            copy(&mut output, "attachments", row.get("attachments"));
            for key in [
                "categoryName",
                "categoryKind",
                "accountName",
                "fromAccountName",
                "toAccountName",
                "createdByUserId",
                "updatedByUserId",
            ] {
                copy(&mut output, key, row.get(key));
            }
        }
        BeeCountEntityKind::Tag => {
            copy(&mut output, "name", row.get("name"));
            copy(&mut output, "color", row.get("color"));
            copy(&mut output, "sortOrder", row.get("sortOrder"));
        }
        BeeCountEntityKind::Budget => {
            copy(&mut output, "type", row.get("budgetType"));
            namespaced_into(&mut output, "categoryId", row.get("categoryId"));
            cents_into(&mut output, "amount", row.get("amountCents"))?;
            copy(&mut output, "period", row.get("period"));
            copy(&mut output, "startDay", row.get("startDay"));
            copy(&mut output, "enabled", row.get("enabled"));
        }
        BeeCountEntityKind::ExchangeRateOverride => {
            if let Some(value) = row.get("value").and_then(Value::as_object) {
                output.extend(value.clone());
            }
        }
    }
    Ok(Value::Object(output))
}

impl BeeCountEntityKind {
    pub fn as_beecount_type(self) -> &'static str {
        match self {
            Self::Ledger => "ledger",
            Self::Account => "account",
            Self::Category => "category",
            Self::Transaction => "transaction",
            Self::Tag => "tag",
            Self::Budget => "budget",
            Self::ExchangeRateOverride => "exchange_rate_override",
        }
    }

    pub fn from_lifetrace(entity_type: &str) -> Option<Self> {
        match entity_type {
            "finance.ledger" => Some(Self::Ledger),
            "finance.account" => Some(Self::Account),
            "finance.category" => Some(Self::Category),
            "finance.transaction" => Some(Self::Transaction),
            "finance.tag" => Some(Self::Tag),
            "finance.budget" => Some(Self::Budget),
            "user.preference" => Some(Self::ExchangeRateOverride),
            _ => None,
        }
    }
}

fn string<'a>(row: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| row.get(*key).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
}

fn integer(row: &Map<String, Value>, key: &str) -> Option<i64> {
    row.get(key).and_then(Value::as_i64)
}

fn boolean(row: &Map<String, Value>, key: &str) -> Option<bool> {
    row.get(key).and_then(Value::as_bool)
}

fn required_amount_cents(value: Option<&Value>, field: &str) -> Result<i64, BeeCountBoundaryError> {
    optional_amount_cents(value)?
        .ok_or_else(|| BeeCountBoundaryError::InvalidPayload(format!("{field} is required")))
}

fn optional_amount_cents(value: Option<&Value>) -> Result<Option<i64>, BeeCountBoundaryError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => decimal_amount_to_cents(&value.to_string()).map(Some),
        Some(Value::String(value)) => decimal_amount_to_cents(value).map(Some),
        Some(_) => Err(BeeCountBoundaryError::InvalidAmount),
    }
}

fn optional_namespaced_id(row: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    string(row, keys)
        .filter(|value| !value.is_empty())
        .map(lifetrace_entity_id)
}

fn namespaced_ids(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(lifetrace_entity_id)
                .collect()
        })
        .unwrap_or_default()
}

fn cents_value(cents: i64) -> Result<Value, BeeCountBoundaryError> {
    let negative = cents < 0;
    let magnitude = i128::from(cents).abs();
    let raw = format!(
        "{}{}.{:02}",
        if negative { "-" } else { "" },
        magnitude / 100,
        magnitude % 100
    );
    let number = raw
        .parse::<Number>()
        .map_err(|_| BeeCountBoundaryError::AmountOutOfRange)?;
    Ok(Value::Number(number))
}

fn copy(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value.filter(|value| !value.is_null()) {
        output.insert(key.to_owned(), value.clone());
    }
}

fn cents_into(
    output: &mut Map<String, Value>,
    key: &str,
    value: Option<&Value>,
) -> Result<(), BeeCountBoundaryError> {
    if let Some(cents) = value.and_then(Value::as_i64) {
        output.insert(key.to_owned(), cents_value(cents)?);
    }
    Ok(())
}

fn namespaced_into(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        output.insert(key.to_owned(), Value::String(beecount_wire_id(value)));
    }
}

fn ids_into(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(values) = value.and_then(Value::as_array) {
        output.insert(
            key.to_owned(),
            Value::Array(
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(beecount_wire_id)
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn entity_mapping_preserves_scope_and_namespace() {
        let transaction = BeeCountEntityKind::parse("transaction").unwrap();
        assert_eq!(transaction.lifetrace_entity_type(), "finance.transaction");
        assert_eq!(transaction.scope(), BeeCountScope::Ledger);
        assert_eq!(lifetrace_entity_id("tx-1"), "beecount:tx-1");
        assert_eq!(lifetrace_entity_id("beecount:tx-1"), "beecount:tx-1");
        assert_eq!(beecount_sync_id("beecount:tx-1"), "tx-1");
        assert_eq!(beecount_wire_id("tx-native"), "lifetrace:tx-native");
        assert_eq!(lifetrace_entity_id("lifetrace:tx-native"), "tx-native");
    }

    #[test]
    fn amount_conversion_is_exact_and_rounds_away_from_zero() {
        assert_eq!(decimal_amount_to_cents("12.34"), Ok(1234));
        assert_eq!(decimal_amount_to_cents("12.345"), Ok(1235));
        assert_eq!(decimal_amount_to_cents("-12.345"), Ok(-1235));
        assert_eq!(decimal_amount_to_cents("1.2e2"), Ok(12_000));
        assert_eq!(decimal_amount_to_cents(".009"), Ok(1));
    }

    #[test]
    fn lww_uses_timestamp_then_device_and_clamps_future_clock() {
        let now = Utc.with_ymd_and_hms(2026, 8, 13, 10, 0, 0).unwrap();
        assert_eq!(
            clamp_client_updated_at(now + Duration::minutes(3), now),
            now + Duration::seconds(5)
        );
        assert!(incoming_clock_wins(now, "device-b", now, "device-a"));
        assert!(!incoming_clock_wins(now, "device-a", now, "device-a"));
        assert!(!incoming_clock_wins(
            now - Duration::seconds(1),
            "device-z",
            now,
            "device-a"
        ));
    }

    #[test]
    fn transaction_payload_round_trips_through_integer_cents() {
        let updated = Utc.with_ymd_and_hms(2026, 8, 13, 10, 0, 0).unwrap();
        let raw = json!({
            "syncId": "tx-1",
            "type": "expense",
            "amount": 12.345,
            "happenedAt": "2026-08-13T10:00:00Z",
            "currencyCode": "CNY",
            "accountId": "cash-1",
            "tagIds": ["tag-1"],
        });
        let canonical = canonical_payload(
            BeeCountEntityKind::Transaction,
            "tx-1",
            Some("ledger-1"),
            &raw,
            &UserId::new("user-1"),
            "device-1",
            updated,
            None,
            1,
        )
        .unwrap();
        assert_eq!(canonical.0["amountCents"], 1235);
        assert_eq!(canonical.0["accountId"], "beecount:cash-1");
        assert_eq!(canonical.0["ledgerId"], "beecount:ledger-1");

        let output =
            beecount_payload(BeeCountEntityKind::Transaction, "tx-1", &canonical.0).unwrap();
        assert_eq!(output["amount"], 12.35);
        assert_eq!(output["accountId"], "cash-1");
        assert_eq!(output["tagIds"][0], "tag-1");
    }

    #[test]
    fn native_canonical_transaction_can_be_exposed_to_beecount() {
        let canonical = json!({
            "meta": {"id": "native-tx"},
            "transactionType": "income",
            "amountCents": 999,
            "currency": "CNY",
            "occurredAt": "2026-08-13T10:00:00Z",
            "note": "native",
            "ledgerId": "beecount:ledger-1",
        });
        let output =
            beecount_payload(BeeCountEntityKind::Transaction, "native-tx", &canonical).unwrap();
        assert_eq!(output["type"], "income");
        assert_eq!(output["amount"], 9.99);
        assert_eq!(output["happenedAt"], "2026-08-13T10:00:00Z");
    }

    #[test]
    fn sync_responses_keep_beecount_snake_case() {
        let response = BeeCountSyncPushResponse {
            accepted: 1,
            rejected: 0,
            conflict_count: 0,
            conflict_samples: vec![],
            server_cursor: 7,
            server_timestamp: Utc.with_ymd_and_hms(2026, 8, 13, 10, 0, 0).unwrap(),
        };
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["server_cursor"], 7);
        assert_eq!(value["conflict_count"], 0);
        assert!(value.get("serverCursor").is_none());
        assert!(value.get("conflictCount").is_none());
    }
}
