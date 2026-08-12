//! 财务 Repository：真实列与前端 DTO 的转换与读写。
//!
//! 数据库内部金额永远使用整数分（`amount_cents` / `opening_balance_cents`），
//! 只有经过这里转换成 DTO 时才会除以 100 返回 `amount`。

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::database::legacy::json_parser;

pub const DEFAULT_USER_ID: &str = "local";
pub const CATEGORY_FALLBACK: &str = "未分类";
pub const CURRENCY_CNY: &str = "CNY";

const ACCOUNT_TYPES: [&str; 6] = ["cash", "bank", "wechat", "alipay", "investment", "other"];
const TRANSACTION_TYPES: [&str; 5] = ["expense", "income", "transfer", "refund", "fee"];

/// number → 分（要求有限数字；负数在交易场景由调用方拒绝）。
pub fn to_cents(value: f64) -> Result<i64, String> {
    if !value.is_finite() {
        return Err(format!("金额不是有限数字: {value}"));
    }
    Ok((value * 100.0).round() as i64)
}

/// 交易金额转换：number → 分，拒绝负数。
pub fn amount_to_cents(value: f64) -> Result<i64, String> {
    let cents = to_cents(value)?;
    if cents < 0 {
        return Err(format!("交易金额不能为负数: {value}"));
    }
    Ok(cents)
}

/// 分 → number（仅用于返回前端 DTO）。
pub fn cents_to_amount(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// 时间解析：支持 RFC3339 与 `YYYY-MM-DD`，统一输出 RFC3339。
pub fn normalize_occurred_at(value: &str) -> Result<String, String> {
    if let Ok(date_time) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(date_time.to_rfc3339());
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| format!("时间无法推导: {value}"))?;
        let utc: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::from_naive_utc_and_offset(naive, Utc);
        return Ok(utc.to_rfc3339());
    }
    Err(format!("时间格式无法解析: {value}"))
}

/// 从 RFC3339 / 日期推导业务自然日 `YYYY-MM-DD`。
pub fn local_date_of(value: &str) -> Result<String, String> {
    let normalized = normalize_occurred_at(value)?;
    normalized
        .get(0..10)
        .map(str::to_owned)
        .ok_or_else(|| format!("无法推导自然日: {value}"))
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn user_id(object: &serde_json::Map<String, Value>) -> String {
    json_parser::string_field(object, "userId")
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_USER_ID)
        .to_owned()
}

/// 规范化账户行。
#[derive(Debug, Clone)]
pub struct AccountRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub account_type: String,
    pub opening_balance_cents: Option<i64>,
    pub balance_at: Option<String>,
    pub last4: Option<String>,
    pub color: String,
    pub icon: String,
    pub is_archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}

/// 规范化交易行。
#[derive(Debug, Clone)]
pub struct TransactionRow {
    pub id: String,
    pub user_id: String,
    pub transaction_type: String,
    pub amount_cents: i64,
    pub currency: String,
    pub account_id: Option<String>,
    pub to_account_id: Option<String>,
    pub category_id: Option<String>,
    pub counterparty: Option<String>,
    pub merchant: Option<String>,
    pub item: Option<String>,
    pub note: Option<String>,
    pub occurred_at: String,
    pub local_date: String,
    pub status: String,
    pub source_type: String,
    pub external_transaction_id: Option<String>,
    pub legacy_category_name: Option<String>,
    pub legacy_account_name: Option<String>,
    pub raw_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: i64,
    pub modified_by_device: Option<String>,
}

