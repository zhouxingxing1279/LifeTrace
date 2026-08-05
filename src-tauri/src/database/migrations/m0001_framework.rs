use rusqlite::Transaction;

use crate::database::migration_runner::{
    Migration, MigrationContext, MigrationError, MigrationReport,
};

/// m0001：Migration 与备份框架落表。
///
/// 将旧 `app_meta` 收归版本化管理，并写入框架标记。此版本不迁移任何业务表。
pub struct M0001Framework;

impl Migration for M0001Framework {
    fn version(&self) -> i64 {
        1
    }

    fn name(&self) -> &'static str {
        "framework"
    }

    fn checksum(&self) -> &'static str {
        "m0001-framework-v1"
    }

    fn up(
        &self,
        transaction: &Transaction,
        _context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError> {
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS app_meta(key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )?;
        transaction.execute(
            "INSERT OR REPLACE INTO app_meta(key, value) VALUES('schema_framework', 'versioned')",
            [],
        )?;
        let mut report = MigrationReport::default();
        report.migrated = 1;
        report.metrics.insert("app_meta_ready".to_owned(), 1);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn framework_migration_is_repeatable() {
        let mut connection = Connection::open_in_memory().unwrap();
        let context = MigrationContext::new(std::env::temp_dir());
        let first = connection.transaction().unwrap();
        let report = M0001Framework
            .up(&first, &context)
            .expect("framework migration should succeed");
        assert_eq!(report.migrated, 1);
        first.commit().unwrap();
        let second = connection.transaction().unwrap();
        M0001Framework
            .up(&second, &context)
            .expect("framework migration should be repeatable");
        second.commit().unwrap();
    }
}
