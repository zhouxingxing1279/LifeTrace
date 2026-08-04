use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde_json::Value;

use crate::database::legacy::json_parser;
use crate::database::migration_runner::{Migration, MigrationContext, MigrationError, MigrationReport};
use crate::database::repositories::finance;

const LEGACY_ACCOUNTS_TABLE: &str = "legacy_finance_accounts_json_v1";
const LEGACY_TRANSACTIONS_TABLE: &str = "legacy_transactions_json_v1";

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
}

/// m0002：财务 schema 规范化。
///
/// 旧 JSON 表重命名为 `legacy_*_json_v1`（保留数据），创建真实列表，
/// 迁移账户/分类/交易/证据，并校验数量与金额（差值为 0 分）。
pub struct M0002Finance;

impl Migration for M0002Finance {
    fn version(&self) -> i64 {
        2
    }

    fn name(&self) -> &'static str {
        "finance-normalization"
    }

    fn checksum(&self) -> &'static str {
        "m0002-finance-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        rename_legacy_tables(transaction)?;
        create_normalized_tables(transaction)?;

        let legacy_accounts = if table_exists(transaction, LEGACY_ACCOUNTS_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_ACCOUNTS_TABLE)?
        } else {
            Vec::new()
        };
        let legacy_transactions = if table_exists(transaction, LEGACY_TRANSACTIONS_TABLE) {
            json_parser::read_json_rows(transaction, LEGACY_TRANSACTIONS_TABLE)?
        } else {
            Vec::new()
        };

        let mut account_count = 0usize;
        for value in &legacy_accounts {
            let row = finance::account_from_legacy_json(value)?;
            finance::upsert_account(transaction, &row)?;
            account_count += 1;
        }

        // 分类：按 (交易类型, 分类名) 去重，保留 legacy_category_name。
        let mut category_entries = BTreeMap::<(String, String), String>::new();
        for value in &legacy_transactions {
            let object = json_parser::as_object(value, "交易记录")?;
            let id = json_parser::string_field(object, "id").unwrap_or_default();
            let transaction_type = json_parser::string_field(object, "type")
                .ok_or_else(|| format!("交易 {id} 缺少 type"))?;
            let category = json_parser::string_field(object, "category").unwrap_or_default();
            let user = json_parser::string_field(object, "userId")
                .filter(|value| !value.is_empty())
                .unwrap_or(finance::DEFAULT_USER_ID);
            let category_id =
                finance::find_or_create_category(transaction, user, transaction_type, category)?;
            category_entries.insert(
                (transaction_type.to_owned(), category.to_owned()),
                category_id,
            );
        }

        let mut transaction_count = 0usize;
        for value in &legacy_transactions {
            let row =
                finance::transaction_from_legacy_json(transaction, value, Some(context), Some(transaction))?;
            finance::upsert_transaction(transaction, &row, Some(&value.to_string()))?;
            transaction_count += 1;
        }

        validate_finance(transaction, &legacy_accounts, &legacy_transactions)?;

        let mut report = MigrationReport::default();
        report.migrated = account_count + category_entries.len() + transaction_count;
        report
            .metrics
            .insert("accounts".to_owned(), account_count as i64);
        report
            .metrics
            .insert("categories".to_owned(), category_entries.len() as i64);
        report
            .metrics
            .insert("transactions".to_owned(), transaction_count as i64);
        report.metrics.insert("evidence".to_owned(), transaction_count as i64);
        Ok(report)
    }
}

fn rename_legacy_tables(connection: &Connection) -> Result<(), MigrationError> {
    if table_exists(connection, "finance_accounts") && !table_exists(connection, LEGACY_ACCOUNTS_TABLE)
    {
        connection
            .execute(&format!("ALTER TABLE finance_accounts RENAME TO {LEGACY_ACCOUNTS_TABLE}"), [])
            .map_err(|error| MigrationError {
                version: 2,
                message: format!("重命名 finance_accounts 失败: {error}"),
            })?;
    }
    if table_exists(connection, "transactions") && !table_exists(connection, LEGACY_TRANSACTIONS_TABLE)
    {
        connection
            .execute(
                &format!("ALTER TABLE transactions RENAME TO {LEGACY_TRANSACTIONS_TABLE}"),
                [],
            )
            .map_err(|error| MigrationError {
                version: 2,
                message: format!("重命名 transactions 失败: {error}"),
            })?;
    }
    Ok(())
}

