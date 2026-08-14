use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use image::ImageFormat;
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::fs;
use uuid::Uuid;

use crate::database;
use crate::server::photo;

use super::runtime::SyncDesktopState;

const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StagedPhotoList {
    items: Vec<StagedPhoto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StagedPhoto {
    id: String,
    source: String,
    client_asset_id: Option<String>,
    sha256: String,
    original_name: String,
    media_type: String,
    mime_type: String,
    size_bytes: i64,
    captured_at: Option<String>,
}

pub async fn drain(state: &SyncDesktopState) -> Result<usize, String> {
    let auth = state.auth.read().await.clone();
    let token = auth
        .access_token
        .ok_or_else(|| "照片云端暂存同步需要登录".to_owned())?;
    if auth.origin.trim().is_empty() {
        return Err("照片云端暂存同步缺少云端地址".to_owned());
    }
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| error.to_string())?;
    let base = auth.origin.trim_end_matches('/');
    let list_response = client
        .get(format!("{base}/api/v1/photo-staging?limit=50"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|error| format!("无法读取云端暂存照片: {error}"))?;
    if !list_response.status().is_success() {
        return Err(format!(
            "读取云端暂存照片失败: HTTP {}",
            list_response.status().as_u16()
        ));
    }
    let list = list_response
        .json::<StagedPhotoList>()
        .await
        .map_err(|error| format!("云端暂存照片列表无效: {error}"))?;

    let mut imported = 0usize;
    for item in list.items {
        match import_one(&client, base, &token, &state.data_dir, &item).await {
            Ok(()) => imported += 1,
            Err(error) => {
                eprintln!(
                    "LifeTrace cloud photo staging import skipped id={} source={} clientAsset={:?}: {}",
                    item.id, item.source, item.client_asset_id, error
                );
            }
        }
    }
    Ok(imported)
}

async fn import_one(
    client: &Client,
    base: &str,
    token: &str,
    data_dir: &Path,
    item: &StagedPhoto,
) -> Result<(), String> {
    if item.size_bytes <= 0 || item.size_bytes as usize > MAX_DOWNLOAD_BYTES {
        return Err("云端暂存照片大小超出桌面端限制".to_owned());
    }
    if !matches!(item.media_type.as_str(), "image" | "video") {
        return Err(format!("不支持的媒体类型: {}", item.media_type));
    }
    let response = client
        .get(format!("{base}/api/v1/photo-staging/{}/content", item.id))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| format!("下载暂存照片失败: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("下载暂存照片失败: HTTP {}", response.status().as_u16()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("读取暂存照片内容失败: {error}"))?;
    if bytes.len() != item.size_bytes as usize || bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err("暂存照片下载大小校验失败".to_owned());
    }
    let actual_hash = hex::encode(Sha256::digest(&bytes));
    if actual_hash != item.sha256 {
        return Err("暂存照片 SHA-256 校验失败，云端副本不会删除".to_owned());
    }

    // ACK only after the original has been safely committed to the local
    // LifeTrace photo library (or confirmed as an existing local duplicate).
    import_into_local_library(data_dir, item, &bytes).await?;
    let ack = client
        .delete(format!("{base}/api/v1/photo-staging/{}", item.id))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| format!("本地已保存，但删除云端暂存副本失败: {error}"))?;
    if !ack.status().is_success() && ack.status() != reqwest::StatusCode::NOT_FOUND {
        return Err(format!(
            "本地已保存，但删除云端暂存副本失败: HTTP {}",
            ack.status().as_u16()
        ));
    }
    Ok(())
}

