use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use uuid::Uuid;

use crate::database::{backup, validation};

/// Migration 执行上下文：数据目录、备份目录、当前运行记录 id。
#[derive(Debug, Clone)]
pub struct MigrationContext {
    pub data_dir: PathBuf,
    pub run_id: Option<String>,
}

impl MigrationContext {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            run_id: None,
        }
    }

    pub fn with_run(&self, run_id: String) -> Self {
        Self {
            run_id: Some(run_id),
            ..self.clone()
        }
    }
}

/// Migration 完成后返回的统计报告。
#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    pub migrated: usize,
    pub warnings: usize,
    pub errors: usize,
    pub metrics: BTreeMap<String, i64>,
}

/// 迁移错误：包含版本与可读信息。
#[derive(Debug)]
pub struct MigrationError {
    pub version: i64,
    pub message: String,
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Migration v{} 失败: {}",
            self.version, self.message
        )
    }
}

impl std::error::Error for MigrationError {}

impl From<rusqlite::Error> for MigrationError {
    fn from(error: rusqlite::Error) -> Self {
        Self {
            version: 0,
            message: error.to_string(),
        }
    }
}

impl From<String> for MigrationError {
    fn from(message: String) -> Self {
        Self {
            version: 0,
            message,
        }
    }
}

/// 版本化 Migration 接口。
///
/// `up()` 在 Runner 开启的 `BEGIN IMMEDIATE` 事务内执行，失败整体回滚。
pub trait Migration: Send + Sync {
    fn version(&self) -> i64;
    fn name(&self) -> &'static str;
    fn checksum(&self) -> &'static str;
    fn up(
        &self,
        transaction: &Transaction,
        context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError>;
}

/// 一次成功应用的 Migration。
#[derive(Debug, Clone)]
pub struct AppliedMigration {
    pub version: i64,
    pub name: String,
    pub report: MigrationReport,
}

/// 一次 `run()` 的整体结果。
#[derive(Debug, Default)]
pub struct MigrationSummary {
    pub applied: Vec<AppliedMigration>,
    pub skipped: usize,
}

/// 防止多进程同时执行 Migration 的文件锁。
struct MigrationLock {
    path: PathBuf,
}

impl MigrationLock {
    fn acquire(data_dir: &Path) -> Result<Self, MigrationError> {
        let path = data_dir.join("lifetrace.migration.lock");
        let mut attempt = 0;
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let stale = fs::metadata(&path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                        .is_some_and(|modified| {
                            modified.elapsed().is_ok_and(|age| age.as_secs() > 600)
                        });
                    if stale && attempt == 0 {
                        let _ = fs::remove_file(&path);
                        attempt += 1;
                        continue;
                    }
                    return Err(MigrationError {
                        version: 0,
                        message: "另一个迁移进程正在运行（或存在残留锁文件）".to_owned(),
                    });
                }
                Err(error) => {
                    return Err(MigrationError {
                        version: 0,
                        message: format!("创建迁移锁失败: {error}"),
                    })
                }
            }
        }
    }
}

impl Drop for MigrationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 创建迁移元数据表（幂等，Runner 引导阶段调用）。
pub fn bootstrap(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           version INTEGER PRIMARY KEY,
           name TEXT NOT NULL,
           checksum TEXT NOT NULL,
           applied_at TEXT NOT NULL,
           app_version TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS migration_runs (
           id TEXT PRIMARY KEY,
           from_version INTEGER NOT NULL,
           to_version INTEGER NOT NULL,
           status TEXT NOT NULL CHECK (status IN ('running','succeeded','failed')),
           backup_path TEXT,
           started_at TEXT NOT NULL,
           finished_at TEXT,
           error_message TEXT
         );
         CREATE TABLE IF NOT EXISTS migration_issues (
           id TEXT PRIMARY KEY,
           migration_run_id TEXT NOT NULL,
           entity_type TEXT NOT NULL,
           entity_id TEXT,
           severity TEXT NOT NULL CHECK (severity IN ('warning','error')),
           message TEXT NOT NULL,
           raw_json TEXT,
           created_at TEXT NOT NULL,
           FOREIGN KEY (migration_run_id) REFERENCES migration_runs(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_migration_issues_run
           ON migration_issues(migration_run_id);",
    )
}

fn current_version(connection: &Connection) -> rusqlite::Result<i64> {
    connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )
}

