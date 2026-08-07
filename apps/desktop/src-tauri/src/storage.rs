use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use rusqlite::{backup::Backup, params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const CONFIG_FILE: &str = "storage-location.json";
const MARKER_FILE: &str = ".lifetrace-storage.json";
const DATABASE_FILE: &str = "lifetrace.db";
const LARGE_FILE_THRESHOLD: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingMigration {
    source: PathBuf,
    target: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StorageConfig {
    active_data_dir: Option<PathBuf>,
    pending_migration: Option<PendingMigration>,
    cleanup_pending: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    current_path: String,
    default_path: String,
    target_path: Option<String>,
    phase: String,
    files_total: u64,
    files_copied: u64,
    bytes_total: u64,
    bytes_copied: u64,
    progress: f64,
    restart_required: bool,
    error: Option<String>,
}

impl StorageStatus {
    fn idle(current: &Path, default: &Path) -> Self {
        Self {
            current_path: current.display().to_string(),
            default_path: default.display().to_string(),
            target_path: None,
            phase: "idle".to_owned(),
            files_total: 0,
            files_copied: 0,
            bytes_total: 0,
            bytes_copied: 0,
            progress: 0.0,
            restart_required: false,
            error: None,
        }
    }
}

#[derive(Clone)]
pub struct StorageState {
    current_data_dir: PathBuf,
    default_data_dir: PathBuf,
    config_path: PathBuf,
    status: Arc<Mutex<StorageStatus>>,
}

impl StorageState {
    pub fn new(current_data_dir: PathBuf, default_data_dir: PathBuf, config_path: PathBuf) -> Self {
        Self {
            status: Arc::new(Mutex::new(StorageStatus::idle(
                &current_data_dir,
                &default_data_dir,
            ))),
            current_data_dir,
            default_data_dir,
            config_path,
        }
    }
}

#[derive(Debug)]
struct FileEntry {
    source: PathBuf,
    relative: PathBuf,
    size: u64,
}

fn error(context: &str, value: impl std::fmt::Display) -> String {
    format!("{context}: {value}")
}

fn locator_path(app: &AppHandle, default_data_dir: &Path) -> Result<PathBuf, String> {
    let config_candidate = app
        .path()
        .app_config_dir()
        .ok()
        .map(|path| path.join("storage-config"));
    let local_candidate = app
        .path()
        .app_local_data_dir()
        .ok()
        .map(|path| path.join("storage-config"));
    let sibling_candidate = default_data_dir
        .parent()
        .map(|path| path.join(".lifetrace-storage-config"));

    for directory in [config_candidate, local_candidate, sibling_candidate]
        .into_iter()
        .flatten()
    {
        if directory.starts_with(default_data_dir) || default_data_dir.starts_with(&directory) {
            continue;
        }
        fs::create_dir_all(&directory).map_err(|value| error("创建独立存储配置目录失败", value))?;
        return Ok(directory.join(CONFIG_FILE));
    }

    Err("无法找到与 LifeTrace 数据目录分离的存储配置位置".to_owned())
}

fn load_config(path: &Path) -> Result<StorageConfig, String> {
    if !path.is_file() {
        return Ok(StorageConfig::default());
    }
    let content = fs::read_to_string(path).map_err(|value| error("读取存储配置失败", value))?;
    serde_json::from_str(&content).map_err(|value| error("解析存储配置失败", value))
}

fn save_config(path: &Path, config: &StorageConfig) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "存储配置路径无效".to_owned())?;
    fs::create_dir_all(parent).map_err(|value| error("创建存储配置目录失败", value))?;
    let temporary = path.with_extension("json.tmp");
    let bytes =
        serde_json::to_vec_pretty(config).map_err(|value| error("序列化存储配置失败", value))?;
    fs::write(&temporary, bytes).map_err(|value| error("写入存储配置失败", value))?;
    if path.exists() {
        fs::remove_file(path).map_err(|value| error("替换存储配置失败", value))?;
    }
    fs::rename(&temporary, path).map_err(|value| error("提交存储配置失败", value))
}

