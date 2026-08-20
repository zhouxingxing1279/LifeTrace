//! LifeTrace Web finance facade over the BeeCount-compatible PostgreSQL entity store.
//!
//! This route intentionally does not call an external BeeCount Cloud instance.
//! BeeCount Android compatibility writes and LifeTrace Web finance reads both use
//! the same `sync_entities` / `sync_change_log` authoritative store.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use lifetrace_contracts::ErrorCode;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::auth::security::cookie_value;
use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::beecount_compat::{decimal_amount_to_cents, lifetrace_entity_id, BeeCountReadLedgerOut};
use crate::beecount_sync::BeeCountSyncService;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/integrations/beecount/status", get(status))
        .route("/api/v1/integrations/beecount/ledgers", get(ledgers))
        .route(
            "/api/v1/integrations/beecount/ledgers/{ledger_id}/snapshot",
            get(snapshot),
        )
}

#[derive(Debug, Deserialize)]
struct SnapshotQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    200
}

async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = integration_principal(&state, &headers).await?;
    principal.require_scope("finance:read")?;
    Ok(Json(json!({
        "enabled": state.database_enabled,
        "readOnly": true,
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "upstreamReachable": state.database_enabled,
        "upstreamVersion": {
            "name": "LifeTrace BeeCount compatibility",
            "version": 1,
            "authoritativeStore": "sync_entities"
        }
    })))
}

async fn ledgers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = integration_principal(&state, &headers).await?;
    principal.require_scope("finance:read")?;
    require_database(&state)?;
    let rows = BeeCountSyncService::new(state.pool.clone())
        .read_ledgers(&principal.user_id)
        .await?;
    let items = rows.iter().map(normalize_ledger).collect::<Vec<_>>();
    Ok(Json(json!({
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "readOnly": true,
        "items": items,
        "fetchedAt": Utc::now()
    })))
}

async fn snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ledger_id): Path<String>,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<Value>, ApiError> {
    let principal = integration_principal(&state, &headers).await?;
    principal.require_scope("finance:read")?;
    require_database(&state)?;
    if ledger_id.is_empty() || ledger_id.len() > 256 || query.limit == 0 || query.limit > 500 {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "ledger ID and limit must be valid",
            StatusCode::BAD_REQUEST,
        ));
    }

    let service = BeeCountSyncService::new(state.pool.clone());
    let ledger = service
        .read_ledgers(&principal.user_id)
        .await?
        .into_iter()
        .find(|item| item.ledger_id == ledger_id)
        .ok_or_else(|| not_found("BeeCount ledger not found"))?;
    let full = service.full(&principal.user_id, &ledger_id).await?;
    let snapshot = full
        .snapshot
        .ok_or_else(|| not_found("BeeCount ledger snapshot not found"))?;
    let content_raw = snapshot
        .payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| internal("BeeCount snapshot content is missing"))?;
    let content: Value = serde_json::from_str(content_raw)
        .map_err(|_| internal("BeeCount snapshot content is invalid"))?;
    let content = content
        .as_object()
        .ok_or_else(|| internal("BeeCount snapshot must be an object"))?;

    let accounts_raw = array(content, "accounts");
    let categories_raw = array(content, "categories");
    let tags_raw = array(content, "tags");
    let budgets_raw = array(content, "budgets");
    let transactions_raw = array(content, "items");

    let account_names = name_map(&accounts_raw);
    let category_names = name_map(&categories_raw);
    let tag_names = name_map(&tags_raw);

    let total = transactions_raw.len();
    let transactions = transactions_raw
        .iter()
        .skip(query.offset)
        .take(query.limit)
        .map(|value| {
            normalize_transaction(value, &ledger, &account_names, &category_names, &tag_names)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let accounts = accounts_raw
        .iter()
        .map(|value| normalize_account(value, &ledger, &transactions_raw))
        .collect::<Result<Vec<_>, _>>()?;
    let categories = categories_raw
        .iter()
        .map(|value| normalize_category(value, &transactions_raw, &category_names))
        .collect::<Result<Vec<_>, _>>()?;
    let tags = tags_raw
        .iter()
        .map(|value| normalize_tag(value, &transactions_raw))
        .collect::<Result<Vec<_>, _>>()?;
    let budgets = budgets_raw
        .iter()
        .map(|value| normalize_budget(value, &category_names))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(json!({
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "readOnly": true,
        "fetchedAt": Utc::now(),
        "ledger": normalize_ledger(&ledger),
        "transactions": {
            "items": transactions,
            "total": total,
            "limit": query.limit,
            "offset": query.offset
        },
        "accounts": accounts,
        "categories": categories,
        "tags": tags,
        "budgets": budgets
    })))
}