fn optional_text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    json_parser::string_field(object, key)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// 旧 JSON 账户 → 规范化行。
pub fn account_from_legacy_json(value: &Value) -> Result<AccountRow, String> {
    let object = json_parser::as_object(value, "账户记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("账户缺少 id: {}", value))?;
    let account_type = json_parser::string_field(object, "type")
        .ok_or_else(|| format!("账户 {id} 缺少 type"))?
        .to_owned();
    if !ACCOUNT_TYPES.contains(&account_type.as_str()) {
        return Err(format!("账户 {id} 类型不合法: {account_type}"));
    }
    let name = json_parser::string_field(object, "name")
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("账户 {id} 缺少 name"))?
        .to_owned();
    let balance_cents = match json_parser::number_field(object, "balance") {
        Some(balance) => Some(to_cents(balance)?),
        None => None,
    };
    Ok(AccountRow {
        id: id.to_owned(),
        user_id: user_id(object),
        name,
        account_type,
        opening_balance_cents: balance_cents,
        balance_at: optional_text(object, "balanceAt"),
        last4: optional_text(object, "last4"),
        color: optional_text(object, "color").unwrap_or_else(|| "#5f7d70".to_owned()),
        icon: optional_text(object, "icon").unwrap_or_default(),
        is_archived: object
            .get("isArchived")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        created_at: optional_text(object, "createdAt").unwrap_or_else(now),
        updated_at: optional_text(object, "updatedAt").unwrap_or_else(now),
        deleted_at: None,
        version: 1,
        modified_by_device: None,
    })
}

fn resolve_account_id(
    connection: &Connection,
    user_id: &str,
    account_id: Option<&str>,
    account_name: Option<&str>,
) -> Option<String> {
    if let Some(id) = account_id.filter(|id| !id.is_empty()) {
        let exists: bool = connection
            .query_row(
                "SELECT 1 FROM finance_accounts WHERE id=?1 AND deleted_at IS NULL",
                [id],
                |_| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_some();
        if exists {
            return Some(id.to_owned());
        }
    }
    if let Some(name) = account_name.filter(|name| !name.is_empty()) {
        return connection
            .query_row(
                "SELECT id FROM finance_accounts
                 WHERE user_id=?1 AND name=?2 AND deleted_at IS NULL",
                params![user_id, name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten();
    }
    None
}

/// 查找或创建分类，返回稳定 id（基于类型 + 名称）。
pub fn find_or_create_category(
    connection: &Connection,
    user_id: &str,
    category_type: &str,
    name: &str,
) -> Result<String, String> {
    let normalized = if name.trim().is_empty() {
        CATEGORY_FALLBACK.to_owned()
    } else {
        name.trim().to_owned()
    };
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM transaction_categories
             WHERE user_id=?1 AND category_type=?2 AND name=?3 AND deleted_at IS NULL",
            params![user_id, category_type, normalized],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        return Ok(id);
    }
    let id = format!(
        "cat-{}-{}",
        category_type,
        &format!("{:x}", md5::compute(normalized.as_bytes()))[..16]
    );
    let stamp = now();
    connection
        .execute(
            "INSERT OR IGNORE INTO transaction_categories(
               id, user_id, name, category_type, parent_id, icon, color,
               is_system, is_archived, created_at, updated_at, deleted_at, version
             ) VALUES(?1,?2,?3,?4,NULL,NULL,NULL,0,0,?5,?6,NULL,1)",
            params![id, user_id, normalized, category_type, stamp, stamp],
        )
        .map_err(|error| error.to_string())?;
    Ok(id)
}

/// 旧 JSON 交易 → 规范化行；账户解析失败会记录 migration issue 但保留原数据。
pub fn transaction_from_legacy_json(
    connection: &Connection,
    value: &Value,
    context: Option<&crate::database::migration_runner::MigrationContext>,
    transaction: Option<&Transaction>,
) -> Result<TransactionRow, String> {
    let object = json_parser::as_object(value, "交易记录")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("交易缺少 id: {}", value))?;
    let transaction_type = json_parser::string_field(object, "type")
        .ok_or_else(|| format!("交易 {id} 缺少 type"))?
        .to_owned();
    if !TRANSACTION_TYPES.contains(&transaction_type.as_str()) {
        return Err(format!("交易 {id} 类型不合法: {transaction_type}"));
    }
    let amount = json_parser::number_field(object, "amount")
        .ok_or_else(|| format!("交易 {id} 缺少 amount"))?;
    let amount_cents = amount_to_cents(amount)?;
    let user = user_id(object);
    let account_id = resolve_account_id(
        connection,
        &user,
        json_parser::string_field(object, "accountId"),
        json_parser::string_field(object, "account"),
    );
    if account_id.is_none() {
        if let Some(account_name) = json_parser::string_field(object, "account") {
            let message = format!(
                "交易 {id} 未找到账户「{account_name}」，account_id 置空，原名称保留在 legacy_account_name"
            );
            if let (Some(transaction), Some(context)) = (transaction, context) {
                let _ = crate::database::migration_runner::record_issue(
                    transaction,
                    context,
                    "transactions",
                    Some(id),
                    "warning",
                    &message,
                    Some(&value.to_string()),
                );
            }
        }
    }
    let to_account_id = resolve_account_id(
        connection,
        &user,
        json_parser::string_field(object, "toAccountId"),
        json_parser::string_field(object, "toAccount"),
    );
    let category_name = json_parser::string_field(object, "category").unwrap_or_default();
    let category_id = find_or_create_category(connection, &user, &transaction_type, category_name)?;
    let occurred_at_raw = json_parser::string_field(object, "occurredAt")
        .or_else(|| json_parser::string_field(object, "createdAt"))
        .ok_or_else(|| format!("交易 {id} 缺少 occurredAt/createdAt"))?;
    let occurred_at = normalize_occurred_at(occurred_at_raw)?;
    let local_date = local_date_of(&occurred_at)?;
    Ok(TransactionRow {
        id: id.to_owned(),
        user_id: user,
        transaction_type,
        amount_cents,
        currency: CURRENCY_CNY.to_owned(),
        account_id,
        to_account_id,
        category_id: Some(category_id),
        counterparty: optional_text(object, "counterparty"),
        merchant: None,
        item: optional_text(object, "item"),
        note: optional_text(object, "note"),
        occurred_at,
        local_date,
        status: "confirmed".to_owned(),
        source_type: "manual".to_owned(),
        external_transaction_id: None,
        legacy_category_name: optional_text(object, "category"),
        legacy_account_name: optional_text(object, "account"),
        raw_json: Some(value.to_string()),
        created_at: optional_text(object, "createdAt").unwrap_or_else(now),
        updated_at: optional_text(object, "updatedAt").unwrap_or_else(now),
        deleted_at: None,
        version: 1,
        modified_by_device: None,
    })
}