fn write_marker(target: &Path) -> Result<(), String> {
    let value = serde_json::json!({ "format": "lifetrace-storage", "version": 1 });
    let bytes = serde_json::to_vec_pretty(&value).map_err(|value| value.to_string())?;
    fs::write(target.join(MARKER_FILE), bytes)
        .map_err(|value| error("写入 LifeTrace 存储标记失败", value))
}

fn is_database_file(relative: &Path) -> bool {
    matches!(
        relative.to_str(),
        Some("lifetrace.db")
            | Some("lifetrace.db-wal")
            | Some("lifetrace.db-shm")
            | Some("lifetrace.db-journal")
    )
}

fn should_skip(relative: &Path) -> bool {
    is_database_file(relative) || relative == Path::new(MARKER_FILE)
}

fn collect_files(root: &Path) -> Result<Vec<FileEntry>, String> {
    fn walk(root: &Path, current: &Path, output: &mut Vec<FileEntry>) -> Result<(), String> {
        for entry in fs::read_dir(current).map_err(|value| error("扫描存储目录失败", value))?
        {
            let entry = entry.map_err(|value| error("读取存储目录项失败", value))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|value| error("读取文件类型失败", value))?;
            let relative = path
                .strip_prefix(root)
                .map_err(|value| error("计算相对路径失败", value))?
                .to_path_buf();
            if should_skip(&relative) {
                continue;
            }
            if file_type.is_dir() {
                walk(root, &path, output)?;
            } else if file_type.is_file() {
                let size = entry
                    .metadata()
                    .map_err(|value| error("读取文件信息失败", value))?
                    .len();
                output.push(FileEntry {
                    source: path,
                    relative,
                    size,
                });
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    if root.is_dir() {
        walk(root, root, &mut files)?;
    }
    Ok(files)
}

fn validate_target(source: &Path, target: &Path, locator: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|value| error("创建新存储目录失败", value))?;
    let source = fs::canonicalize(source).map_err(|value| error("读取当前存储目录失败", value))?;
    let target = fs::canonicalize(target).map_err(|value| error("读取新存储目录失败", value))?;
    let locator_parent = locator.parent().unwrap_or(locator);
    let locator_parent =
        fs::canonicalize(locator_parent).unwrap_or_else(|_| locator_parent.to_path_buf());

    if source == target || target.starts_with(&source) || source.starts_with(&target) {
        return Err("新存储目录不能与当前目录相同，也不能互相包含".to_owned());
    }
    if locator_parent.starts_with(&target) || target.starts_with(&locator_parent) {
        return Err("该目录用于保存 LifeTrace 的存储位置配置，请选择其他文件夹".to_owned());
    }

    if target.join(MARKER_FILE).is_file() {
        return Ok(());
    }
    let mut entries = fs::read_dir(&target).map_err(|value| error("检查新存储目录失败", value))?;
    if entries.next().is_some() {
        return Err("请选择空文件夹，或选择之前由 LifeTrace 创建的存储文件夹".to_owned());
    }
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|value| error("创建目标目录失败", value))?;
    }
    fs::copy(source, destination).map_err(|value| error("复制本地数据失败", value))?;
    Ok(())
}

fn remove_database_target(target: &Path) -> Result<(), String> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let path = if suffix.is_empty() {
            target.to_path_buf()
        } else {
            PathBuf::from(format!("{}{}", target.display(), suffix))
        };
        if path.exists() {
            fs::remove_file(&path).map_err(|value| error("清理旧数据库副本失败", value))?;
        }
    }
    Ok(())
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|value| error("检查数据库表失败", value))?;
    Ok(count > 0)
}