fn create_normalized_tables(connection: &Connection) -> Result<(), MigrationError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS finance_accounts (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               name TEXT NOT NULL,
               account_type TEXT NOT NULL CHECK (account_type IN (
                 'cash','bank','wechat','alipay','investment','other'
               )),
               opening_balance_cents INTEGER,
               balance_at TEXT,
               last4 TEXT,
               color TEXT NOT NULL DEFAULT '#5f7d70',
               icon TEXT NOT NULL DEFAULT '',
               is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT
             );
             CREATE TABLE IF NOT EXISTS transaction_categories (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               name TEXT NOT NULL,
               category_type TEXT NOT NULL CHECK (category_type IN (
                 'expense','income','transfer'
               )),
               parent_id TEXT,
               icon TEXT,
               color TEXT,
               is_system INTEGER NOT NULL DEFAULT 0,
               is_archived INTEGER NOT NULL DEFAULT 0,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT,
               FOREIGN KEY (parent_id) REFERENCES transaction_categories(id)
             );
             CREATE UNIQUE INDEX IF NOT EXISTS uq_transaction_categories_name
               ON transaction_categories(user_id, category_type, name)
               WHERE deleted_at IS NULL;
             CREATE TABLE IF NOT EXISTS transactions (
               id TEXT PRIMARY KEY,
               user_id TEXT NOT NULL DEFAULT 'local',
               transaction_type TEXT NOT NULL CHECK (transaction_type IN (
                 'expense','income','transfer','refund','fee'
               )),
               amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0),
               currency TEXT NOT NULL DEFAULT 'CNY',
               account_id TEXT,
               to_account_id TEXT,
               category_id TEXT,
               counterparty TEXT,
               merchant TEXT,
               item TEXT,
               note TEXT,
               occurred_at TEXT NOT NULL,
               local_date TEXT NOT NULL,
               status TEXT NOT NULL DEFAULT 'confirmed' CHECK (status IN (
                 'candidate','provisional','confirmed','ignored'
               )),
               source_type TEXT NOT NULL DEFAULT 'manual',
               external_transaction_id TEXT,
               legacy_category_name TEXT,
               legacy_account_name TEXT,
               raw_json TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               deleted_at TEXT,
               version INTEGER NOT NULL DEFAULT 1,
               modified_by_device TEXT,
               FOREIGN KEY (account_id) REFERENCES finance_accounts(id),
               FOREIGN KEY (to_account_id) REFERENCES finance_accounts(id),
               FOREIGN KEY (category_id) REFERENCES transaction_categories(id)
             );
             CREATE INDEX IF NOT EXISTS idx_transactions_date
               ON transactions(user_id, local_date, deleted_at);
             CREATE INDEX IF NOT EXISTS idx_transactions_account
               ON transactions(user_id, account_id, occurred_at);
             CREATE INDEX IF NOT EXISTS idx_transactions_category
               ON transactions(user_id, category_id, occurred_at);
             CREATE UNIQUE INDEX IF NOT EXISTS uq_transactions_external
               ON transactions(user_id, source_type, external_transaction_id)
               WHERE external_transaction_id IS NOT NULL AND deleted_at IS NULL;
             CREATE TABLE IF NOT EXISTS transaction_evidence (
               id TEXT PRIMARY KEY,
               transaction_id TEXT NOT NULL,
               source_type TEXT NOT NULL,
               source_id TEXT,
               external_transaction_id TEXT,
               confidence REAL,
               raw_json TEXT,
               created_at TEXT NOT NULL,
               FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_transaction_evidence_transaction
               ON transaction_evidence(transaction_id);",
        )
        .map_err(|error| MigrationError {
            version: 2,
            message: format!("创建财务规范化表失败: {error}"),
        })
}

#[derive(Debug, Default, Clone)]
struct FinanceTotals {
    expense_cents: i64,
    income_cents: i64,
    transfer_cents: i64,
    monthly: BTreeMap<String, (i64, i64, i64)>,
}

