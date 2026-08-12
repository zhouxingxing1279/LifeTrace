//! Finance domain DTOs.
//!
//! The model is intentionally compatible with LifeTrace's existing sync
//! protocol while covering the useful single-user bookkeeping domain exposed
//! by BeeCount. Money remains integer cents on the wire.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::domain::enums::{AccountType, TransactionStatus, TransactionType};
use crate::ids::EntityId;
use crate::money::CurrencyCode;
use crate::time::{LocalDate, UtcTimestamp};

fn default_personal() -> String { "personal".to_owned() }
fn default_month_start_day() -> i32 { 1 }
fn default_one() -> i32 { 1 }
fn default_monthly() -> String { "monthly".to_owned() }
fn default_total() -> String { "total".to_owned() }
fn default_material() -> String { "material".to_owned() }
fn default_true() -> bool { true }

/// `finance.ledger`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceLedger {
    pub meta: EntityMeta,
    pub name: String,
    pub currency: CurrencyCode,
    #[serde(default = "default_personal")]
    pub ledger_type: String,
    #[serde(default = "default_month_start_day")]
    pub month_start_day: i32,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub is_archived: bool,
}

/// `finance.account`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceAccount {
    pub meta: EntityMeta,
    #[serde(default)]
    pub ledger_id: Option<EntityId>,
    pub name: String,
    pub account_type: AccountType,
    /// Baseline balance in cents at `balance_at`.
    pub opening_balance_cents: Option<i64>,
    pub balance_at: Option<UtcTimestamp>,
    pub last4: Option<String>,
    pub color: String,
    pub icon: String,
    pub is_archived: bool,
    pub currency: CurrencyCode,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub credit_limit_cents: Option<i64>,
    #[serde(default)]
    pub billing_day: Option<i32>,
    #[serde(default)]
    pub payment_due_day: Option<i32>,
    #[serde(default)]
    pub bank_name: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub is_hidden: bool,
}

/// `finance.category`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TransactionCategory {
    pub meta: EntityMeta,
    #[serde(default)]
    pub ledger_id: Option<EntityId>,
    pub name: String,
    pub category_type: TransactionType,
    pub parent_id: Option<EntityId>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub is_system: bool,
    pub is_archived: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default = "default_one")]
    pub level: i32,
    #[serde(default = "default_material")]
    pub icon_type: String,
    #[serde(default)]
    pub custom_icon_file_id: Option<EntityId>,
}

/// `finance.transaction`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Transaction {
    pub meta: EntityMeta,
    #[serde(default)]
    pub ledger_id: Option<EntityId>,
    pub transaction_type: TransactionType,
    /// Amount in cents in `currency`. Never floats on the wire.
    pub amount_cents: i64,
    pub currency: CurrencyCode,
    pub account_id: Option<EntityId>,
    pub to_account_id: Option<EntityId>,
    pub category_id: Option<EntityId>,
    pub counterparty: Option<String>,
    pub merchant: Option<String>,
    pub item: Option<String>,
    pub note: Option<String>,
    pub occurred_at: UtcTimestamp,
    /// Business natural day `YYYY-MM-DD`; never derived from UTC slicing.
    pub local_date: LocalDate,
    pub status: TransactionStatus,
    pub source_type: String,
    pub external_transaction_id: Option<String>,
    #[serde(default)]
    pub recurring_transaction_id: Option<EntityId>,
    /// Exclude only from income/expense statistics; balances are unaffected.
    #[serde(default)]
    pub exclude_from_stats: bool,
    /// Independent budget exclusion switch.
    #[serde(default)]
    pub exclude_from_budget: bool,
    /// Frozen amount converted to the ledger base currency at bookkeeping time.
    #[serde(default)]
    pub native_amount_cents: Option<i64>,
    #[serde(default)]
    pub native_currency: Option<CurrencyCode>,
    /// Decimal string snapshot, never a binary floating point wire value.
    #[serde(default)]
    pub exchange_rate: Option<String>,
}

/// `finance.recurring_transaction`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RecurringTransaction {
    pub meta: EntityMeta,
    pub ledger_id: EntityId,
    pub transaction_type: TransactionType,
    pub amount_cents: i64,
    pub currency: CurrencyCode,
    pub category_id: Option<EntityId>,
    pub account_id: Option<EntityId>,
    pub to_account_id: Option<EntityId>,
    pub note: Option<String>,
    pub frequency: String,
    #[serde(default = "default_one")]
    pub interval: i32,
    pub day_of_month: Option<i32>,
    pub day_of_week: Option<i32>,
    pub month_of_year: Option<i32>,
    pub start_date: LocalDate,
    pub end_date: Option<LocalDate>,
    pub last_generated_date: Option<LocalDate>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// `finance.tag`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceTag {
    pub meta: EntityMeta,
    pub ledger_id: EntityId,
    pub name: String,
    pub color: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub is_archived: bool,
}

