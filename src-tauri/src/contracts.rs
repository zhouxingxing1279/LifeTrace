//! EPIC-02 desktop contract adapter (example).
//!
//! Converts the SQLite/Domain transaction representation to the public
//! contract `finance.transaction` DTO and then to a `SyncChangeV1` wire
//! change. This is contract adaptation only: there is no network sync, no
//! outbox and no background worker here.
//!
//! Layer separation (per EPIC-02 ADR-001):
//! SQLite Row (`TransactionRow`) -> Domain/Contract DTO (`Transaction`)
//!                              -> Sync wire (`SyncChangeV1`) -> JSON.

use lifetrace_contracts::domain::{Transaction, TransactionStatus, TransactionType};
use lifetrace_contracts::ids::{ChangeId, DeviceId, EntityId, ServerVersion, UserId};
use lifetrace_contracts::sync::v1::{ChangeOperation, SyncChangeV1};
use lifetrace_contracts::{
    CurrencyCode, EntityMeta, EntityType, JsonValue, LocalDate, UtcTimestamp,
};

use crate::database::repositories::finance::TransactionRow;

fn parse_timestamp(value: &str) -> Result<UtcTimestamp, String> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&chrono::Utc))
        .map_err(|error| format!("invalid RFC3339 timestamp `{value}`: {error}"))
}

/// SQLite/Domain `TransactionRow` -> public contract `Transaction`.
///
/// `amount_cents` is preserved exactly. The EPIC-01 local `version` column
/// becomes `meta.localVersion` and MUST NOT be presented as a server version;
/// `meta.serverVersion` stays `None` until the server assigns one.
pub fn transaction_row_to_contract(row: &TransactionRow) -> Result<Transaction, String> {
    let created_at = parse_timestamp(&row.created_at)?;
    let updated_at = parse_timestamp(&row.updated_at)?;
    let occurred_at = parse_timestamp(&row.occurred_at)?;
    let deleted_at = row
        .deleted_at
        .as_deref()
        .map(parse_timestamp)
        .transpose()?;

    Ok(Transaction {
        meta: EntityMeta {
            id: EntityId::new(&row.id),
            user_id: UserId::new(&row.user_id),
            created_at,
            updated_at,
            deleted_at,
            local_version: row.version.max(1) as u64,
            server_version: None,
            modified_by_device: row.modified_by_device.as_deref().map(DeviceId::new),
        },
        transaction_type: TransactionType::new(&row.transaction_type),
        amount_cents: row.amount_cents,
        currency: CurrencyCode::new(&row.currency).unwrap_or_else(|_| CurrencyCode::cny()),
        account_id: row.account_id.as_deref().map(EntityId::new),
        to_account_id: row.to_account_id.as_deref().map(EntityId::new),
        category_id: row.category_id.as_deref().map(EntityId::new),
        counterparty: row.counterparty.clone(),
        merchant: row.merchant.clone(),
        item: row.item.clone(),
        note: row.note.clone(),
        occurred_at,
        local_date: LocalDate::new(&row.local_date)?,
        status: TransactionStatus::new(&row.status),
        source_type: row.source_type.clone(),
        external_transaction_id: row.external_transaction_id.clone(),
    })
}

