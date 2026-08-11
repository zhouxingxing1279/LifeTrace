use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use serde_json::{json, Value};
use tauri::State;
use tokio::fs;
use uuid::Uuid;

use crate::server::photo::Runtime;

pub struct DesktopState {
    pub data_dir: PathBuf,
    pub photo_runtime: Arc<Runtime>,
}

fn safe_segment(value: &str) -> Result<&str, String> {
    if value.is_empty()
        || value.len() > 180
        || value.contains(['/', '\\'])
        || value == "."
        || value == ".."
    {
        Err("文件标识无效".to_owned())
    } else {
        Ok(value)
    }
}

#[tauri::command]
pub fn desktop_open_url(url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|_| "链接地址无效".to_owned())?;
    if !matches!(parsed.scheme(), "http" | "https" | "mailto") {
        return Err("不允许打开该类型的链接".to_owned());
    }

    #[cfg(target_os = "windows")]
    let result = Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(parsed.as_str())
        .spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(parsed.as_str()).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(parsed.as_str()).spawn();

    result
        .map(|_| ())
        .map_err(|error| format!("无法调用系统默认浏览器：{error}"))
}

#[tauri::command]
pub fn photo_status(state: State<'_, DesktopState>) -> Value {
    json!({ "ok": true, "status": state.photo_runtime.status() })
}

#[tauri::command]
pub fn mobile_upload_status(state: State<'_, DesktopState>) -> Value {
    json!({ "ok": true, "status": state.photo_runtime.mobile_upload_status() })
}

#[tauri::command]
pub fn mobile_upload_start(state: State<'_, DesktopState>) -> Value {
    state.photo_runtime.set_active(true);
    json!({ "ok": true, "status": state.photo_runtime.mobile_upload_status() })
}

#[tauri::command]
pub fn mobile_upload_stop(state: State<'_, DesktopState>) -> Value {
    state.photo_runtime.set_active(false);
    json!({ "ok": true, "status": state.photo_runtime.mobile_upload_status() })
}

#[tauri::command]
pub fn photo_create_pairing(state: State<'_, DesktopState>) -> Value {
    match state.photo_runtime.create_pairing() {
        Ok(status) => json!({ "ok": true, "status": status }),
        Err(error) => json!({ "ok": false, "error": error }),
    }
}

#[tauri::command]
pub fn photo_cancel_pairing(state: State<'_, DesktopState>, pair_code: String) -> Value {
    state.photo_runtime.cancel_pairing(&pair_code);
    json!({ "ok": true, "status": state.photo_runtime.status() })
}

#[tauri::command]
pub fn photo_recover(state: State<'_, DesktopState>) -> Value {
    json!({ "ok": true, "status": state.photo_runtime.status() })
}

#[tauri::command]
pub fn photo_set_compatibility(state: State<'_, DesktopState>, enabled: bool) -> Value {
    state.photo_runtime.set_compatibility(enabled);
    json!({ "ok": true, "status": state.photo_runtime.status() })
}

#[tauri::command]
pub async fn photo_export_certificate(
    state: State<'_, DesktopState>,
    destination: String,
) -> Result<Value, String> {
    fs::copy(state.photo_runtime.certificate_path(), &destination)
        .await
        .map_err(|value| value.to_string())?;
    let mut status = state.photo_runtime.status();
    if let Some(object) = status.as_object_mut() {
        object.insert("certificateExported".to_owned(), json!(true));
        object.insert("certificateExportPath".to_owned(), json!(destination));
    }
    Ok(json!({ "ok": true, "status": status }))
}

#[tauri::command]
pub async fn note_copy_attachment(
    state: State<'_, DesktopState>,
    note_id: String,
    source_path: String,
) -> Result<Value, String> {
    let note_id = safe_segment(&note_id)?;
    let source = PathBuf::from(source_path);
    if !source.is_file() {
        return Err("所选附件不存在".to_owned());
    }
    let original_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "附件名称无效".to_owned())?
        .to_owned();
    let clean_name = original_name
        .chars()
        .filter(|character| !character.is_control() && !matches!(character, '/' | '\\'))
        .take(120)
        .collect::<String>();
    let file_name = format!("{}-{}", Uuid::new_v4(), clean_name);
    let folder = state.data_dir.join("attachments").join(note_id);
    fs::create_dir_all(&folder)
        .await
        .map_err(|value| value.to_string())?;
    let destination = folder.join(&file_name);
    fs::copy(&source, &destination)
        .await
        .map_err(|value| value.to_string())?;
    let metadata = fs::metadata(&destination)
        .await
        .map_err(|value| value.to_string())?;
    Ok(json!({
        "id": Uuid::new_v4().to_string(), "noteId": note_id, "fileName": file_name,
        "originalName": original_name, "mimeType": mime_from_path(&source),
        "fileSize": metadata.len(), "storagePath": destination.display().to_string(),
        "createdAt": chrono::Utc::now().to_rfc3339()
    }))
}

fn mime_from_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "md" | "txt" => "text/plain",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn attachment_path(
    state: &DesktopState,
    note_id: &str,
    file_name: &str,
) -> Result<PathBuf, String> {
    Ok(state
        .data_dir
        .join("attachments")
        .join(safe_segment(note_id)?)
        .join(safe_segment(file_name)?))
}

#[tauri::command]
pub async fn note_delete_attachment(
    state: State<'_, DesktopState>,
    note_id: String,
    file_name: String,
) -> Result<Value, String> {
    let path = attachment_path(&state, &note_id, &file_name)?;
    match fs::remove_file(path).await {
        Ok(_) => Ok(json!({ "ok": true })),
        Err(value) if value.kind() == std::io::ErrorKind::NotFound => Ok(json!({ "ok": true })),
        Err(value) => Err(value.to_string()),
    }
}

fn open_with_explorer(path: &Path, select: bool) -> Result<Value, String> {
    let mut command = Command::new("explorer.exe");
    if select {
        command.arg(format!("/select,{}", path.display()));
    } else {
        command.arg(path);
    }
    command.spawn().map_err(|value| value.to_string())?;
    Ok(json!({ "ok": true }))
}

#[tauri::command]
pub fn note_open_attachment(
    state: State<'_, DesktopState>,
    note_id: String,
    file_name: String,
) -> Result<Value, String> {
    open_with_explorer(&attachment_path(&state, &note_id, &file_name)?, false)
}

#[tauri::command]
pub fn note_show_attachment(
    state: State<'_, DesktopState>,
    note_id: String,
    file_name: String,
) -> Result<Value, String> {
    open_with_explorer(&attachment_path(&state, &note_id, &file_name)?, true)
}

#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<Value, String> {
    fs::write(path, content)
        .await
        .map_err(|value| value.to_string())?;
    Ok(json!({ "ok": true }))
}

#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(path)
        .await
        .map_err(|value| value.to_string())
}