fn rewrite_local_file_paths(connection: &Connection, target_root: &Path) -> Result<(), String> {
    if !table_exists(connection, "note_attachments")? {
        return Ok(());
    }

    let attachments = {
        let mut statement = connection
            .prepare("SELECT id, note_id, file_name FROM note_attachments")
            .map_err(|value| error("读取笔记附件路径失败", value))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|value| error("读取笔记附件路径失败", value))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|value| error("读取笔记附件路径失败", value))?
    };

    for (id, note_id, file_name) in attachments {
        let storage_path = target_root
            .join("attachments")
            .join(note_id)
            .join(file_name)
            .display()
            .to_string();
        connection
            .execute(
                "UPDATE note_attachments SET storage_path=?1 WHERE id=?2",
                params![storage_path, id],
            )
            .map_err(|value| error("更新笔记附件存储路径失败", value))?;
    }
    Ok(())
}

fn backup_database(source: &Path, target: &Path) -> Result<(), String> {
    if !source.is_file() {
        return Ok(());
    }
    let target_root = target
        .parent()
        .ok_or_else(|| "新数据库目录无效".to_owned())?;
    fs::create_dir_all(target_root).map_err(|value| error("创建数据库目标目录失败", value))?;
    remove_database_target(target)?;

    let source_connection =
        Connection::open(source).map_err(|value| error("打开当前数据库失败", value))?;
    let mut target_connection =
        Connection::open(target).map_err(|value| error("创建新数据库失败", value))?;
    {
        let backup = Backup::new(&source_connection, &mut target_connection)
            .map_err(|value| error("创建数据库一致性副本失败", value))?;
        let progress: Option<fn(rusqlite::backup::Progress)> = None;
        backup
            .run_to_completion(128, Duration::from_millis(10), progress)
            .map_err(|value| error("复制数据库失败", value))?;
    }

    rewrite_local_file_paths(&target_connection, target_root)?;
    let integrity: String = target_connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|value| error("校验新数据库失败", value))?;
    if integrity != "ok" {
        return Err(format!("新数据库完整性校验失败: {integrity}"));
    }
    Ok(())
}

fn update_progress(status: &Arc<Mutex<StorageStatus>>, files: u64, bytes: u64) {
    if let Ok(mut value) = status.lock() {
        value.files_copied = files;
        value.bytes_copied = bytes.min(value.bytes_total);
        value.progress = if value.bytes_total == 0 {
            100.0
        } else {
            (value.bytes_copied as f64 / value.bytes_total as f64 * 100.0).clamp(0.0, 100.0)
        };
    }
}

