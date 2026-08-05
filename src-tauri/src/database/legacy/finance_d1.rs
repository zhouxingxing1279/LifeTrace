//! 旧 D1 财务 JSON 数据直接导入规范化表。
//!
//! 当目标 `finance_accounts` / `transactions` 已是真实列表（无 `data_json` 列）时，
//! 旧 `migration.rs` 的 JSON 复制不再适用，改由这里复用 Repository 转换逻辑导入。

use rusqlite::Connection;

use crate::database::repositories::finance;

/// 从旧 JSON 表读取并写入规范化表，返回导入条数。
pub fn import_json_table(
    source: &Connection,
    destination: &mut Connection,
    source_table: &str,
    destination_table: &str,
) -> Result<usize, String> {
    let rows = super::json_parser::read_json_rows(source, source_table)?;
    let transaction = destination
        .transaction()
        .map_err(|error| error.to_string())?;
    let mut imported = 0usize;
    for value in &rows {
        match destination_table {
            "finance_accounts" => {
                let row = finance::account_from_legacy_json(value)?;
                finance::upsert_account(&transaction, &row)?;
            }
            "transactions" => {
                let row = finance::transaction_from_legacy_json(&transaction, value, None, None)?;
                finance::upsert_transaction(&transaction, &row, None)?;
            }
            _ => continue,
        }
        imported += 1;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, Migration, MigrationContext};
    use crate::database::migrations::{M0001Framework, M0002Finance};
    use rusqlite::params;
    use rusqlite::Connection;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-d1-{label}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn d1_finance_imports_into_normalized_tables() {
        let directory = temp_dir("finance");
        let mut destination = Connection::open(directory.join("dest.db")).unwrap();
        let context = MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> =
            vec![Box::new(M0001Framework), Box::new(M0002Finance)];
        run(&mut destination, &context, &migrations).unwrap();

        // 构造旧 D1 源库（JSON 表）。
        let source = Connection::open(directory.join("source.db")).unwrap();
        source
            .execute_batch(
                "CREATE TABLE finance_accounts(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE transactions(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );",
            )
            .unwrap();
        source
            .execute(
                "INSERT INTO finance_accounts VALUES('a1', ?1, '2026-01-01T00:00:00Z')",
                params![json!({
                    "id": "a1", "userId": "local-user", "name": "现金", "type": "cash",
                    "balance": 100, "color": "#fff", "icon": "cash",
                    "isArchived": false, "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-01T00:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
        source
            .execute(
                "INSERT INTO transactions VALUES('t1', ?1, '2026-01-02T00:00:00Z')",
                params![json!({
                    "id": "t1", "userId": "local-user", "type": "expense", "amount": 12.34,
                    "category": "交通", "account": "现金", "accountId": "a1",
                    "occurredAt": "2026-01-02T08:00:00Z", "createdAt": "2026-01-02T08:00:00Z",
                    "updatedAt": "2026-01-02T08:00:00Z"
                })
                .to_string()],
            )
            .unwrap();

        let imported = import_json_table(
            &source,
            &mut destination,
            "finance_accounts",
            "finance_accounts",
        )
        .unwrap();
        assert_eq!(imported, 1);
        let imported =
            import_json_table(&source, &mut destination, "transactions", "transactions").unwrap();
        assert_eq!(imported, 1);

        let cents: i64 = destination
            .query_row(
                "SELECT amount_cents FROM transactions WHERE id='t1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cents, 1234);
        let account_id: Option<String> = destination
            .query_row(
                "SELECT account_id FROM transactions WHERE id='t1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(account_id.as_deref(), Some("a1"));

        fs::remove_dir_all(&directory).ok();
    }
}