async fn integration_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedPrincipal, ApiError> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if authorization.is_some() {
        return state
            .auth
            .authenticate(AuthCredential::Bearer(authorization))
            .await;
    }
    let session = cookie_value(headers, &state.config.auth_cookie_name);
    state
        .auth
        .authenticate(AuthCredential::WebSession(session.as_deref()))
        .await
}

fn require_database(state: &AppState) -> Result<(), ApiError> {
    if state.database_enabled {
        Ok(())
    } else {
        Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "BeeCount finance requires the LifeTrace PostgreSQL store",
            StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

fn normalize_ledger(row: &BeeCountReadLedgerOut) -> Value {
    json!({
        "id": lifetrace_entity_id(&row.ledger_id),
        "sourceId": row.ledger_id,
        "name": row.ledger_name,
        "currency": row.currency,
        "monthStartDay": row.month_start_day,
        "transactionCount": row.transaction_count,
        "incomeTotalCents": decimal_f64_to_cents(row.income_total),
        "expenseTotalCents": decimal_f64_to_cents(row.expense_total),
        "balanceCents": decimal_f64_to_cents(row.balance),
        "updatedAt": row.updated_at,
        "role": row.role,
        "isShared": row.is_shared,
        "memberCount": row.member_count,
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "readOnly": true
    })
}

fn normalize_transaction(
    value: &Value,
    ledger: &BeeCountReadLedgerOut,
    account_names: &HashMap<String, String>,
    category_names: &HashMap<String, String>,
    tag_names: &HashMap<String, String>,
) -> Result<Value, ApiError> {
    let row = object(value)?;
    let source_id = required_string(row, "syncId")?;
    let occurred_at = required_string(row, "happenedAt")?;
    let account_source_id =
        optional_string(row, "accountId").or_else(|| optional_string(row, "fromAccountId"));
    let to_account_source_id = optional_string(row, "toAccountId");
    let category_source_id = optional_string(row, "categoryId");
    let tag_source_ids = string_array(row.get("tagIds"));
    let tags = tag_source_ids
        .iter()
        .filter_map(|id| tag_names.get(id).cloned())
        .collect::<Vec<_>>();
    let account_name = optional_string(row, "accountName")
        .map(str::to_owned)
        .or_else(|| account_source_id.and_then(|id| account_names.get(id).cloned()));
    let from_account_name = optional_string(row, "fromAccountName")
        .map(str::to_owned)
        .or_else(|| account_source_id.and_then(|id| account_names.get(id).cloned()));
    let to_account_name = optional_string(row, "toAccountName")
        .map(str::to_owned)
        .or_else(|| to_account_source_id.and_then(|id| account_names.get(id).cloned()));
    let category_name = optional_string(row, "categoryName")
        .map(str::to_owned)
        .or_else(|| category_source_id.and_then(|id| category_names.get(id).cloned()));
    let currency = optional_string(row, "currencyCode").unwrap_or(&ledger.currency);

    Ok(json!({
        "id": lifetrace_entity_id(source_id),
        "externalTransactionId": source_id,
        "transactionType": required_string(row, "type")?,
        "amountCents": amount_cents(row.get("amount"), "transaction amount")?,
        "nativeAmountCents": optional_amount_cents(row.get("nativeAmount"))?,
        "currency": currency,
        "occurredAt": occurred_at,
        "localDate": occurred_at.get(0..10),
        "status": "confirmed",
        "sourceType": "beecount-cloud",
        "note": row.get("note").cloned().unwrap_or(Value::Null),
        "ledgerId": lifetrace_entity_id(&ledger.ledger_id),
        "ledgerName": ledger.ledger_name,
        "accountId": account_source_id.map(lifetrace_entity_id),
        "toAccountId": to_account_source_id.map(lifetrace_entity_id),
        "categoryId": category_source_id.map(lifetrace_entity_id),
        "accountName": account_name,
        "fromAccountName": from_account_name,
        "toAccountName": to_account_name,
        "categoryName": category_name,
        "tags": tags,
        "tagIds": tag_source_ids.iter().map(|id| lifetrace_entity_id(id)).collect::<Vec<_>>(),
        "attachments": row.get("attachments").filter(|v| v.is_array()).cloned().unwrap_or_else(|| json!([])),
        "excludeFromStats": optional_bool(row, "excludeFromStats").unwrap_or(false),
        "excludeFromBudget": optional_bool(row, "excludeFromBudget").unwrap_or(false),
        "readOnly": true
    }))
}

fn normalize_account(
    value: &Value,
    ledger: &BeeCountReadLedgerOut,
    transactions: &[Value],
) -> Result<Value, ApiError> {
    let row = object(value)?;
    let source_id = required_string(row, "syncId")?;
    let opening = optional_amount_cents(row.get("initialBalance"))?.unwrap_or(0);
    let mut balance = opening;
    let mut income = 0_i64;
    let mut expense = 0_i64;
    let mut count = 0_i64;
    for transaction in transactions {
        let Ok(tx) = object(transaction) else {
            continue;
        };
        let from_id =
            optional_string(tx, "accountId").or_else(|| optional_string(tx, "fromAccountId"));
        let to_id = optional_string(tx, "toAccountId");
        let amount = optional_amount_cents(tx.get("amount"))?.unwrap_or(0);
        let kind = optional_string(tx, "type").unwrap_or("expense");
        if from_id == Some(source_id) || to_id == Some(source_id) {
            count += 1;
        }
        match kind {
            "income" | "refund" if from_id == Some(source_id) => {
                balance += amount;
                income += amount;
            }
            "expense" | "fee" if from_id == Some(source_id) => {
                balance -= amount;
                expense += amount;
            }
            "transfer" => {
                if from_id == Some(source_id) {
                    balance -= amount;
                }
                if to_id == Some(source_id) {
                    balance += amount;
                }
            }
            _ => {}
        }
    }
    Ok(json!({
        "id": lifetrace_entity_id(source_id),
        "sourceId": source_id,
        "name": required_string(row, "name")?,
        "accountType": row.get("type").cloned().unwrap_or(Value::Null),
        "currency": optional_string(row, "currency").unwrap_or(&ledger.currency),
        "openingBalanceCents": opening,
        "balanceCents": balance,
        "incomeTotalCents": income,
        "expenseTotalCents": expense,
        "transactionCount": count,
        "hidden": optional_bool(row, "hidden").unwrap_or(false),
        "note": row.get("note").cloned().unwrap_or(Value::Null),
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "readOnly": true
    }))
}

fn normalize_category(
    value: &Value,
    transactions: &[Value],
    category_names: &HashMap<String, String>,
) -> Result<Value, ApiError> {
    let row = object(value)?;
    let source_id = required_string(row, "syncId")?;
    let count = transactions
        .iter()
        .filter(|value| {
            object(value)
                .ok()
                .and_then(|tx| optional_string(tx, "categoryId"))
                == Some(source_id)
        })
        .count();
    let parent_id = optional_string(row, "parentSyncId");
    Ok(json!({
        "id": lifetrace_entity_id(source_id),
        "sourceId": source_id,
        "name": required_string(row, "name")?,
        "categoryType": optional_string(row, "kind").unwrap_or("expense"),
        "level": optional_i64(row, "level"),
        "sortOrder": optional_i64(row, "sortOrder"),
        "icon": row.get("icon").cloned().unwrap_or(Value::Null),
        "parentName": parent_id.and_then(|id| category_names.get(id).cloned()),
        "transactionCount": count,
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "readOnly": true
    }))
}

fn normalize_tag(value: &Value, transactions: &[Value]) -> Result<Value, ApiError> {
    let row = object(value)?;
    let source_id = required_string(row, "syncId")?;
    let mut count = 0_i64;
    let mut income = 0_i64;
    let mut expense = 0_i64;
    for transaction in transactions {
        let Ok(tx) = object(transaction) else {
            continue;
        };
        if !string_array(tx.get("tagIds"))
            .iter()
            .any(|id| id == source_id)
        {
            continue;
        }
        count += 1;
        let amount = optional_amount_cents(tx.get("amount"))?.unwrap_or(0);
        match optional_string(tx, "type").unwrap_or("expense") {
            "income" | "refund" => income += amount,
            "expense" | "fee" => expense += amount,
            _ => {}
        }
    }
    Ok(json!({
        "id": lifetrace_entity_id(source_id),
        "sourceId": source_id,
        "name": required_string(row, "name")?,
        "color": row.get("color").cloned().unwrap_or(Value::Null),
        "transactionCount": count,
        "incomeTotalCents": income,
        "expenseTotalCents": expense,
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "readOnly": true
    }))
}

fn normalize_budget(
    value: &Value,
    category_names: &HashMap<String, String>,
) -> Result<Value, ApiError> {
    let row = object(value)?;
    let source_id = required_string(row, "syncId")?;
    let category_id = optional_string(row, "categoryId");
    Ok(json!({
        "id": lifetrace_entity_id(source_id),
        "sourceId": source_id,
        "budgetType": optional_string(row, "type").unwrap_or("total"),
        "categoryId": category_id.map(lifetrace_entity_id),
        "categoryName": category_id.and_then(|id| category_names.get(id).cloned()),
        "amountCents": amount_cents(row.get("amount"), "budget amount")?,
        "period": optional_string(row, "period").unwrap_or("monthly"),
        "startDay": optional_i64(row, "startDay").unwrap_or(1),
        "enabled": optional_bool(row, "enabled").unwrap_or(true),
        "source": "beecount-cloud",
        "storage": "lifetrace-postgresql",
        "readOnly": true
    }))
}

fn array<'a>(row: &'a Map<String, Value>, key: &str) -> Vec<Value> {
    row.get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn name_map(values: &[Value]) -> HashMap<String, String> {
    values
        .iter()
        .filter_map(|value| {
            let row = value.as_object()?;
            Some((
                row.get("syncId")?.as_str()?.to_owned(),
                row.get("name")?.as_str()?.to_owned(),
            ))
        })
        .collect()
}

fn object(value: &Value) -> Result<&Map<String, Value>, ApiError> {
    value
        .as_object()
        .ok_or_else(|| internal("BeeCount entity must be an object"))
}

fn required_string<'a>(row: &'a Map<String, Value>, key: &str) -> Result<&'a str, ApiError> {
    optional_string(row, key).ok_or_else(|| internal(&format!("BeeCount field {key} is missing")))
}