/// 插入或更新账户（按 id upsert）。
pub fn upsert_account(connection: &Connection, row: &AccountRow) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO finance_accounts(
               id, user_id, name, account_type, opening_balance_cents, balance_at, last4,
               color, icon, is_archived, created_at, updated_at, deleted_at, version,
               modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
             ON CONFLICT(id) DO UPDATE SET
               user_id=excluded.user_id, name=excluded.name, account_type=excluded.account_type,
               opening_balance_cents=excluded.opening_balance_cents, balance_at=excluded.balance_at,
               last4=excluded.last4, color=excluded.color, icon=excluded.icon,
               is_archived=excluded.is_archived, updated_at=excluded.updated_at,
               deleted_at=excluded.deleted_at, version=excluded.version,
               modified_by_device=excluded.modified_by_device",
            params![
                row.id,
                row.user_id,
                row.name,
                row.account_type,
                row.opening_balance_cents,
                row.balance_at,
                row.last4,
                row.color,
                row.icon,
                row.is_archived,
                row.created_at,
                row.updated_at,
                row.deleted_at,
                row.version,
                row.modified_by_device
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// 插入或更新交易；`evidence_raw` 存在时同时写入一条 evidence。
pub fn upsert_transaction(
    connection: &Connection,
    row: &TransactionRow,
    evidence_raw: Option<&str>,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO transactions(
               id, user_id, transaction_type, amount_cents, currency, account_id, to_account_id,
               category_id, counterparty, merchant, item, note, occurred_at, local_date, status,
               source_type, external_transaction_id, legacy_category_name, legacy_account_name,
               raw_json, created_at, updated_at, deleted_at, version, modified_by_device
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)
             ON CONFLICT(id) DO UPDATE SET
               user_id=excluded.user_id, transaction_type=excluded.transaction_type,
               amount_cents=excluded.amount_cents, currency=excluded.currency,
               account_id=excluded.account_id, to_account_id=excluded.to_account_id,
               category_id=excluded.category_id, counterparty=excluded.counterparty,
               merchant=excluded.merchant, item=excluded.item, note=excluded.note,
               occurred_at=excluded.occurred_at, local_date=excluded.local_date,
               status=excluded.status, source_type=excluded.source_type,
               external_transaction_id=excluded.external_transaction_id,
               legacy_category_name=excluded.legacy_category_name,
               legacy_account_name=excluded.legacy_account_name,
               raw_json=excluded.raw_json, updated_at=excluded.updated_at,
               deleted_at=excluded.deleted_at, version=excluded.version,
               modified_by_device=excluded.modified_by_device",
            params![
                row.id,
                row.user_id,
                row.transaction_type,
                row.amount_cents,
                row.currency,
                row.account_id,
                row.to_account_id,
                row.category_id,
                row.counterparty,
                row.merchant,
                row.item,
                row.note,
                row.occurred_at,
                row.local_date,
                row.status,
                row.source_type,
                row.external_transaction_id,
                row.legacy_category_name,
                row.legacy_account_name,
                row.raw_json,
                row.created_at,
                row.updated_at,
                row.deleted_at,
                row.version,
                row.modified_by_device
            ],
        )
        .map_err(|error| error.to_string())?;
    if let Some(raw) = evidence_raw {
        connection
            .execute(
                "INSERT INTO transaction_evidence(id, transaction_id, source_type, source_id, external_transaction_id, confidence, raw_json, created_at)
                 VALUES(?1,?2,'legacy_json',?3,NULL,NULL,?4,?5)
                 ON CONFLICT(id) DO NOTHING",
                params![Uuid::new_v4().to_string(), row.id, row.id, raw, now()],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn account_to_dto(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let cents: Option<i64> = row.get(4)?;
    let balance_at: Option<String> = row.get(5)?;
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "userId": row.get::<_, String>(1)?,
        "name": row.get::<_, String>(2)?,
        "type": row.get::<_, String>(3)?,
        "balance": cents.map(cents_to_amount),
        "balanceAt": balance_at,
        "last4": row.get::<_, Option<String>>(6)?,
        "color": row.get::<_, String>(7)?,
        "icon": row.get::<_, String>(8)?,
        "isArchived": row.get::<_, bool>(9)?,
        "createdAt": row.get::<_, String>(10)?,
        "updatedAt": row.get::<_, String>(11)?
    }))
}

/// 账户列表（DTO，amount 已从分转回元）。
pub fn list_accounts(connection: &Connection) -> Result<Vec<Value>, String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, user_id, name, account_type, opening_balance_cents, balance_at, last4,
                    color, icon, is_archived, created_at, updated_at
             FROM finance_accounts
             WHERE deleted_at IS NULL AND user_id=?1
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([profile_id], account_to_dto)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn transaction_to_dto(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let amount_cents: i64 = row.get(3)?;
    let account_id: Option<String> = row.get(5)?;
    let to_account_id: Option<String> = row.get(6)?;
    let category_name: Option<String> = row.get(25)?;
    let account_name: Option<String> = row.get(26)?;
    let to_account_name: Option<String> = row.get(27)?;
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "userId": row.get::<_, String>(1)?,
        "type": row.get::<_, String>(2)?,
        "amount": cents_to_amount(amount_cents),
        "category": category_name,
        "categoryId": row.get::<_, Option<String>>(7)?,
        "account": account_name,
        "accountId": account_id,
        "toAccount": to_account_name,
        "toAccountId": to_account_id,
        "counterparty": row.get::<_, Option<String>>(8)?,
        "item": row.get::<_, Option<String>>(10)?,
        "note": row.get::<_, Option<String>>(11)?,
        "occurredAt": row.get::<_, String>(12)?,
        "createdAt": row.get::<_, String>(20)?,
        "updatedAt": row.get::<_, String>(21)?
    }))
}

