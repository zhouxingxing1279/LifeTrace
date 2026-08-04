use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::database::validation;

/// 一次备份的记录信息。
#[derive(Debug, Clone)]
pub struct BackupRecord {
    pub path: PathBuf,
    pub sha256: String,
    pub size_bytes: u64,
    pub integrity_ok: bool,
}

/// 备份目录：`{data_dir}/backups/database/`
pub fn backup_directory(data_dir: &Path) -> PathBuf {
    data_dir.join("backups").join("database")
}

/// 使用 SQLite Backup API 创建一致性备份。
///
/// 禁止在 WAL 模式下直接复制主 .db 文件；备份完成后立即执行
/// `PRAGMA integrity_check` 并计算 SHA-256。
pub fn create_backup(
    connection: &Connection,
    data_dir: &Path,
    label: &str,
) -> Result<BackupRecord, String> {
    let directory = backup_directory(data_dir);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("创建备份目录失败: {error}"))?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S%.3f");
    let path = directory.join(format!("lifetrace-{label}-{stamp}.db"));

    let mut destination = Connection::open(&path)
        .map_err(|error| format!("打开备份目标失败: {error}"))?;
    let mut backup = rusqlite::backup::Backup::new(connection, &mut destination)
        .map_err(|error| format!("创建备份失败: {error}"))?;
    let progress: Option<fn(rusqlite::backup::Progress)> = None;
    backup
        .run_to_completion(32, Duration::from_millis(50), progress)
        .map_err(|error| format!("备份执行失败: {error}"))?;
    drop(backup);
    drop(destination);

    let integrity_ok = verify_backup(&path)?;
    let bytes = fs::read(&path).map_err(|error| format!("读取备份文件失败: {error}"))?;
    let size_bytes = bytes.len() as u64;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(BackupRecord {
        path,
        sha256,
        size_bytes,
        integrity_ok,
    })
}

/// 只读打开备份并执行 `PRAGMA integrity_check`。
pub fn verify_backup(path: &Path) -> Result<bool, String> {
    let connection = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("打开备份失败: {error}"))?;
    validation::integrity_ok(&connection).map_err(|error| error.to_string())
}

/// 保留最近 `keep` 个迁移前备份；清理失败不影响启动。
///
/// 约定：文件名为 `lifetrace-before-schema-v*.db`，按时间戳倒序保留。
pub fn cleanup_old_backups(data_dir: &Path, keep: usize) -> usize {
    let directory = backup_directory(data_dir);
    let Ok(entries) = fs::read_dir(&directory) else {
        return 0;
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("lifetrace-before-schema-v") && name.ends_with(".db") {
            files.push((name, entry.path()));
        }
    }
    files.sort_by(|left, right| right.0.cmp(&left.0));
    let mut removed = 0;
    for (_, path) in files.into_iter().skip(keep.max(1)) {
        if fs::remove_file(path).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn backup_is_openable_and_passes_integrity() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-backup-test-{unique}"));
        fs::create_dir_all(&directory).unwrap();
        let connection = Connection::open(directory.join("source.db")).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sample(id TEXT PRIMARY KEY, value TEXT);
                 INSERT INTO sample VALUES('a', 'hello');",
            )
            .unwrap();

        let record = create_backup(&connection, &directory, "before-schema-v1").unwrap();
        assert!(record.integrity_ok);
        assert!(record.path.exists());
        assert!(record.size_bytes > 0);
        assert_eq!(record.sha256.len(), 64);

        let reopened = Connection::open(&record.path).unwrap();
        let value: String = reopened
            .query_row("SELECT value FROM sample WHERE id='a'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "hello");

        fs::remove_dir_all(&directory).ok();
    }
}