fn optional_string<'a>(row: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    row.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn optional_bool(row: &Map<String, Value>, key: &str) -> Option<bool> {
    row.get(key).and_then(Value::as_bool)
}

fn optional_i64(row: &Map<String, Value>, key: &str) -> Option<i64> {
    row.get(key).and_then(Value::as_i64)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn amount_cents(value: Option<&Value>, field: &str) -> Result<i64, ApiError> {
    optional_amount_cents(value)?.ok_or_else(|| internal(&format!("BeeCount {field} is missing")))
}

fn optional_amount_cents(value: Option<&Value>) -> Result<Option<i64>, ApiError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => decimal_amount_to_cents(&number.to_string())
            .map(Some)
            .map_err(|_| internal("BeeCount amount is invalid")),
        Some(Value::String(raw)) => decimal_amount_to_cents(raw)
            .map(Some)
            .map_err(|_| internal("BeeCount amount is invalid")),
        Some(_) => Err(internal("BeeCount amount is invalid")),
    }
}

fn decimal_f64_to_cents(value: f64) -> i64 {
    (value * 100.0).round() as i64
}

fn not_found(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::NOT_FOUND)
}

fn internal(message: &str) -> ApiError {
    ApiError::new(
        ErrorCode::InternalError,
        message,
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}