fn bulk_copy(
    source: &Path,
    target: &Path,
    locator: &Path,
    status: &Arc<Mutex<StorageStatus>>,
) -> Result<(), String> {
    validate_target(source, target, locator)?;
    write_marker(target)?;
    let files = collect_files(source)?;
    let database_size = fs::metadata(source.join(DATABASE_FILE))
        .map(|value| value.len())
        .unwrap_or(0);
    let bytes_total = files.iter().map(|entry| entry.size).sum::<u64>() + database_size;
    if let Ok(mut value) = status.lock() {
        value.files_total = files.len() as u64 + u64::from(database_size > 0);
        value.bytes_total = bytes_total;
    }

    let mut copied_files = 0;
    let mut copied_bytes = 0;
    for entry in files {
        match copy_file(&entry.source, &target.join(&entry.relative)) {
            Ok(()) => {
                copied_files += 1;
                copied_bytes += entry.size;
                update_progress(status, copied_files, copied_bytes);
            }
            Err(_) if !entry.source.exists() => {
                copied_files += 1;
                update_progress(status, copied_files, copied_bytes);
            }
            Err(value) => return Err(value),
        }
    }

    if database_size > 0 {
        backup_database(&source.join(DATABASE_FILE), &target.join(DATABASE_FILE))?;
        copied_files += 1;
        copied_bytes += database_size;
        update_progress(status, copied_files, copied_bytes);
    }
    Ok(())
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn copy_incremental(source: &Path, target: &Path) -> Result<(), String> {
    for entry in collect_files(source)? {
        let destination = target.join(&entry.relative);
        let should_copy = match fs::metadata(&destination) {
            Ok(_) if entry.size <= LARGE_FILE_THRESHOLD => true,
            Ok(metadata) if metadata.len() != entry.size => true,
            Ok(_) => match (modified(&entry.source), modified(&destination)) {
                (Some(source_time), Some(target_time)) => source_time > target_time,
                _ => true,
            },
            Err(_) => true,
        };
        if should_copy {
            copy_file(&entry.source, &destination)?;
        }
    }
    Ok(())
}

fn remove_stale_entries(source: &Path, target: &Path) -> Result<(), String> {
    if !target.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(target).map_err(|value| error("校准新存储目录失败", value))?
    {
        let entry = entry.map_err(|value| error("读取新存储目录失败", value))?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text == MARKER_FILE || name_text.starts_with(DATABASE_FILE) {
            continue;
        }
        let target_path = entry.path();
        let source_path = source.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|value| error("读取新存储文件类型失败", value))?;
        if !source_path.exists() {
            if file_type.is_dir() {
                fs::remove_dir_all(&target_path)
                    .map_err(|value| error("清理已删除目录副本失败", value))?;
            } else {
                fs::remove_file(&target_path)
                    .map_err(|value| error("清理已删除文件副本失败", value))?;
            }
        } else if file_type.is_dir() && source_path.is_dir() {
            remove_stale_entries(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn verify_tree(source: &Path, target: &Path) -> Result<(), String> {
    for entry in collect_files(source)? {
        let destination = target.join(&entry.relative);
        let metadata = fs::metadata(&destination)
            .map_err(|_| format!("迁移校验失败，目标文件缺失: {}", entry.relative.display()))?;
        if metadata.len() != entry.size {
            return Err(format!(
                "迁移校验失败，文件大小不一致: {}",
                entry.relative.display()
            ));
        }
    }
    if source.join(DATABASE_FILE).is_file() {
        let connection = Connection::open(target.join(DATABASE_FILE))
            .map_err(|value| error("打开迁移后的数据库失败", value))?;
        let integrity: String = connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|value| error("校验迁移后的数据库失败", value))?;
        if integrity != "ok" {
            return Err(format!("迁移后的数据库完整性检查失败: {integrity}"));
        }
    }
    Ok(())
}

fn finalize_pending(pending: &PendingMigration, locator: &Path) -> Result<(), String> {
    if !pending.source.is_dir() {
        return Err(format!("原存储目录不可用: {}", pending.source.display()));
    }
    validate_target(&pending.source, &pending.target, locator)?;
    copy_incremental(&pending.source, &pending.target)?;
    remove_stale_entries(&pending.source, &pending.target)?;
    backup_database(
        &pending.source.join(DATABASE_FILE),
        &pending.target.join(DATABASE_FILE),
    )?;
    verify_tree(&pending.source, &pending.target)?;
    write_marker(&pending.target)
}

fn commit_migration(
    config: &mut StorageConfig,
    locator: &Path,
    pending: &PendingMigration,
) -> Result<(), String> {
    config.active_data_dir = Some(pending.target.clone());
    config.pending_migration = None;
    config.cleanup_pending = Some(pending.source.clone());
    save_config(locator, config)
}

fn cancel_failed_migration(
    config: &mut StorageConfig,
    locator: &Path,
    pending: &PendingMigration,
    failure: &str,
) -> Result<(), String> {
    eprintln!(
        "LifeTrace storage migration finalization failed; continuing with old directory {}: {failure}",
        pending.source.display()
    );
    config.active_data_dir = Some(pending.source.clone());
    config.pending_migration = None;
    config.cleanup_pending = None;
    save_config(locator, config)
}

fn retry_old_directory_cleanup(config_path: &Path) -> Result<(), String> {
    let initial = load_config(config_path)?;
    let Some(old_path) = initial.cleanup_pending else {
        return Ok(());
    };
    if initial.active_data_dir.as_ref() == Some(&old_path) {
        let mut latest = load_config(config_path)?;
        if latest.cleanup_pending.as_ref() == Some(&old_path) {
            latest.cleanup_pending = None;
            save_config(config_path, &latest)?;
        }
        return Ok(());
    }

    if old_path.exists() {
        fs::remove_dir_all(&old_path)
            .map_err(|value| error("删除迁移完成后的旧存储目录失败", value))?;
    }

    let mut latest = load_config(config_path)?;
    if latest.cleanup_pending.as_ref() == Some(&old_path)
        && latest.active_data_dir.as_ref() != Some(&old_path)
    {
        latest.cleanup_pending = None;
        save_config(config_path, &latest)?;
    }
    Ok(())
}

pub fn schedule_pending_cleanup(config_path: PathBuf) {
    tokio::task::spawn_blocking(move || {
        if let Err(value) = retry_old_directory_cleanup(&config_path) {
            eprintln!("LifeTrace old storage cleanup deferred: {value}");
        }
    });
}

pub fn bootstrap(app: &AppHandle) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let default_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|value| error("无法读取默认数据目录", value))?;
    let locator = locator_path(app, &default_data_dir)?;
    let mut config = load_config(&locator)?;

    if let Some(pending) = config.pending_migration.clone() {
        match finalize_pending(&pending, &locator) {
            Ok(()) => commit_migration(&mut config, &locator, &pending)?,
            Err(failure) => cancel_failed_migration(&mut config, &locator, &pending, &failure)?,
        }
    }

    let current_data_dir = config
        .active_data_dir
        .clone()
        .unwrap_or_else(|| default_data_dir.clone());
    if config.active_data_dir.is_some() && !current_data_dir.is_dir() {
        return Err(format!(
            "当前 LifeTrace 存储位置不可用: {}",
            current_data_dir.display()
        ));
    }
    fs::create_dir_all(&current_data_dir)
        .map_err(|value| error("创建 LifeTrace 数据目录失败", value))?;
    Ok((current_data_dir, default_data_dir, locator))
}

#[tauri::command]
pub fn storage_status(state: State<'_, StorageState>) -> StorageStatus {
    state
        .status
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| StorageStatus::idle(&state.current_data_dir, &state.default_data_dir))
}

