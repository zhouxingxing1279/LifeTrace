//! Finance domain DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::domain::enums::{AccountType, TransactionStatus, TransactionType};
use crate::ids::EntityId;
use crate::money::CurrencyCode;
use crate::time::{LocalDate, UtcTimestamp};

/// `finance.account`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceAccount {
    pub meta: EntityMeta,
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
}

/// `finance.category`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TransactionCategory {
    pub meta: EntityMeta,
    pub name: String,
    pub category_type: TransactionType,
    pub parent_id: Option<EntityId>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub is_system: bool,
    pub is_archived: bool,
}

/// `finance.transaction`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Transaction {
    pub meta: EntityMeta,
    pub transaction_type: TransactionType,
    /// Amount in cents. Never floats on the wire.
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

/// One bill candidate extracted from a payment screenshot.
///
/// Capture results intentionally contain human-readable account/category hints
/// instead of durable entity IDs. Every client resolves those hints against its
/// own synchronized finance data before creating a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceCaptureBill {
    pub amount_cents: i64,
    pub currency: CurrencyCode,
    pub transaction_type: TransactionType,
    pub merchant: Option<String>,
    pub item: Option<String>,
    pub occurred_at: Option<UtcTimestamp>,
    pub account_hint: Option<String>,
    pub category_hint: Option<String>,
    pub external_transaction_id: Option<String>,
    pub confidence: Option<f64>,
}

/// Response returned by the command-style image capture API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FinanceCaptureResponse {
    pub provider: String,
    pub model: String,
    pub bills: Vec<FinanceCaptureBill>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{EntityId, UserId};

    fn stamp() -> UtcTimestamp {
        "2026-08-04T15:30:00Z".parse().unwrap()
    }

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
    fn transaction_wire_uses_amount_cents_and_camel_case() {
        let transaction = Transaction {
            meta: meta("tx-1"),
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
        };
        let json = serde_json::to_value(&transaction).unwrap();
        assert_eq!(json["amountCents"], 12525);
        assert_eq!(json["currency"], "CNY");
        assert_eq!(json["transactionType"], "expense");
        assert_eq!(json["localDate"], "2026-08-04");
        assert!(json.get("amount").is_none(), "no float amount on the wire");
        let back: Transaction = serde_json::from_value(json).unwrap();
        assert_eq!(back, transaction);
    }

    #[test]
    fn finance_capture_wire_is_stable_and_uses_hints() {
        let response = FinanceCaptureResponse {
            provider: "zhipu".to_owned(),
            model: "glm-4v-flash".to_owned(),
            bills: vec![FinanceCaptureBill {
                amount_cents: 2850,
                currency: CurrencyCode::cny(),
                transaction_type: TransactionType::new(TransactionType::EXPENSE),
                merchant: Some("瑞幸咖啡".to_owned()),
                item: None,
                occurred_at: Some(stamp()),
                account_hint: Some("招商银行储蓄卡".to_owned()),
                category_hint: Some("餐饮".to_owned()),
                external_transaction_id: None,
                confidence: Some(0.92),
            }],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["bills"][0]["amountCents"], 2850);
        assert_eq!(json["bills"][0]["transactionType"], "expense");
        assert_eq!(json["bills"][0]["accountHint"], "招商银行储蓄卡");
        assert!(json["bills"][0].get("accountId").is_none());
    }
}