fn totals_from_legacy(values: &[Value]) -> Result<FinanceTotals, String> {
    let mut totals = FinanceTotals::default();
    for value in values {
        let object = json_parser::as_object(value, "交易记录")?;
        let transaction_type = json_parser::string_field(object, "type").unwrap_or_default();
        let amount = json_parser::number_field(object, "amount")
            .ok_or_else(|| "交易缺少 amount".to_owned())?;
        let cents = finance::amount_to_cents(amount)?;
        let occurred_at = json_parser::string_field(object, "occurredAt")
            .or_else(|| json_parser::string_field(object, "createdAt"))
            .ok_or_else(|| "交易缺少时间".to_owned())?;
        let month = finance::local_date_of(occurred_at)?.get(0..7).unwrap_or_default().to_owned();
        match transaction_type {
            "expense" => {
                totals.expense_cents += cents;
                totals.monthly.entry(month).or_default().0 += cents;
            }
            "income" => {
                totals.income_cents += cents;
                totals.monthly.entry(month).or_default().1 += cents;
            }
            "transfer" => {
                totals.transfer_cents += cents;
                totals.monthly.entry(month).or_default().2 += cents;
            }
            other => return Err(format!("未知交易类型: {other}")),
        }
    }
    Ok(totals)
}

fn totals_from_database(connection: &Connection) -> Result<FinanceTotals, String> {
    let mut totals = FinanceTotals::default();
    let mut statement = connection
        .prepare(
            "SELECT transaction_type, SUM(amount_cents), substr(local_date,1,7)
             FROM transactions WHERE deleted_at IS NULL
             GROUP BY transaction_type, substr(local_date,1,7)",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (transaction_type, cents, month) = row.map_err(|error| error.to_string())?;
        let entry = totals.monthly.entry(month).or_default();
        match transaction_type.as_str() {
            "expense" => {
                totals.expense_cents += cents;
                entry.0 += cents;
            }
            "income" => {
                totals.income_cents += cents;
                entry.1 += cents;
            }
            "transfer" => {
                totals.transfer_cents += cents;
                entry.2 += cents;
            }
            "refund" | "fee" => {
                totals.expense_cents += cents;
                entry.0 += cents;
            }
            _ => {}
        }
    }
    Ok(totals)
}

