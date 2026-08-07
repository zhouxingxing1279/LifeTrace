use std::path::Path;

use rusqlite::Connection;

/// 旧 D1 / wrangler 数据库导入入口。
///
/// 阶段 1 先复用现有 `server::migration::migrate_once` 实现，保证行为不变；
/// 阶段 7 将把实现收敛到本模块并补齐缺失表。
pub fn import_once(connection: &mut Connection, data_dir: &Path) -> Result<usize, String> {
    crate::server::migration::migrate_once(connection, data_dir)
}
