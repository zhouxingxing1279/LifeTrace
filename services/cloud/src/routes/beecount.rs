//! Authenticated, read-only LifeTrace view of BeeCount Cloud finance data.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use lifetrace_contracts::ErrorCode;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::auth::AuthenticatedPrincipal;
use crate::auth::{security::cookie_value, AuthCredential};
use crate::beecount_adapter::{BeeCountAdapter, BeeCountAdapterError, RawBeeCountSnapshot};
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
    let Some(adapter) = state.beecount_adapter.as_deref() else {
        return Ok(Json(json!({
            "enabled": false,
            "readOnly": true,
            "source": "beecount-cloud",
            "upstreamReachable": false
        })));
    };
    require_bound_user(adapter, &principal)?;
    match adapter.version().await {
        Ok(version) => Ok(Json(json!({
            "enabled": true,
            "readOnly": true,
            "source": "beecount-cloud",
            "upstreamReachable": true,
            "upstreamVersion": version
        }))),
        Err(_) => Ok(Json(json!({
            "enabled": true,
            "readOnly": true,
            "source": "beecount-cloud",
            "upstreamReachable": false
        }))),
    }
}

async fn ledgers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = integration_principal(&state, &headers).await?;
    let adapter = authorized_adapter(&state, &principal)?;
    let raw = adapter.ledgers().await.map_err(adapter_error)?;
    let rows = value_array(&raw)?;
    let items = rows
        .iter()
        .map(normalize_ledger)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(json!({
        "source": "beecount-cloud",
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
    if ledger_id.is_empty() || ledger_id.len() > 256 || query.limit == 0 || query.limit > 500 {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "ledger ID and limit must be valid",
            StatusCode::BAD_REQUEST,
        ));
    }
    let adapter = authorized_adapter(&state, &principal)?;
    let raw = adapter
        .ledger_snapshot(&ledger_id, query.limit, query.offset)
        .await
        .map_err(adapter_error)?;
    let normalized = normalize_snapshot(raw, query.limit, query.offset)?;
    Ok(Json(normalized))
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

fn authorized_adapter<'a>(
    state: &'a AppState,
    principal: &AuthenticatedPrincipal,
) -> Result<&'a BeeCountAdapter, ApiError> {
    principal.require_scope("finance:read")?;
    let adapter = state.beecount_adapter.as_deref().ok_or_else(|| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "BeeCount integration is disabled",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    require_bound_user(adapter, principal)?;
    Ok(adapter)
}

fn require_bound_user(
    adapter: &BeeCountAdapter,
    principal: &AuthenticatedPrincipal,
) -> Result<(), ApiError> {
    if principal.user_id.as_str() != adapter.bound_lifetrace_user_id() {
        return Err(ApiError::new(
            ErrorCode::AuthScopeDenied,
            "BeeCount integration is not bound to this account",
            StatusCode::FORBIDDEN,
        ));
    }
    Ok(())
}

fn adapter_error(error: BeeCountAdapterError) -> ApiError {
    let (message, status, retryable) = match error {
        BeeCountAdapterError::NotConfigured => (
            "BeeCount integration is disabled",
            StatusCode::SERVICE_UNAVAILABLE,
            false,
        ),
        BeeCountAdapterError::Unavailable => (
            "BeeCount integration is temporarily unavailable",
            StatusCode::SERVICE_UNAVAILABLE,
            true,
        ),
        BeeCountAdapterError::Authentication | BeeCountAdapterError::TwoFactorRequired => (
            "BeeCount integration authentication requires operator attention",
            StatusCode::BAD_GATEWAY,
            false,
        ),
        BeeCountAdapterError::UpstreamRejected
        | BeeCountAdapterError::InvalidResponse
        | BeeCountAdapterError::ResponseTooLarge => (
            "BeeCount integration returned an invalid upstream response",
            StatusCode::BAD_GATEWAY,
            false,
        ),
    };
    let mut api = ApiError::new(ErrorCode::TemporarilyUnavailable, message, status);
    api.body.retryable = retryable;
    api
}

fn normalize_snapshot(
    raw: RawBeeCountSnapshot,
    limit: usize,
    offset: usize,
) -> Result<Value, ApiError> {
    let ledger = normalize_ledger(&raw.ledger)?;
    let currency = ledger
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("CNY")
        .to_owned();
    let transaction_total = ledger
        .get("transactionCount")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let transactions = value_array(&raw.transactions)?
        .iter()
        .map(|item| normalize_transaction(item, &currency))
        .collect::<Result<Vec<_>, _>>()?;
    let accounts = value_array(&raw.accounts)?
        .iter()
        .map(normalize_account)
        .collect::<Result<Vec<_>, _>>()?;
    let categories = value_array(&raw.categories)?
        .iter()
        .map(normalize_category)
        .collect::<Result<Vec<_>, _>>()?;
    let tags = value_array(&raw.tags)?
        .iter()
        .map(normalize_tag)
        .collect::<Result<Vec<_>, _>>()?;
    let budgets = value_array(&raw.budgets)?
        .iter()
        .map(normalize_budget)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(json!({
        "source": "beecount-cloud",
        "readOnly": true,
        "fetchedAt": Utc::now(),
        "ledger": ledger,
        "transactions": {
            "items": transactions,
            "total": transaction_total,
            "limit": limit,
            "offset": offset
        },
        "accounts": accounts,
        "categories": categories,
        "tags": tags,
        "budgets": budgets
    }))
}

