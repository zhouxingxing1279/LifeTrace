use rusqlite::Connection;
use serde_json::{Map, Value};

/// 读取一张 JSON 实体表的全部记录（`id, data_json, updated_at`）。
pub fn read_json_rows(connection: &Connection, table: &str) -> Result<Vec<Value>, String> {
    let mut statement = connection
        .prepare(&format!("SELECT id, data_json, updated_at FROM {table}"))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        let (id, raw, updated_at) = row.map_err(|error| error.to_string())?;
        let mut value: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("表 {table} 记录 {id} 不是合法 JSON: {error}"))?;
        if let Some(object) = value.as_object_mut() {
            object
                .entry("id".to_owned())
                .or_insert_with(|| Value::String(id));
            object
                .entry("updatedAt".to_owned())
                .or_insert_with(|| Value::String(updated_at));
        }
        result.push(value);
    }
    Ok(result)
}

/// 把 JSON 值作为对象返回，带可读错误。
pub fn as_object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} 不是 JSON 对象"))
}

/// 获取字符串字段（可能缺失）。
pub fn string_field<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(Value::as_str)
}

/// 获取整数/浮点字段。
pub fn number_field(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(Value::as_f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn parses_json_rows_and_fills_missing_ids() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sample(id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL);
                 INSERT INTO sample VALUES('a', '{\"name\":\"x\"}', '2026-01-01T00:00:00Z');",
            )
            .unwrap();
        let rows = read_json_rows(&connection, "sample").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], "a");
        assert_eq!(rows[0]["name"], "x");
        assert_eq!(rows[0]["updatedAt"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn invalid_json_returns_error() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sample(id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL);
                 INSERT INTO sample VALUES('bad', '{oops', '2026-01-01T00:00:00Z');",
            )
            .unwrap();
        assert!(read_json_rows(&connection, "sample").is_err());
    }
}
