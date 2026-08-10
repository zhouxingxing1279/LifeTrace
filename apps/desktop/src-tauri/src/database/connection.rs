use std::path::Path;

use rusqlite::Connection;

/// 打开数据库并应用统一 PRAGMA 配置。
///
/// 连接配置只有这一处，业务模块不得自行打开或配置主连接。
pub fn open(database_path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(database_path)?;
    configure(&connection)?;
    Ok(connection)
}

/// 对已有连接应用统一 PRAGMA 配置。
pub fn configure(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA secure_delete=ON;
         PRAGMA trusted_schema=OFF;
         PRAGMA temp_store=MEMORY;",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_pragmas_are_enabled() {
        let connection = Connection::open_in_memory().unwrap();
        configure(&connection).unwrap();
        let secure_delete: i64 = connection
            .query_row("PRAGMA secure_delete", [], |row| row.get(0))
            .unwrap();
        let trusted_schema: i64 = connection
            .query_row("PRAGMA trusted_schema", [], |row| row.get(0))
            .unwrap();
        let temp_store: i64 = connection
            .query_row("PRAGMA temp_store", [], |row| row.get(0))
            .unwrap();
        assert_ne!(secure_delete, 0);
        assert_eq!(trusted_schema, 0);
        assert_eq!(temp_store, 2);
    }
}
