use rusqlite::Connection;

/// 执行 `PRAGMA integrity_check`，返回是否完整。
pub fn integrity_ok(connection: &Connection) -> Result<bool, rusqlite::Error> {
    let value: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    Ok(value == "ok")
}

/// 外键检查结果：(child_table, rowid, parent_table, fkid)
pub type ForeignKeyViolation = (String, i64, String, i64);

/// 执行 `PRAGMA foreign_key_check`，返回所有外键异常。
pub fn foreign_key_violations(
    connection: &Connection,
) -> Result<Vec<ForeignKeyViolation>, rusqlite::Error> {
    let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    rows.collect()
}

/// Migration 完成后的统一校验：完整性 + 外键。
pub fn validate(connection: &Connection) -> Result<(), String> {
    let ok = integrity_ok(connection).map_err(|error| format!("integrity_check 失败: {error}"))?;
    if !ok {
        return Err("PRAGMA integrity_check 未通过".to_owned());
    }
    let violations =
        foreign_key_violations(connection).map_err(|error| format!("foreign_key_check 失败: {error}"))?;
    if !violations.is_empty() {
        let first = violations.first().cloned().unwrap_or_default();
        return Err(format!(
            "外键异常 {} 条（示例: {}/{} → {}）",
            violations.len(),
            first.0,
            first.1,
            first.2
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_check_passes_for_empty_database() {
        let connection = Connection::open_in_memory().unwrap();
        assert!(integrity_ok(&connection).unwrap());
    }

    #[test]
    fn foreign_key_check_reports_violations() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE parent(id TEXT PRIMARY KEY);
                 CREATE TABLE child(parent_id TEXT REFERENCES parent(id));
                 PRAGMA foreign_keys=OFF;
                 INSERT INTO child(parent_id) VALUES('missing');
                 PRAGMA foreign_keys=ON;",
            )
            .unwrap();
        let violations = foreign_key_violations(&connection).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, "child");
    }
}