/// Contract `Transaction` -> `SyncChangeV1` upsert (full snapshot payload).
pub fn transaction_to_change(
    transaction: &Transaction,
    change_id: ChangeId,
    base_server_version: ServerVersion,
) -> SyncChangeV1 {
    let payload: JsonValue = serde_json::to_value(transaction)
        .expect("contract Transaction must serialize")
        .into();
    SyncChangeV1 {
        change_id,
        entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
        entity_id: transaction.meta.id.clone(),
        operation: ChangeOperation::new(ChangeOperation::UPSERT),
        base_server_version,
        entity_schema_version: 1,
        client_modified_at: transaction.meta.updated_at,
        payload: Some(payload),
        atomic_group_id: None,
        dependencies: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{MigrationContext, run};
    use crate::database::migrations::all;
    use lifetrace_contracts::ids::UserId;
    use rusqlite::Connection;
    use serde_json::json;

    fn in_memory_db_with_schema() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
            .unwrap();
        let context = MigrationContext::new(std::env::temp_dir());
        run(&mut connection, &context, &all()).expect("migrations should apply");
        connection
    }

    fn read_transaction_row(connection: &Connection, id: &str) -> TransactionRow {
        connection
            .query_row(
                "SELECT id, user_id, transaction_type, amount_cents, currency, account_id,
                        to_account_id, category_id, counterparty, merchant, item, note,
                        occurred_at, local_date, status, source_type, external_transaction_id,
                        legacy_category_name, legacy_account_name, raw_json, created_at,
                        updated_at, deleted_at, version, modified_by_device
                 FROM transactions WHERE id = ?1",
                [id],
                |row| {
                    Ok(TransactionRow {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        transaction_type: row.get(2)?,
                        amount_cents: row.get(3)?,
                        currency: row.get(4)?,
                        account_id: row.get(5)?,
                        to_account_id: row.get(6)?,
                        category_id: row.get(7)?,
                        counterparty: row.get(8)?,
                        merchant: row.get(9)?,
                        item: row.get(10)?,
                        note: row.get(11)?,
                        occurred_at: row.get(12)?,
                        local_date: row.get(13)?,
                        status: row.get(14)?,
                        source_type: row.get(15)?,
                        external_transaction_id: row.get(16)?,
                        legacy_category_name: row.get(17)?,
                        legacy_account_name: row.get(18)?,
                        raw_json: row.get(19)?,
                        created_at: row.get(20)?,
                        updated_at: row.get(21)?,
                        deleted_at: row.get(22)?,
                        version: row.get(23)?,
                        modified_by_device: row.get(24)?,
                    })
                },
            )
            .unwrap()
    }

    #[test]
    fn transaction_db_row_to_contract_to_sync_change_json_round_trips() {
        let connection = in_memory_db_with_schema();
        let legacy = json!({
            "id": "tx-adapter-1",
            "userId": "local-user",
            "type": "expense",
            "amount": 125.25,
            "category": "餐饮",
            "account": "微信零钱",
            "accountId": "wechat-wallet",
            "counterparty": "coffee shop",
            "item": "latte",
            "occurredAt": "2026-08-04T09:00:24Z",
            "createdAt": "2026-08-04T09:00:00Z",
            "updatedAt": "2026-08-04T09:05:00Z"
        });
        crate::database::repositories::finance::save_transaction(&connection, &legacy).unwrap();

        // SQLite row -> contract DTO.
        let row = read_transaction_row(&connection, "tx-adapter-1");
        assert_eq!(row.amount_cents, 12525, "amount is stored as integer cents");
        let contract = transaction_row_to_contract(&row).unwrap();
        assert_eq!(contract.amount_cents, 12525);
        assert_eq!(contract.currency.as_str(), "CNY");
        assert_eq!(contract.local_date.as_str(), "2026-08-04");
        assert_eq!(contract.meta.local_version, 1);
        assert_eq!(contract.meta.server_version, None, "local version is not a server version");

        // Contract DTO -> SyncChangeV1 -> JSON -> deserialize.
        let change = transaction_to_change(
            &contract,
            ChangeId::new("change-adapter-1"),
            ServerVersion::zero(),
        );
        let wire = serde_json::to_value(&change).unwrap();
        assert_eq!(wire["entityType"], "finance.transaction");
        assert_eq!(wire["operation"], "upsert");
        assert_eq!(wire["baseServerVersion"], "0");
        assert!(wire["payload"].get("amount").is_none(), "no float amount on the wire");
        assert_eq!(wire["payload"]["amountCents"], 12525);

        let back: SyncChangeV1 = serde_json::from_value(wire).unwrap();
        let payload: Transaction = serde_json::from_value(back.payload.unwrap().0).unwrap();
        assert_eq!(payload.meta.id, contract.meta.id);
        assert_eq!(payload.amount_cents, contract.amount_cents);
        assert_eq!(payload.currency, contract.currency);
        assert_eq!(payload.transaction_type, contract.transaction_type);
        assert_eq!(payload.occurred_at, contract.occurred_at);
        assert_eq!(payload.local_date, contract.local_date);
        assert_eq!(payload.status, contract.status);
        assert_eq!(payload.account_id, contract.account_id);
        assert_eq!(payload.counterparty, contract.counterparty);
        assert_eq!(payload.meta.local_version, contract.meta.local_version);
        assert_eq!(payload.meta.server_version, contract.meta.server_version);
    }

    #[test]
    fn contract_entity_ids_preserve_legacy_non_uuid_ids() {
        let mut transaction = transaction_row_to_contract(&TransactionRow {
            id: "wechat-wallet-tx-1".to_owned(),
            user_id: "local".to_owned(),
            transaction_type: "expense".to_owned(),
            amount_cents: 100,
            currency: "CNY".to_owned(),
            account_id: Some("wechat-wallet".to_owned()),
            to_account_id: None,
            category_id: None,
            counterparty: None,
            merchant: None,
            item: None,
            note: None,
            occurred_at: "2026-08-04T09:00:24Z".to_owned(),
            local_date: "2026-08-04".to_owned(),
            status: "confirmed".to_owned(),
            source_type: "manual".to_owned(),
            external_transaction_id: None,
            legacy_category_name: None,
            legacy_account_name: None,
            raw_json: None,
            created_at: "2026-08-04T09:00:00Z".to_owned(),
            updated_at: "2026-08-04T09:05:00Z".to_owned(),
            deleted_at: None,
            version: 1,
            modified_by_device: None,
        })
        .unwrap();
        assert_eq!(transaction.meta.id.as_str(), "wechat-wallet-tx-1");
        assert_eq!(transaction.meta.user_id.as_str(), "local");
        transaction.meta.user_id = UserId::new("local-user");
        let wire = serde_json::to_value(&transaction).unwrap();
        assert_eq!(wire["meta"]["id"], "wechat-wallet-tx-1");
        assert_eq!(wire["meta"]["userId"], "local-user");
    }
}