fn recorded_migration(
    connection: &Connection,
    version: i64,
) -> rusqlite::Result<Option<(String, String)>> {
    connection
        .query_row(
            "SELECT name, checksum FROM schema_migrations WHERE version=?1",
            [version],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
}

/// 在迁移事务内记录一条 migration issue（warning/error）。
pub fn record_issue(
    transaction: &Transaction,
    context: &MigrationContext,
    entity_type: &str,
    entity_id: Option<&str>,
    severity: &str,
    message: &str,
    raw_json: Option<&str>,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO migration_issues(
           id, migration_run_id, entity_type, entity_id, severity, message, raw_json, created_at
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            Uuid::new_v4().to_string(),
            context.run_id.as_deref().unwrap_or(""),
            entity_type,
            entity_id,
            severity,
            message,
            raw_json,
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

/// 按版本顺序执行未应用的 Migration。
///
/// 流程：备份 → 写 running 记录 → BEGIN IMMEDIATE → up() → 校验 → 写 schema_migrations
/// → COMMIT → 标记 succeeded。任一步失败则 ROLLBACK 并标记 failed，保留备份。
pub fn run(
    connection: &mut Connection,
    context: &MigrationContext,
    migrations: &[Box<dyn Migration>],
) -> Result<MigrationSummary, MigrationError> {
    bootstrap(connection).map_err(|error| MigrationError {
        version: 0,
        message: format!("初始化迁移元数据表失败: {error}"),
    })?;
    let _lock = MigrationLock::acquire(&context.data_dir)?;

    let mut pending: Vec<&Box<dyn Migration>> = migrations.iter().collect();
    pending.sort_by_key(|migration| migration.version());
    for pair in pending.windows(2) {
        if pair[0].version() == pair[1].version() {
            return Err(MigrationError {
                version: pair[0].version(),
                message: format!(
                    "重复的 Migration 版本: {}（{} 与 {}）",
                    pair[0].version(),
                    pair[0].name(),
                    pair[1].name()
                ),
            });
        }
    }

    let mut current = current_version(connection).map_err(|error| MigrationError {
        version: 0,
        message: format!("读取当前 Migration 版本失败: {error}"),
    })?;
    let mut summary = MigrationSummary::default();

    // 先校验全部已记录版本的 checksum，防止代码回改已应用 Migration。
    for migration in &pending {
        if let Some((name, checksum)) = recorded_migration(connection, migration.version())
            .map_err(|error| MigrationError {
                version: migration.version(),
                message: format!("读取已记录 Migration 失败: {error}"),
            })?
        {
            if checksum != migration.checksum() {
                return Err(MigrationError {
                    version: migration.version(),
                    message: format!(
                        "checksum 不匹配: 已记录 {name}/{checksum}，期望 {}",
                        migration.checksum()
                    ),
                });
            }
        }
    }

    for migration in pending {
        if migration.version() <= current {
            summary.skipped += 1;
            continue;
        }

        let backup_record = backup::create_backup(
            connection,
            &context.data_dir,
            &format!("before-schema-v{}", migration.version()),
        )
        .map_err(|message| MigrationError {
            version: migration.version(),
            message,
        })?;
        if !backup_record.integrity_ok {
            return Err(MigrationError {
                version: migration.version(),
                message: format!("迁移前备份完整性校验失败: {}", backup_record.path.display()),
            });
        }
        eprintln!(
            "LifeTrace 迁移前备份: {}（{} 字节，sha256 {}）",
            backup_record.path.display(),
            backup_record.size_bytes,
            backup_record.sha256
        );
        let run_id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO migration_runs(id, from_version, to_version, status, backup_path, started_at)
                 VALUES(?1,?2,?3,'running',?4,?5)",
                params![
                    run_id,
                    current,
                    migration.version(),
                    backup_record.path.display().to_string(),
                    started_at
                ],
            )
            .map_err(|error| MigrationError {
                version: migration.version(),
                message: format!("写入迁移运行记录失败: {error}"),
            })?;

        let run_context = context.with_run(run_id.clone());
        let run_result: Result<MigrationReport, MigrationError> = (|| {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| MigrationError {
                    version: migration.version(),
                    message: format!("开启迁移事务失败: {error}"),
                })?;
            let mut report = migration.up(&transaction, &run_context)?;
            report.warnings = transaction
                .query_row(
                    "SELECT COUNT(*) FROM migration_issues
                     WHERE migration_run_id=?1 AND severity='warning'",
                    [&run_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0) as usize;
            report.errors = transaction
                .query_row(
                    "SELECT COUNT(*) FROM migration_issues
                     WHERE migration_run_id=?1 AND severity='error'",
                    [&run_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0) as usize;
            validation::validate(&transaction).map_err(|message| MigrationError {
                version: migration.version(),
                message,
            })?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations(version, name, checksum, applied_at, app_version)
                     VALUES(?1,?2,?3,?4,?5)",
                    params![
                        migration.version(),
                        migration.name(),
                        migration.checksum(),
                        Utc::now().to_rfc3339(),
                        env!("CARGO_PKG_VERSION")
                    ],
                )
                .map_err(|error| MigrationError {
                    version: migration.version(),
                    message: format!("写入 schema_migrations 失败: {error}"),
                })?;
            transaction.commit().map_err(|error| MigrationError {
                version: migration.version(),
                message: format!("提交迁移失败: {error}"),
            })?;
            Ok(report)
        })();

        match run_result {
            Ok(report) => {
                connection
                    .execute(
                        "UPDATE migration_runs SET status='succeeded', finished_at=?1 WHERE id=?2",
                        params![Utc::now().to_rfc3339(), run_id],
                    )
                    .map_err(|error| MigrationError {
                        version: migration.version(),
                        message: format!("标记迁移成功失败: {error}"),
                    })?;
                // 保留最近 3 份备份；清理失败不阻断。
                let _ = backup::cleanup_old_backups(&context.data_dir, 3);
                summary.applied.push(AppliedMigration {
                    version: migration.version(),
                    name: migration.name().to_owned(),
                    report,
                });
                current = migration.version();
            }
            Err(error) => {
                let _ = connection.execute(
                    "UPDATE migration_runs SET status='failed', finished_at=?1, error_message=?2 WHERE id=?3",
                    params![Utc::now().to_rfc3339(), error.message.clone(), run_id],
                );
                return Err(error);
            }
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestMigration {
        version: i64,
        name: &'static str,
        checksum: &'static str,
        fail: bool,
    }

    impl Migration for TestMigration {
        fn version(&self) -> i64 {
            self.version
        }
        fn name(&self) -> &'static str {
            self.name
        }
        fn checksum(&self) -> &'static str {
            self.checksum
        }
        fn up(
            &self,
            transaction: &Transaction,
            _context: &MigrationContext,
        ) -> Result<MigrationReport, MigrationError> {
            transaction.execute(
                "CREATE TABLE IF NOT EXISTS scratch(id TEXT PRIMARY KEY, value TEXT)",
                [],
            )?;
            transaction.execute("INSERT INTO scratch(id, value) VALUES('x', 'y')", [])?;
            if self.fail {
                return Err(MigrationError {
                    version: self.version,
                    message: "注入的失败".to_owned(),
                });
            }
            let mut report = MigrationReport::default();
            report.migrated = 1;
            Ok(report)
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-runner-{label}-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn run_is_idempotent() {
        let directory = temp_dir("idempotent");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        let context = MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![Box::new(TestMigration {
            version: 1,
            name: "test",
            checksum: "abc",
            fail: false,
        })];

        let first = run(&mut connection, &context, &migrations).unwrap();
        assert_eq!(first.applied.len(), 1);
        let second = run(&mut connection, &context, &migrations).unwrap();
        assert_eq!(second.applied.len(), 0);
        assert_eq!(second.skipped, 1);
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn failed_migration_rolls_back_and_keeps_backup() {
        let directory = temp_dir("rollback");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        let context = MigrationContext::new(directory.clone());
        let migrations: Vec<Box<dyn Migration>> = vec![Box::new(TestMigration {
            version: 1,
            name: "test-failure",
            checksum: "def",
            fail: true,
        })];

        let result = run(&mut connection, &context, &migrations);
        assert!(result.is_err());
        // 事务已回滚：scratch 表不存在。
        let scratch_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='scratch'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(scratch_count, 0);
        // 不写入成功版本。
        let versions: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(versions, 0);
        // 运行记录标记为 failed。
        let status: String = connection
            .query_row("SELECT status FROM migration_runs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status, "failed");
        // 备份保留。
        let backup_count = fs::read_dir(backup::backup_directory(&directory))
            .unwrap()
            .count();
        assert_eq!(backup_count, 1);

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn checksum_mismatch_is_rejected() {
        let directory = temp_dir("checksum");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        let context = MigrationContext::new(directory.clone());
        bootstrap(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations(version, name, checksum, applied_at, app_version)
                 VALUES(1, 'test', 'old-checksum', '2026-01-01T00:00:00Z', '0.2.1')",
                [],
            )
            .unwrap();
        let migrations: Vec<Box<dyn Migration>> = vec![Box::new(TestMigration {
            version: 1,
            name: "test",
            checksum: "new-checksum",
            fail: false,
        })];
        let result = run(&mut connection, &context, &migrations);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("checksum"));

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn lock_prevents_duplicate_concurrent_runs() {
        let directory = temp_dir("lock");
        let lock = MigrationLock::acquire(&directory).unwrap();
        let second = MigrationLock::acquire(&directory);
        assert!(second.is_err());
        drop(lock);
        assert!(MigrationLock::acquire(&directory).is_ok());
        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn record_issue_writes_row() {
        let directory = temp_dir("issue");
        let mut connection = Connection::open(directory.join("test.db")).unwrap();
        let context = MigrationContext::new(directory.clone());
        bootstrap(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO migration_runs(id, from_version, to_version, status, started_at)
                 VALUES('run-1', 0, 1, 'running', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        let tx = connection.transaction().unwrap();
        record_issue(
            &tx,
            &context.with_run("run-1".to_owned()),
            "transactions",
            Some("id-1"),
            "warning",
            "测试 issue",
            Some(r#"{"amount":1}"#),
        )
        .unwrap();
        tx.commit().unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM migration_issues", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        fs::remove_dir_all(&directory).ok();
    }
}