/// 交易列表（DTO）。`category`/`account` 优先返回解析后的名称，
/// 缺失时回退到 legacy 名称，保证旧 UI 兼容。
pub fn list_transactions(connection: &Connection) -> Result<Vec<Value>, String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT t.id, t.user_id, t.transaction_type, t.amount_cents, t.currency,
                    t.account_id, t.to_account_id, t.category_id, t.counterparty,
                    t.merchant, t.item, t.note, t.occurred_at, t.local_date, t.status,
                    t.source_type, t.external_transaction_id, t.legacy_category_name,
                    t.legacy_account_name, t.raw_json, t.created_at, t.updated_at,
                    t.deleted_at, t.version, t.modified_by_device,
                    COALESCE(c.name, t.legacy_category_name) AS category_name,
                    COALESCE(a.name, t.legacy_account_name) AS account_name,
                    COALESCE(a2.name, t.legacy_account_name) AS to_account_name
             FROM transactions t
             LEFT JOIN transaction_categories c ON c.id = t.category_id
             LEFT JOIN finance_accounts a ON a.id = t.account_id
             LEFT JOIN finance_accounts a2 ON a2.id = t.to_account_id
             WHERE t.deleted_at IS NULL AND t.user_id=?1
             ORDER BY t.updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([profile_id], transaction_to_dto)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