#[tauri::command]
pub fn storage_migrate(
    state: State<'_, StorageState>,
    target_path: String,
) -> Result<StorageStatus, String> {
    let target = PathBuf::from(target_path.trim());
    if target.as_os_str().is_empty() {
        return Err("请选择新的存储文件夹".to_owned());
    }
    validate_target(&state.current_data_dir, &target, &state.config_path)?;

    {
        let mut value = state
            .status
            .lock()
            .map_err(|_| "存储迁移状态不可用".to_owned())?;
        if matches!(value.phase.as_str(), "copying" | "finalizing") {
            return Err("已有存储迁移正在后台执行".to_owned());
        }
        *value = StorageStatus {
            current_path: state.current_data_dir.display().to_string(),
            default_path: state.default_data_dir.display().to_string(),
            target_path: Some(target.display().to_string()),
            phase: "copying".to_owned(),
            files_total: 0,
            files_copied: 0,
            bytes_total: 0,
            bytes_copied: 0,
            progress: 0.0,
            restart_required: false,
            error: None,
        };
    }

    let source = state.current_data_dir.clone();
    let locator = state.config_path.clone();
    let status = Arc::clone(&state.status);
    let target_for_task = target.clone();
    tokio::task::spawn_blocking(move || {
        let result = bulk_copy(&source, &target_for_task, &locator, &status).and_then(|_| {
            if let Ok(mut value) = status.lock() {
                value.phase = "finalizing".to_owned();
                value.progress = 100.0;
            }
            let mut config = load_config(&locator)?;
            config.pending_migration = Some(PendingMigration {
                source: source.clone(),
                target: target_for_task.clone(),
            });
            save_config(&locator, &config)
        });

        match result {
            Ok(()) => {
                if let Ok(mut value) = status.lock() {
                    value.phase = "ready".to_owned();
                    value.progress = 100.0;
                    value.restart_required = true;
                    value.error = None;
                }
            }
            Err(message) => {
                if let Ok(mut value) = status.lock() {
                    value.phase = "error".to_owned();
                    value.error = Some(message);
                    value.restart_required = false;
                }
            }
        }
    });

    Ok(storage_status(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("lifetrace-storage-{name}-{unique}"))
    }

    #[test]
    fn bulk_copy_preserves_files_sqlite_and_attachment_paths() {
        let source = temp("source");
        let target = temp("target");
        let locator = temp("locator").join(CONFIG_FILE);
        fs::create_dir_all(source.join("photos")).unwrap();
        fs::create_dir_all(source.join("attachments/n1")).unwrap();
        fs::create_dir_all(locator.parent().unwrap()).unwrap();
        fs::write(source.join("photos/a.jpg"), b"photo").unwrap();
        fs::write(source.join("attachments/n1/a.txt"), b"note").unwrap();
        let connection = Connection::open(source.join(DATABASE_FILE)).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sample(value TEXT);
                 INSERT INTO sample VALUES('ok');
                 CREATE TABLE note_attachments(id TEXT PRIMARY KEY,note_id TEXT,file_name TEXT,storage_path TEXT);
                 INSERT INTO note_attachments VALUES('a','n1','a.txt','old-path');",
            )
            .unwrap();
        drop(connection);

        let status = Arc::new(Mutex::new(StorageStatus::idle(&source, &source)));
        bulk_copy(&source, &target, &locator, &status).unwrap();
        verify_tree(&source, &target).unwrap();

        assert_eq!(fs::read(target.join("photos/a.jpg")).unwrap(), b"photo");
        let reopened = Connection::open(target.join(DATABASE_FILE)).unwrap();
        let value: String = reopened
            .query_row("SELECT value FROM sample", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "ok");
        let path: String = reopened
            .query_row(
                "SELECT storage_path FROM note_attachments WHERE id='a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            path,
            target.join("attachments/n1/a.txt").display().to_string()
        );

        fs::remove_dir_all(source).ok();
        fs::remove_dir_all(target).ok();
        fs::remove_dir_all(locator.parent().unwrap()).ok();
    }

    #[test]
    fn finalize_verifies_before_source_is_eligible_for_cleanup() {
        let source = temp("final-source");
        let target = temp("final-target");
        let locator = temp("final-locator").join(CONFIG_FILE);
        fs::create_dir_all(source.join("vault")).unwrap();
        fs::create_dir_all(locator.parent().unwrap()).unwrap();
        fs::write(source.join("vault/item.bin"), b"secret").unwrap();
        let connection = Connection::open(source.join(DATABASE_FILE)).unwrap();
        connection
            .execute_batch("CREATE TABLE t(id INTEGER); INSERT INTO t VALUES(1);")
            .unwrap();
        drop(connection);
        let status = Arc::new(Mutex::new(StorageStatus::idle(&source, &source)));
        bulk_copy(&source, &target, &locator, &status).unwrap();

        finalize_pending(
            &PendingMigration {
                source: source.clone(),
                target: target.clone(),
            },
            &locator,
        )
        .unwrap();
        assert!(source.exists());
        assert!(target.join("vault/item.bin").is_file());

        fs::remove_dir_all(source).ok();
        fs::remove_dir_all(target).ok();
        fs::remove_dir_all(locator.parent().unwrap()).ok();
    }

    #[test]
    fn failed_finalization_keeps_source_as_active_storage() {
        let source = temp("fallback-source");
        let target = temp("fallback-target");
        let locator = temp("fallback-locator").join(CONFIG_FILE);
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(locator.parent().unwrap()).unwrap();
        let pending = PendingMigration {
            source: source.clone(),
            target,
        };
        let mut config = StorageConfig {
            active_data_dir: Some(source.clone()),
            pending_migration: Some(pending.clone()),
            cleanup_pending: None,
        };

        cancel_failed_migration(&mut config, &locator, &pending, "simulated failure").unwrap();
        assert_eq!(config.active_data_dir, Some(source.clone()));
        assert!(config.pending_migration.is_none());
        assert!(source.exists());

        fs::remove_dir_all(source).ok();
        fs::remove_dir_all(locator.parent().unwrap()).ok();
    }
}