/// `finance.transaction_tag`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceTransactionTag {
    pub meta: EntityMeta,
    pub transaction_id: EntityId,
    pub tag_id: EntityId,
}

/// `finance.budget`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceBudget {
    pub meta: EntityMeta,
    pub ledger_id: EntityId,
    #[serde(default = "default_total")]
    pub budget_type: String,
    pub category_id: Option<EntityId>,
    pub amount_cents: i64,
    pub currency: CurrencyCode,
    #[serde(default = "default_monthly")]
    pub period: String,
    #[serde(default = "default_one")]
    pub start_day: i32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// `finance.transaction_attachment`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TransactionAttachment {
    pub meta: EntityMeta,
    pub transaction_id: EntityId,
    pub file_name: String,
    pub original_name: Option<String>,
    pub file_size: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    #[serde(default)]
    pub sort_order: i32,
    /// Reference into LifeTrace's existing file subsystem; no finance-specific
    /// object storage is introduced.
    pub file_id: Option<EntityId>,
    pub sha256: Option<String>,
}

/// `finance.transaction_evidence`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TransactionEvidence {
    pub meta: EntityMeta,
    pub transaction_id: EntityId,
    pub source_type: String,
    pub source_id: Option<String>,
    pub external_transaction_id: Option<String>,
    pub confidence: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{EntityId, UserId};

    fn stamp() -> UtcTimestamp { "2026-08-04T15:30:00Z".parse().unwrap() }

    fn meta(id: &str) -> EntityMeta {
        EntityMeta {
            id: EntityId::new(id),
            user_id: UserId::new("local-user"),
            created_at: stamp(),
            updated_at: stamp(),
            deleted_at: None,
            local_version: 1,
            server_version: None,
            modified_by_device: None,
        }
    }

    #[test]
    fn old_transaction_payload_remains_readable() {
        let value = serde_json::json!({
            "meta": serde_json::to_value(meta("tx-old")).unwrap(),
            "transactionType": "expense",
            "amountCents": 12525,
            "currency": "CNY",
            "accountId": null,
            "toAccountId": null,
            "categoryId": null,
            "counterparty": null,
            "merchant": null,
            "item": null,
            "note": null,
            "occurredAt": "2026-08-04T15:30:00Z",
            "localDate": "2026-08-04",
            "status": "confirmed",
            "sourceType": "manual",
            "externalTransactionId": null
        });
        let tx: Transaction = serde_json::from_value(value).unwrap();
        assert_eq!(tx.ledger_id, None);
        assert!(!tx.exclude_from_stats);
        assert!(!tx.exclude_from_budget);
    }

    #[test]
    fn transaction_wire_keeps_integer_money_and_flags() {
        let transaction = Transaction {
            meta: meta("tx-1"),
            ledger_id: Some(EntityId::new("ledger-1")),
            transaction_type: TransactionType::new(TransactionType::EXPENSE),
            amount_cents: 12525,
            currency: CurrencyCode::cny(),
            account_id: Some(EntityId::new("wechat-wallet")),
            to_account_id: None,
            category_id: Some(EntityId::new("cat-food")),
            counterparty: Some("coffee shop".to_owned()),
            merchant: None,
            item: Some("latte".to_owned()),
            note: None,
            occurred_at: stamp(),
            local_date: LocalDate::new("2026-08-04").unwrap(),
            status: TransactionStatus::new(TransactionStatus::CONFIRMED),
            source_type: "manual".to_owned(),
            external_transaction_id: None,
            recurring_transaction_id: None,
            exclude_from_stats: true,
            exclude_from_budget: false,
            native_amount_cents: Some(12525),
            native_currency: Some(CurrencyCode::cny()),
            exchange_rate: Some("1".to_owned()),
        };
        let json = serde_json::to_value(&transaction).unwrap();
        assert_eq!(json["amountCents"], 12525);
        assert_eq!(json["excludeFromStats"], true);
        assert!(json.get("amount").is_none());
        assert_eq!(serde_json::from_value::<Transaction>(json).unwrap(), transaction);
    }
}