/// 当前资料下可用的账单分类。分类是正式实体，交易仍同时返回名称以兼容旧界面。
pub fn list_categories(connection: &Connection) -> Result<Vec<Value>, String> {
    let profile_id = crate::database::profile::active_profile_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, user_id, name, category_type, parent_id, icon, color,
                    is_system, is_archived, created_at, updated_at
             FROM transaction_categories
             WHERE user_id=?1 AND deleted_at IS NULL AND is_archived=0
             ORDER BY category_type, is_system DESC, name",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([profile_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "userId": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "type": row.get::<_, String>(3)?,
                "parentId": row.get::<_, Option<String>>(4)?,
                "icon": row.get::<_, Option<String>>(5)?,
                "color": row.get::<_, Option<String>>(6)?,
                "isSystem": row.get::<_, bool>(7)?,
                "isArchived": row.get::<_, bool>(8)?,
                "createdAt": row.get::<_, String>(9)?,
                "updatedAt": row.get::<_, String>(10)?,
            }))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn save_category(connection: &Connection, dto: &Value) -> Result<(), String> {
    let owned = crate::database::profile::assign_active_owner(connection, dto)?;
    let object = json_parser::as_object(&owned, "分类数据")?;
    let user_id = user_id(object);
    let name = json_parser::string_field(object, "name")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "分类名称不能为空".to_owned())?;
    let category_type = json_parser::string_field(object, "type")
        .filter(|value| matches!(*value, "expense" | "income"))
        .ok_or_else(|| "分类类型仅支持 expense 或 income".to_owned())?;
    let id = json_parser::string_field(object, "id")
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    connection
        .execute(
            "INSERT INTO transaction_categories(
           id,user_id,name,category_type,parent_id,icon,color,is_system,is_archived,
           created_at,updated_at,deleted_at,version
         ) VALUES(?1,?2,?3,?4,NULL,?5,?6,0,0,?7,?7,NULL,1)
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, category_type=excluded.category_type,
           icon=excluded.icon, color=excluded.color, is_archived=0, deleted_at=NULL,
           updated_at=excluded.updated_at, version=transaction_categories.version+1",
            params![
                id,
                user_id,
                name,
                category_type,
                optional_text(object, "icon"),
                optional_text(object, "color"),
                stamp,
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn delete_category(connection: &Connection, id: &str) -> Result<(), String> {
    connection
        .execute(
            "UPDATE transaction_categories SET is_archived=1, updated_at=?1, version=version+1
         WHERE id=?2 AND is_system=0",
            params![now(), id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn existing_version(connection: &Connection, table: &str, id: &str) -> Result<i64, String> {
    let value: Option<i64> = connection
        .query_row(
            &format!("SELECT version FROM {table} WHERE id=?1"),
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(value.unwrap_or(0))
}

/// 保存账户 DTO（写路径）。
pub fn save_account(connection: &Connection, dto: &Value) -> Result<(), String> {
    let owned = crate::database::profile::assign_active_owner(connection, dto)?;
    let dto = &owned;
    let object = json_parser::as_object(dto, "账户数据")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut row = account_from_legacy_json(dto)?;
    row.id = id.clone();
    row.version = existing_version(connection, "finance_accounts", &id)? + 1;
    row.updated_at = optional_text(object, "updatedAt").unwrap_or_else(now);
    upsert_account(connection, &row)
}

/// 保存交易 DTO（写路径）。
pub fn save_transaction(connection: &Connection, dto: &Value) -> Result<(), String> {
    let owned = crate::database::profile::assign_active_owner(connection, dto)?;
    let dto = &owned;
    let object = json_parser::as_object(dto, "交易数据")?;
    let id = json_parser::string_field(object, "id")
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut row = transaction_from_legacy_json(connection, dto, None, None)?;
    row.id = id.clone();
    row.version = existing_version(connection, "transactions", &id)? + 1;
    row.updated_at = optional_text(object, "updatedAt").unwrap_or_else(now);
    upsert_transaction(connection, &row, None)
}

/// 软删除账户。
pub fn delete_account(connection: &Connection, id: &str) -> Result<(), String> {
    connection
        .execute(
            "UPDATE finance_accounts SET deleted_at=?1, updated_at=?1, version=version+1 WHERE id=?2",
            params![now(), id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// 软删除交易。
pub fn delete_transaction(connection: &Connection, id: &str) -> Result<(), String> {
    connection
        .execute(
            "UPDATE transactions SET deleted_at=?1, updated_at=?1, version=version+1 WHERE id=?2",
            params![now(), id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// 恢复：事务内清空财务数据并从 DTO 数组重建。
pub fn replace_all(
    transaction: &Transaction,
    accounts: &[Value],
    transactions: &[Value],
) -> Result<(), String> {
    transaction
        .execute("DELETE FROM transaction_evidence", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM transactions", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM finance_accounts", [])
        .map_err(|error| error.to_string())?;
    for account in accounts {
        let owned = crate::database::profile::assign_active_owner(transaction, account)?;
        let row = account_from_legacy_json(&owned)?;
        upsert_account(transaction, &row)?;
    }
    for item in transactions {
        let owned = crate::database::profile::assign_active_owner(transaction, item)?;
        let row = transaction_from_legacy_json(transaction, &owned, None, None)?;
        upsert_transaction(transaction, &row, None)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_conversion_roundtrip() {
        assert_eq!(amount_to_cents(32.5).unwrap(), 3250);
        assert_eq!(amount_to_cents(0.01).unwrap(), 1);
        assert_eq!(amount_to_cents(10000.0).unwrap(), 1_000_000);
        assert_eq!(cents_to_amount(3250), 32.5);
        assert!(amount_to_cents(-1.0).is_err());
        assert!(amount_to_cents(f64::NAN).is_err());
        assert!(amount_to_cents(f64::INFINITY).is_err());
        assert_eq!(to_cents(-0.5).unwrap(), -50);
    }

    #[test]
    fn time_parsing_and_local_date() {
        assert_eq!(
            normalize_occurred_at("2026-07-23T09:00:24.000Z").unwrap(),
            "2026-07-23T09:00:24+00:00"
        );
        assert_eq!(
            normalize_occurred_at("2026-07-23").unwrap(),
            "2026-07-23T00:00:00+00:00"
        );
        assert!(normalize_occurred_at("not-a-date").is_err());
        assert_eq!(local_date_of("2026-07-23T09:00:24Z").unwrap(), "2026-07-23");
    }

    #[test]
    fn legacy_json_account_to_row() {
        let value = json!({
            "id": "wechat-wallet", "userId": "local-user", "name": "微信零钱",
            "type": "wechat", "balance": 128.31, "last4": "", "color": "#2a9c69",
            "icon": "微", "isArchived": false,
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
        });
        let row = account_from_legacy_json(&value).unwrap();
        assert_eq!(row.opening_balance_cents, Some(12831));
        assert_eq!(row.account_type, "wechat");
    }

    #[test]
    fn invalid_account_type_is_rejected() {
        let value = json!({"id": "x", "name": "x", "type": "crypto", "balance": 1});
        assert!(account_from_legacy_json(&value).is_err());
    }

    #[test]
    fn legacy_json_transaction_requires_amount() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE transaction_categories(
                   id TEXT PRIMARY KEY, user_id TEXT, name TEXT, category_type TEXT,
                   is_archived INTEGER, is_system INTEGER, created_at TEXT, updated_at TEXT,
                   deleted_at TEXT, version INTEGER
                 );
                 CREATE TABLE finance_accounts(
                   id TEXT PRIMARY KEY, name TEXT, deleted_at TEXT
                 );",
            )
            .unwrap();
        let value =
            json!({"id": "t1", "userId": "local-user", "type": "expense", "category": "餐饮"});
        assert!(transaction_from_legacy_json(&connection, &value, None, None).is_err());
    }
}