fn normalize_ledger(value: &Value) -> Result<Value, ApiError> {
    let row = value_object(value)?;
    let source_id = required_string(row, "ledger_id")?;
    Ok(json!({
        "id": namespaced(source_id),
        "sourceId": source_id,
        "name": required_string(row, "ledger_name")?,
        "currency": optional_string(row, "currency").unwrap_or("CNY"),
        "monthStartDay": optional_u64(row, "month_start_day").unwrap_or(1),
        "transactionCount": optional_u64(row, "transaction_count").unwrap_or(0),
        "incomeTotalCents": money_field(row, "income_total")?,
        "expenseTotalCents": money_field(row, "expense_total")?,
        "balanceCents": money_field(row, "balance")?,
        "sourceChangeId": optional_u64(row, "source_change_id"),
        "updatedAt": row.get("updated_at").cloned().unwrap_or(Value::Null),
        "role": row.get("role").cloned().unwrap_or(json!("viewer")),
        "isShared": row.get("is_shared").cloned().unwrap_or(json!(false)),
        "memberCount": optional_u64(row, "member_count").unwrap_or(1),
        "source": "beecount-cloud",
        "readOnly": true
    }))
}

fn normalize_transaction(value: &Value, ledger_currency: &str) -> Result<Value, ApiError> {
    let row = value_object(value)?;
    let source_id = required_string(row, "id")?;
    let occurred_at = required_string(row, "happened_at")?;
    let currency = optional_string(row, "currency_code").unwrap_or(ledger_currency);
    Ok(json!({
        "id": namespaced(source_id),
        "externalTransactionId": source_id,
        "transactionType": required_string(row, "tx_type")?,
        "amountCents": money_field(row, "amount")?,
        "nativeAmountCents": optional_money_field(row, "native_amount")?,
        "currency": currency,
        "occurredAt": occurred_at,
        "localDate": occurred_at.get(0..10),
        "status": "confirmed",
        "sourceType": "beecount-cloud",
        "note": row.get("note").cloned().unwrap_or(Value::Null),
        "ledgerId": optional_namespaced(row, "ledger_id"),
        "ledgerName": row.get("ledger_name").cloned().unwrap_or(Value::Null),
        "accountId": optional_namespaced(row, "account_id"),
        "toAccountId": optional_namespaced(row, "to_account_id"),
        "categoryId": optional_namespaced(row, "category_id"),
        "accountName": row.get("account_name").cloned().unwrap_or(Value::Null),
        "fromAccountName": row.get("from_account_name").cloned().unwrap_or(Value::Null),
        "toAccountName": row.get("to_account_name").cloned().unwrap_or(Value::Null),
        "categoryName": row.get("category_name").cloned().unwrap_or(Value::Null),
        "tags": row.get("tags_list").cloned().unwrap_or_else(|| json!([])),
        "tagIds": namespaced_array(row.get("tag_ids")),
        "attachments": row
            .get("attachments")
            .filter(|value| value.is_array())
            .cloned()
            .unwrap_or_else(|| json!([])),
        "excludeFromStats": row.get("exclude_from_stats").cloned().unwrap_or(json!(false)),
        "excludeFromBudget": row.get("exclude_from_budget").cloned().unwrap_or(json!(false)),
        "sourceChangeId": optional_u64(row, "last_change_id"),
        "readOnly": true
    }))
}

fn normalize_account(value: &Value) -> Result<Value, ApiError> {
    let row = value_object(value)?;
    let source_id = required_string(row, "id")?;
    Ok(json!({
        "id": namespaced(source_id),
        "sourceId": source_id,
        "name": required_string(row, "name")?,
        "accountType": row.get("account_type").cloned().unwrap_or(Value::Null),
        "currency": row.get("currency").cloned().unwrap_or(Value::Null),
        "openingBalanceCents": optional_money_field(row, "initial_balance")?,
        "balanceCents": optional_money_field(row, "balance")?,
        "incomeTotalCents": optional_money_field(row, "income_total")?,
        "expenseTotalCents": optional_money_field(row, "expense_total")?,
        "transactionCount": optional_u64(row, "tx_count"),
        "hidden": row.get("hidden").cloned().unwrap_or(json!(false)),
        "note": row.get("note").cloned().unwrap_or(Value::Null),
        "source": "beecount-cloud",
        "readOnly": true
    }))
}