async fn import_into_local_library(
    data_dir: &Path,
    item: &StagedPhoto,
    bytes: &[u8],
) -> Result<String, String> {
    let connection = open_database(data_dir)?;
    if let Some(existing) = connection
        .query_row(
            "SELECT id FROM photos WHERE content_hash=?1 AND deleted_at IS NULL",
            [&item.sha256],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        return Ok(existing);
    }
    drop(connection);

    let root = data_dir.join("photos");
    let originals = root.join("originals");
    let thumbnails = root.join("thumbnails");
    let incoming = root.join(".cloud-staging");
    fs::create_dir_all(&originals).await.map_err(|error| error.to_string())?;
    fs::create_dir_all(&incoming).await.map_err(|error| error.to_string())?;

    let photo_id = format!("photo_{}", Uuid::new_v4());
    let extension = safe_extension(&item.original_name, &item.mime_type, &item.media_type);
    let stored_name = format!("{photo_id}.{extension}");
    let original_relative = format!("originals/{stored_name}");
    let original_path = root.join(&original_relative);
    let temp_path = incoming.join(format!("{}.part", item.id));
    fs::write(&temp_path, bytes).await.map_err(|error| error.to_string())?;
    fs::rename(&temp_path, &original_path)
        .await
        .or_else(|_| std::fs::copy(&temp_path, &original_path).map(|_| ()))
        .map_err(|error| format!("无法把云端照片写入本地相册: {error}"))?;
    fs::remove_file(&temp_path).await.ok();

    let mut width = None;
    let mut height = None;
    let mut thumbnail_relative = None;
    let mut thumbnail_path = None;
    if item.media_type == "image" {
        if let Ok(image) = image::load_from_memory(bytes) {
            width = Some(image.width() as i64);
            height = Some(image.height() as i64);
            fs::create_dir_all(&thumbnails).await.ok();
            let thumb_name = format!("{photo_id}.jpg");
            let path = thumbnails.join(&thumb_name);
            if image
                .thumbnail(640, 640)
                .save_with_format(&path, ImageFormat::Jpeg)
                .is_ok()
            {
                thumbnail_relative = Some(format!("thumbnails/{thumb_name}"));
                thumbnail_path = Some(path);
            }
        }
    }

    let imported_at = Utc::now().to_rfc3339();
    let connection = open_database(data_dir)?;
    let inserted = connection.execute(
        "INSERT OR IGNORE INTO photos(
            id,content_hash,original_file_name,stored_file_name,original_path,thumbnail_path,
            media_type,mime_type,file_size,width,height,duration_ms,captured_at,imported_at,
            processing_status,processing_error,source_device_id,deleted_at
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,NULL,?12,?13,'completed',NULL,NULL,NULL)",
        params![
            photo_id,
            item.sha256,
            clean_name(&item.original_name),
            stored_name,
            original_relative,
            thumbnail_relative,
            item.media_type,
            item.mime_type,
            item.size_bytes,
            width,
            height,
            item.captured_at,
            imported_at,
        ],
    )
    .map_err(|error| error.to_string())?;
    if inserted == 1 {
        return Ok(photo_id);
    }

    // A concurrent import may have inserted the same content hash while the
    // file was being written. Remove this redundant file and use that record.
    fs::remove_file(&original_path).await.ok();
    if let Some(path) = thumbnail_path {
        fs::remove_file(path).await.ok();
    }
    connection
        .query_row(
            "SELECT id FROM photos WHERE content_hash=?1 AND deleted_at IS NULL",
            [&item.sha256],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "照片本地入库失败".to_owned())
}

fn open_database(data_dir: &Path) -> Result<Connection, String> {
    let connection = database::connection::open(&data_dir.join("lifetrace.db"))
        .map_err(|error| error.to_string())?;
    photo::ensure_schema(&connection).map_err(|error| error.to_string())?;
    Ok(connection)
}

fn safe_extension(original_name: &str, mime_type: &str, media_type: &str) -> String {
    let from_name = PathBuf::from(original_name)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty() && value.len() <= 8)
        .map(|value| value.to_ascii_lowercase());
    from_name.unwrap_or_else(|| match mime_type {
        "image/png" => "png".to_owned(),
        "image/webp" => "webp".to_owned(),
        "video/quicktime" => "mov".to_owned(),
        "video/mp4" => "mp4".to_owned(),
        _ if media_type == "video" => "mp4".to_owned(),
        _ => "jpg".to_owned(),
    })
}

fn clean_name(value: &str) -> String {
    Path::new(value)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("photo")
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect()
}
