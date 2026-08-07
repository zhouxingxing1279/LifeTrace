//! 旧 D1 习惯/复盘 JSON 数据直接导入规范化表。

use rusqlite::Connection;

use crate::database::repositories::habits;

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
            "activities" => {
                let row = habits::activity_from_legacy_json(value)?;
                habits::upsert_activity(&transaction, &row)?;
            }
            "activity_logs" => {
                let row = habits::activity_log_from_legacy_json(&transaction, value, None, None)?;
                habits::upsert_activity_log(&transaction, &row)?;
            }
            "daily_reviews" => {
                let row = habits::daily_review_from_legacy_json(value, None, None)?;
                habits::upsert_daily_review(&transaction, &row)?;
            }
            _ => continue,
        }
        imported += 1;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(imported)
}
