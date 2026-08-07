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
         PRAGMA busy_timeout=5000;",
    )
}