fn normalize_category(value: &Value) -> Result<Value, ApiError> {
    let row = value_object(value)?;
    let source_id = required_string(row, "id")?;
    Ok(json!({
        "id": namespaced(source_id),
        "sourceId": source_id,
        "name": required_string(row, "name")?,
        "categoryType": required_string(row, "kind")?,
        "level": optional_u64(row, "level"),
        "sortOrder": optional_u64(row, "sort_order"),
        "icon": row.get("icon").cloned().unwrap_or(Value::Null),
        "parentName": row.get("parent_name").cloned().unwrap_or(Value::Null),
        "transactionCount": optional_u64(row, "tx_count"),
        "source": "beecount-cloud",
        "readOnly": true
    }))
}

fn normalize_tag(value: &Value) -> Result<Value, ApiError> {
    let row = value_object(value)?;
    let source_id = required_string(row, "id")?;
    Ok(json!({
        "id": namespaced(source_id),
        "sourceId": source_id,
        "name": required_string(row, "name")?,
        "color": row.get("color").cloned().unwrap_or(Value::Null),
        "transactionCount": optional_u64(row, "tx_count"),
        "incomeTotalCents": optional_money_field(row, "income_total")?,
        "expenseTotalCents": optional_money_field(row, "expense_total")?,
        "source": "beecount-cloud",
        "readOnly": true
    }))
}

fn normalize_budget(value: &Value) -> Result<Value, ApiError> {
    let row = value_object(value)?;
    let source_id = required_string(row, "id")?;
    Ok(json!({
        "id": namespaced(source_id),
        "sourceId": source_id,
        "budgetType": required_string(row, "type")?,
        "categoryId": optional_namespaced(row, "category_id"),
        "categoryName": row.get("category_name").cloned().unwrap_or(Value::Null),
        "amountCents": money_field(row, "amount")?,
        "period": required_string(row, "period")?,
        "startDay": optional_u64(row, "start_day").unwrap_or(1),
        "enabled": row.get("enabled").cloned().unwrap_or(json!(true)),
        "source": "beecount-cloud",
        "readOnly": true
    }))
}

fn value_object(value: &Value) -> Result<&Map<String, Value>, ApiError> {
    value.as_object().ok_or_else(invalid_upstream)
}

fn value_array(value: &Value) -> Result<&Vec<Value>, ApiError> {
    value.as_array().ok_or_else(invalid_upstream)
}

fn required_string<'a>(row: &'a Map<String, Value>, key: &str) -> Result<&'a str, ApiError> {
    row.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid_upstream)
}

fn optional_string<'a>(row: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    row.get(key).and_then(Value::as_str)
}

fn optional_u64(row: &Map<String, Value>, key: &str) -> Option<u64> {
    row.get(key).and_then(Value::as_u64)
}

fn money_field(row: &Map<String, Value>, key: &str) -> Result<i64, ApiError> {
    row.get(key)
        .and_then(decimal_to_cents)
        .ok_or_else(invalid_upstream)
}

fn optional_money_field(row: &Map<String, Value>, key: &str) -> Result<Option<i64>, ApiError> {
    match row.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => decimal_to_cents(value)
            .map(Some)
            .ok_or_else(invalid_upstream),
    }
}

pub(crate) fn decimal_to_cents(value: &Value) -> Option<i64> {
    let raw = match value {
        Value::Number(number) => number.to_string(),
        Value::String(value) => value.trim().to_owned(),
        _ => return None,
    };
    if raw.contains('e') || raw.contains('E') {
        return raw
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .and_then(|number| (number * 100.0).round().to_string().parse().ok());
    }
    let (negative, unsigned) = raw
        .strip_prefix('-')
        .map_or((false, raw.as_str()), |value| (true, value));
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let mut digits = fraction.bytes().map(|byte| (byte - b'0') as i64);
    let first = digits.next().unwrap_or(0);
    let second = digits.next().unwrap_or(0);
    let round_up = digits.next().unwrap_or(0) >= 5;
    let mut cents = whole.checked_mul(100)?.checked_add(first * 10 + second)?;
    if round_up {
        cents = cents.checked_add(1)?;
    }
    if negative {
        cents.checked_neg()
    } else {
        Some(cents)
    }
}

fn namespaced(id: &str) -> String {
    format!("beecount:{id}")
}

fn optional_namespaced(row: &Map<String, Value>, key: &str) -> Option<String> {
    optional_string(row, key).map(namespaced)
}

fn namespaced_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(namespaced)
                .collect()
        })
        .unwrap_or_default()
}

fn invalid_upstream() -> ApiError {
    adapter_error(BeeCountAdapterError::InvalidResponse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_money_is_converted_to_integer_cents() {
        assert_eq!(decimal_to_cents(&json!(12.34)), Some(1234));
        assert_eq!(decimal_to_cents(&json!("0.105")), Some(11));
        assert_eq!(decimal_to_cents(&json!("-2.345")), Some(-235));
        assert_eq!(decimal_to_cents(&json!("92233720368547758.08")), None);
    }
}
