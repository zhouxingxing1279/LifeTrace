//! 旧 D1 训练 JSON 数据导入规范化表。
use crate::database::repositories::workouts;
use rusqlite::Connection;

/// 从旧 JSON 表导入训练相关实体。
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
            "workouts" => workouts::save_workout(&transaction, value)?,
            "workout_imports" => workouts::save_import(&transaction, value)?,
            "training_notes" => workouts::save_training_note(&transaction, value)?,
            _ => continue,
        }
        imported += 1;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(imported)
}
