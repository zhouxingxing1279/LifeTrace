use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::Utc;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

const LOG_FILE_NAME: &str = "lifetrace-client.log";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const MAX_EVENT_BYTES: usize = 64 * 1024;
const MAX_READ_BYTES: usize = 1024 * 1024;
const ROTATED_FILES: usize = 3;

static LOG_LOCK: Mutex<()> = Mutex::new(());

fn log_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_log_dir()
        .map_err(|error| format!("resolve app log directory: {error}"))?;
    fs::create_dir_all(&directory).map_err(|error| format!("create app log directory: {error}"))?;
    Ok(directory)
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(log_directory(app)?.join(LOG_FILE_NAME))
}

fn rotated_path(path: &Path, index: usize) -> PathBuf {
    path.with_file_name(format!("{LOG_FILE_NAME}.{index}"))
}

fn rotate_if_needed(path: &Path, incoming_bytes: u64) -> Result<(), String> {
    let current_bytes = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if current_bytes.saturating_add(incoming_bytes) <= MAX_LOG_BYTES {
        return Ok(());
    }

    for index in (1..=ROTATED_FILES).rev() {
        let destination = rotated_path(path, index);
        let source = if index == 1 {
            path.to_path_buf()
        } else {
            rotated_path(path, index - 1)
        };
        if !source.exists() {
            continue;
        }
        if destination.exists() {
            fs::remove_file(&destination)
                .map_err(|error| format!("remove rotated client log: {error}"))?;
        }
        fs::rename(&source, &destination).map_err(|error| format!("rotate client log: {error}"))?;
    }
    Ok(())
}

fn serialize_event(event: Value) -> String {
    let serialized = serde_json::to_string(&event).unwrap_or_else(|error| {
        json!({
            "schemaVersion": 1,
            "timestamp": Utc::now().to_rfc3339(),
            "level": "error",
            "event": "logger.serialization.failed",
            "runtime": "tauri",
            "data": { "message": error.to_string() }
        })
        .to_string()
    });

    if serialized.len() <= MAX_EVENT_BYTES {
        return serialized;
    }

    json!({
        "schemaVersion": 1,
        "timestamp": Utc::now().to_rfc3339(),
        "level": "warn",
        "event": "logger.event.truncated",
        "runtime": "tauri",
        "data": {
            "originalBytes": serialized.len(),
            "maxBytes": MAX_EVENT_BYTES
        }
    })
    .to_string()
}

#[tauri::command]
pub fn client_log_write(app: AppHandle, event: Value) -> Result<(), String> {
    let _guard = LOG_LOCK
        .lock()
        .map_err(|_| "client log lock poisoned".to_string())?;
    let path = log_path(&app)?;
    let line = format!("{}\n", serialize_event(event));
    rotate_if_needed(&path, line.len() as u64)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("open client log: {error}"))?;
    file.write_all(line.as_bytes())
        .map_err(|error| format!("write client log: {error}"))?;
    file.flush()
        .map_err(|error| format!("flush client log: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn client_log_path(app: AppHandle) -> Result<String, String> {
    Ok(log_path(&app)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn client_log_read_recent(app: AppHandle, max_bytes: Option<usize>) -> Result<String, String> {
    let _guard = LOG_LOCK
        .lock()
        .map_err(|_| "client log lock poisoned".to_string())?;
    let path = log_path(&app)?;
    if !path.exists() {
        return Ok(String::new());
    }

    let limit = max_bytes
        .unwrap_or(256 * 1024)
        .clamp(4 * 1024, MAX_READ_BYTES);
    let mut file = fs::File::open(&path).map_err(|error| format!("open client log: {error}"))?;
    let length = file
        .metadata()
        .map_err(|error| format!("read client log metadata: {error}"))?
        .len();
    let start = length.saturating_sub(limit as u64);
    if start > 0 {
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(start))
            .map_err(|error| format!("seek client log: {error}"))?;
    }

    let mut bytes = Vec::with_capacity(limit);
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("read client log: {error}"))?;
    let content = String::from_utf8_lossy(&bytes);
    if start == 0 {
        return Ok(content.into_owned());
    }
    Ok(content
        .split_once('\n')
        .map(|(_, rest)| rest.to_string())
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_events_are_replaced_with_a_valid_summary() {
        let event = json!({ "message": "x".repeat(MAX_EVENT_BYTES + 1) });
        let serialized = serialize_event(event);
        let parsed: Value = serde_json::from_str(&serialized).expect("valid JSON");
        assert_eq!(parsed["event"], "logger.event.truncated");
    }

    #[test]
    fn regular_events_remain_json_lines_compatible() {
        let event = json!({ "event": "api.request.start", "level": "info" });
        let serialized = serialize_event(event.clone());
        assert_eq!(serde_json::from_str::<Value>(&serialized).unwrap(), event);
        assert!(!serialized.contains('\n'));
    }
}