fn validate_finance(
    connection: &Connection,
    legacy_accounts: &[Value],
    legacy_transactions: &[Value],
) -> Result<(), MigrationError> {
    let legacy_totals = totals_from_legacy(legacy_transactions).map_err(|message| MigrationError {
        version: 2,
        message,
    })?;
    let new_totals = totals_from_database(connection).map_err(|message| MigrationError {
        version: 2,
        message,
    })?;

    let new_transactions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|error| MigrationError { version: 2, message: error.to_string() })?;
    let new_accounts: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM finance_accounts WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|error| MigrationError { version: 2, message: error.to_string() })?;
    let new_evidence: i64 = connection
        .query_row("SELECT COUNT(*) FROM transaction_evidence", [], |row| row.get(0))
        .map_err(|error| MigrationError { version: 2, message: error.to_string() })?;

    if new_transactions != legacy_transactions.len() as i64 {
        return Err(MigrationError {
            version: 2,
            message: format!(
                "交易数量不一致: 旧 {} 条，新 {new_transactions} 条",
                legacy_transactions.len()
            ),
        });
    }
    if new_accounts != legacy_accounts.len() as i64 {
        return Err(MigrationError {
            version: 2,
            message: format!(
                "账户数量不一致: 旧 {} 条，新 {new_accounts} 条",
                legacy_accounts.len()
            ),
        });
    }
    if new_evidence != new_transactions {
        return Err(MigrationError {
            version: 2,
            message: format!("证据数量不一致: 交易 {new_transactions} 条，证据 {new_evidence} 条"),
        });
    }
    if legacy_totals.expense_cents != new_totals.expense_cents
        || legacy_totals.income_cents != new_totals.income_cents
        || legacy_totals.transfer_cents != new_totals.transfer_cents
    {
        return Err(MigrationError {
            version: 2,
            message: format!(
                "金额汇总不一致: 旧(支出/收入/转账)=({}/{}/{}), 新=({}/{}/{})",
                legacy_totals.expense_cents,
                legacy_totals.income_cents,
                legacy_totals.transfer_cents,
                new_totals.expense_cents,
                new_totals.income_cents,
                new_totals.transfer_cents
            ),
        });
    }
    for (month, legacy_entry) in &legacy_totals.monthly {
        let new_entry = new_totals.monthly.get(month).copied().unwrap_or_default();
        if *legacy_entry != new_entry {
            return Err(MigrationError {
                version: 2,
                message: format!("月份 {month} 汇总不一致: 旧 {legacy_entry:?}，新 {new_entry:?}"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::{M0001Framework, all};
    use crate::database::migration_runner::run;
    use rusqlite::Connection;
    use rusqlite::params;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-finance-{label}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn seed_legacy_json(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE finance_accounts(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE transactions(
                   id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
                 );",
            )
            .unwrap();
        let account = json!({
            "id": "wechat-wallet", "userId": "local-user", "name": "微信零钱",
            "type": "wechat", "balance": 128.31, "last4": "", "color": "#2a9c69",
            "icon": "微", "isArchived": false,
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        });
        connection
            .execute(
                "INSERT INTO finance_accounts VALUES('wechat-wallet', ?1, '2026-01-01T00:00:00Z')",
                params![account.to_string()],
            )
            .unwrap();
        let transactions = vec![
            json!({
                "id": "t1", "userId": "local-user", "type": "expense", "amount": 32.5,
                "category": "餐饮", "account": "微信零钱", "accountId": "wechat-wallet",
                "occurredAt": "2026-07-01T09:00:00Z", "createdAt": "2026-07-01T09:00:00Z",
                "updatedAt": "2026-07-01T09:00:00Z"
            }),
            json!({
                "id": "t2", "userId": "local-user", "type": "income", "amount": 100.0,
                "category": "工资", "account": "微信零钱", "accountId": "wechat-wallet",
                "occurredAt": "2026-07-02T09:00:00Z", "createdAt": "2026-07-02T09:00:00Z",
                "updatedAt": "2026-07-02T09:00:00Z"
            }),
            json!({
                "id": "t3", "userId": "local-user", "type": "transfer", "amount": 50.0,
                "category": "", "account": "微信零钱", "accountId": "wechat-wallet",
                "toAccount": "银行卡", "occurredAt": "2026-07-03T09:00:00Z",
                "createdAt": "2026-07-03T09:00:00Z", "updatedAt": "2026-07-03T09:00:00Z"
            }),
        ];
        for item in &transactions {
            connection
                .execute(
                    "INSERT INTO transactions VALUES(?1, ?2, '2026-07-01T00:00:00Z')",
                    params![item["id"].as_str().unwrap(), item.to_string()],
                )
                .unwrap();
        }
    }

    #[test]
    fn migrates_legacy_json_finance() {
        let directory = temp_dir("migrate");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        seed_legacy_json(&connection);
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> =
            vec![Box::new(M0001Framework), Box::new(M0002Finance)];
        let summary = run(&mut connection, &context, &migrations).unwrap();
        assert_eq!(summary.applied.len(), 2);

        let accounts = finance::list_accounts(&connection).unwrap();
        let transactions = finance::list_transactions(&connection).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0]["balance"], json!(128.31));
        assert_eq!(transactions.len(), 3);
        let t1 = transactions
            .iter()
            .find(|item| item.get("id").and_then(Value::as_str) == Some("t1"))
            .expect("t1 should exist");
        assert_eq!(t1["amount"], json!(32.5));
        assert_eq!(t1["type"], json!("expense"));
        assert_eq!(t1["category"], json!("餐饮"));
        assert_eq!(t1["accountId"], json!("wechat-wallet"));

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM transaction_evidence", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);
        let categories: i64 = connection
            .query_row("SELECT COUNT(*) FROM transaction_categories", [], |row| row.get(0))
            .unwrap();
        assert_eq!(categories, 3); // 餐饮/工资/未分类
        // 旧表已重命名保留。
        let legacy: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='legacy_transactions_json_v1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy, 1);

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn illegal_amount_fails_and_rolls_back() {
        let directory = temp_dir("bad-amount");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        seed_legacy_json(&connection);
        connection
            .execute(
                "INSERT INTO transactions VALUES('bad', ?1, '2026-07-01T00:00:00Z')",
                params![json!({
                    "id": "bad", "userId": "local-user", "type": "expense",
                    "amount": "not-a-number", "category": "餐饮", "account": "微信零钱",
                    "accountId": "wechat-wallet", "occurredAt": "2026-07-01T09:00:00Z",
                    "createdAt": "2026-07-01T09:00:00Z", "updatedAt": "2026-07-01T09:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> =
            vec![Box::new(M0001Framework), Box::new(M0002Finance)];
        let result = run(&mut connection, &context, &migrations);
        assert!(result.is_err());
        // 回滚后旧表仍在，schema_migrations 无 v2。
        let legacy: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='transactions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy, 1);
        let versions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version=2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(versions, 0);
        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn orphan_account_keeps_legacy_name_and_records_issue() {
        let directory = temp_dir("orphan");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        seed_legacy_json(&connection);
        connection
            .execute(
                "INSERT INTO transactions VALUES('t4', ?1, '2026-07-01T00:00:00Z')",
                params![json!({
                    "id": "t4", "userId": "local-user", "type": "expense", "amount": 9.9,
                    "category": "购物", "account": "不存在的账户",
                    "occurredAt": "2026-07-04T09:00:00Z", "createdAt": "2026-07-04T09:00:00Z",
                    "updatedAt": "2026-07-04T09:00:00Z"
                })
                .to_string()],
            )
            .unwrap();
        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> =
            vec![Box::new(M0001Framework), Box::new(M0002Finance)];
        run(&mut connection, &context, &migrations).unwrap();
        let account_id: Option<String> = connection
            .query_row(
                "SELECT account_id FROM transactions WHERE id='t4'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(account_id, None);
        let legacy_name: String = connection
            .query_row(
                "SELECT legacy_account_name FROM transactions WHERE id='t4'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_name, "不存在的账户");
        let issues: i64 = connection
            .query_row("SELECT COUNT(*) FROM migration_issues", [], |row| row.get(0))
            .unwrap();
        assert_eq!(issues, 1);
        fs::remove_dir_all(&directory).ok();
    }

    /// 真实旧库升级演练：通过环境变量 `LIFETRACE_FIXTURE_DB` 指定旧 SQLite 文件，
    /// 复制到临时目录后执行完整 Migration，并与旧 JSON 的 json_extract 汇总核对。
    #[test]
    #[ignore = "需要真实旧库文件（LIFETRACE_FIXTURE_DB）"]
    fn migrates_real_legacy_backup() {
        let fixture = std::env::var("LIFETRACE_FIXTURE_DB")
            .expect("LIFETRACE_FIXTURE_DB 必须指向旧 SQLite 文件");
        let directory = temp_dir("real-backup");
        let target = directory.join("fixture.db");
        fs::copy(&fixture, &target).expect("复制 fixture 失败");
        let mut connection = Connection::open(&target).unwrap();

        let legacy_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='transactions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_count, 1, "fixture 必须包含旧 JSON transactions 表");

        let context = crate::database::migration_runner::MigrationContext::new(directory.clone());
        let migrations = all();
        run(&mut connection, &context, &migrations).expect("真实旧库迁移失败");

        // 新表汇总（分）。
        let new_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let new_expense: i64 = connection
            .query_row(
                "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE transaction_type='expense' AND deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let new_income: i64 = connection
            .query_row(
                "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE transaction_type='income' AND deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // 旧 JSON 汇总（分）。
        let legacy_sums: (i64, i64) = connection
            .query_row(
                "SELECT
                   COALESCE(SUM(CASE WHEN json_extract(data_json,'$.type')='expense'
                       THEN CAST(ROUND(json_extract(data_json,'$.amount')*100) AS INTEGER) END),0),
                   COALESCE(SUM(CASE WHEN json_extract(data_json,'$.type')='income'
                       THEN CAST(ROUND(json_extract(data_json,'$.amount')*100) AS INTEGER) END),0)
                 FROM legacy_transactions_json_v1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .unwrap();
        let legacy_count_total: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM legacy_transactions_json_v1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(new_count, legacy_count_total, "交易数量不一致");
        assert_eq!(new_expense, legacy_sums.0, "支出金额不一致（分）");
        assert_eq!(new_income, legacy_sums.1, "收入金额不一致（分）");
        let activities = crate::database::repositories::habits::list_activities(&connection).unwrap();
        let logs = crate::database::repositories::habits::list_activity_logs(&connection).unwrap();
        let reviews =
            crate::database::repositories::habits::list_daily_reviews(&connection).unwrap();
        eprintln!(
            "真实旧库演练通过: 交易 {new_count} 条，支出 {new_expense} 分，收入 {new_income} 分，\
             习惯 {} 条，打卡 {} 条，复盘 {} 条",
            activities.len(),
            logs.len(),
            reviews.len()
        );

        fs::remove_dir_all(&directory).ok();
    }
}
