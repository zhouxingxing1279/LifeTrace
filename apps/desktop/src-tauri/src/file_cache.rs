//! EPIC-12 on-demand verified file cache.
//!
//! Remote bytes are streamed into a temporary file and become visible only
//! after both size and SHA-256 match Cloud metadata. Thumbnails are derived
//! local cache and can always be regenerated.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::desktop::DesktopState;

const DEFAULT_CACHE_MAX_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const MAX_DOWNLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const THUMBNAIL_EDGE: u32 = 512;

#[tauri::command]
pub async fn file_cache_download(
    state: State<'_, DesktopState>,
    url: String,
    sha256: String,
    size_bytes: u64,
) -> Result<String, String> {
    if size_bytes > MAX_DOWNLOAD_BYTES {
        return Err("文件超过桌面缓存支持的大小".to_owned());
    }
    let root = state.data_dir.join("file-cache");
    let path = download_verified(&root, &url, &sha256, size_bytes).await?;
    let cleanup_root = root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        cleanup_cache(&cleanup_root, DEFAULT_CACHE_MAX_BYTES)
    })
    .await
    .map_err(|error| format!("缓存清理任务失败：{error}"))??;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn file_cache_thumbnail(
    state: State<'_, DesktopState>,
    sha256: String,
) -> Result<String, String> {
    let sha256 = normalize_sha256(&sha256)?;
    let root = state.data_dir.join("file-cache");
    let source = cache_path(&root, &sha256);
    if !source.is_file() {
        return Err("原文件尚未下载到本地缓存".to_owned());
    }
    let target = thumbnail_path(&root, &sha256);
    let generated =
        tauri::async_runtime::spawn_blocking(move || generate_thumbnail(&source, &target))
            .await
            .map_err(|error| format!("缩略图任务失败：{error}"))??;
    Ok(generated.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn file_cache_cleanup(
    state: State<'_, DesktopState>,
    max_bytes: Option<u64>,
) -> Result<u64, String> {
    let root = state.data_dir.join("file-cache");
    let max_bytes = max_bytes
        .unwrap_or(DEFAULT_CACHE_MAX_BYTES)
        .clamp(64 * 1024 * 1024, 20 * 1024 * 1024 * 1024);
    tauri::async_runtime::spawn_blocking(move || cleanup_cache(&root, max_bytes))
        .await
        .map_err(|error| format!("缓存清理任务失败：{error}"))?
}

async fn download_verified(
    root: &Path,
    url: &str,
    expected_sha256: &str,
    expected_size: u64,
) -> Result<PathBuf, String> {
    let expected_sha256 = normalize_sha256(expected_sha256)?;
    let parsed = url::Url::parse(url).map_err(|_| "文件下载地址无效".to_owned())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("文件下载地址必须使用 HTTP(S)".to_owned());
    }
    let target = cache_path(root, &expected_sha256);
    if target.is_file() && verify_file(&target, &expected_sha256, expected_size).await? {
        touch_access(root, &expected_sha256).await?;
        return Ok(target);
    }

    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("创建文件缓存目录失败：{error}"))?;
    }
    let temp = target.with_extension(format!("part-{}", Uuid::new_v4()));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("创建文件下载客户端失败：{error}"))?;
    let response = client
        .get(parsed)
        .send()
        .await
        .map_err(|error| format!("下载文件失败：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("下载文件失败：HTTP {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|length| length != expected_size || length > MAX_DOWNLOAD_BYTES)
    {
        return Err("下载文件 Content-Length 与元数据不一致".to_owned());
    }

    let mut output = tokio::fs::File::create(&temp)
        .await
        .map_err(|error| format!("创建临时缓存文件失败：{error}"))?;
    let mut hasher = Sha256::new();
    let mut written = 0u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("读取下载流失败：{error}"))?;
        written = written
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| "下载文件大小溢出".to_owned())?;
        if written > expected_size || written > MAX_DOWNLOAD_BYTES {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err("下载文件超过声明大小".to_owned());
        }
        hasher.update(&chunk);
        output
            .write_all(&chunk)
            .await
            .map_err(|error| format!("写入临时缓存文件失败：{error}"))?;
    }
    output
        .flush()
        .await
        .map_err(|error| format!("刷新临时缓存文件失败：{error}"))?;
    drop(output);

    let actual_sha256 = hex::encode(hasher.finalize());
    if written != expected_size || actual_sha256 != expected_sha256 {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err("下载文件完整性校验失败".to_owned());
    }
    if target.exists() {
        let _ = tokio::fs::remove_file(&target).await;
    }
    tokio::fs::rename(&temp, &target)
        .await
        .map_err(|error| format!("提交文件缓存失败：{error}"))?;
    touch_access(root, &expected_sha256).await?;
    Ok(target)
}

async fn verify_file(
    path: &Path,
    expected_sha256: &str,
    expected_size: u64,
) -> Result<bool, String> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|error| format!("读取缓存文件信息失败：{error}"))?;
    if metadata.len() != expected_size {
        return Ok(false);
    }
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| format!("打开缓存文件失败：{error}"))?;
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut hasher = Sha256::new();
    loop {
        let count = file
            .read(&mut buffer)
            .await
            .map_err(|error| format!("读取缓存文件失败：{error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()) == expected_sha256)
}

async fn touch_access(root: &Path, sha256: &str) -> Result<(), String> {
    let path = access_path(root, sha256);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("创建缓存访问索引失败：{error}"))?;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    tokio::fs::write(path, timestamp)
        .await
        .map_err(|error| format!("更新缓存访问时间失败：{error}"))
}

fn generate_thumbnail(source: &Path, target: &Path) -> Result<PathBuf, String> {
    if target.is_file() {
        return Ok(target.to_path_buf());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("创建缩略图缓存目录失败：{error}"))?;
    }
    let file = File::open(source).map_err(|error| format!("打开缩略图源文件失败：{error}"))?;
    let reader = image::ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|error| format!("识别图片格式失败：{error}"))?;
    let image = reader
        .decode()
        .map_err(|error| format!("解码图片失败：{error}"))?;
    let thumbnail = image.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE);
    let temp = target.with_extension(format!("part-{}", Uuid::new_v4()));
    thumbnail
        .save_with_format(&temp, image::ImageFormat::Jpeg)
        .map_err(|error| format!("生成缩略图失败：{error}"))?;
    std::fs::rename(&temp, target).map_err(|error| format!("提交缩略图失败：{error}"))?;
    Ok(target.to_path_buf())
}

fn cleanup_cache(root: &Path, max_bytes: u64) -> Result<u64, String> {
    let objects = root.join("objects");
    if !objects.exists() {
        return Ok(0);
    }
    let mut entries = Vec::new();
    collect_blob_files(&objects, &mut entries)?;
    let mut total = entries.iter().map(|entry| entry.1).sum::<u64>();
    if total <= max_bytes {
        return Ok(total);
    }
    entries.sort_by_key(|(path, _, sha256)| {
        access_path(root, sha256)
            .metadata()
            .and_then(|metadata| metadata.modified())
            .or_else(|_| path.metadata().and_then(|metadata| metadata.modified()))
            .unwrap_or(UNIX_EPOCH)
    });
    for (path, size, sha256) in entries {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
            let _ = std::fs::remove_file(access_path(root, &sha256));
            let _ = std::fs::remove_file(thumbnail_path(root, &sha256));
        }
    }
    Ok(total)
}

fn collect_blob_files(root: &Path, output: &mut Vec<(PathBuf, u64, String)>) -> Result<(), String> {
    for entry in std::fs::read_dir(root).map_err(|error| format!("读取缓存目录失败：{error}"))?
    {
        let entry = entry.map_err(|error| format!("读取缓存项失败：{error}"))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("读取缓存项信息失败：{error}"))?;
        if metadata.is_dir() {
            collect_blob_files(&path, output)?;
        } else if metadata.is_file() {
            let sha256 = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| normalize_sha256(value).ok());
            if let Some(sha256) = sha256 {
                output.push((path, metadata.len(), sha256));
            }
        }
    }
    Ok(())
}

fn normalize_sha256(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value)
    } else {
        Err("SHA-256 必须是 64 位十六进制字符串".to_owned())
    }
}

fn cache_path(root: &Path, sha256: &str) -> PathBuf {
    root.join("objects")
        .join(&sha256[..2])
        .join(format!("{sha256}.blob"))
}

fn access_path(root: &Path, sha256: &str) -> PathBuf {
    root.join("access").join(format!("{sha256}.touch"))
}

fn thumbnail_path(root: &Path, sha256: &str) -> PathBuf {
    root.join("thumbnails")
        .join(&sha256[..2])
        .join(format!("{sha256}.jpg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("lifetrace-file-cache-{name}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[tokio::test]
    async fn wrong_hash_never_counts_as_verified_cache() {
        let root = test_root("hash");
        let expected = "a".repeat(64);
        let path = cache_path(&root, &expected);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"not-the-declared-file").unwrap();
        assert!(!verify_file(&path, &expected, 21).await.unwrap());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn same_hash_maps_to_same_content_addressed_path() {
        let root = Path::new("cache");
        let sha = "b".repeat(64);
        assert_eq!(cache_path(root, &sha), cache_path(root, &sha));
        assert!(cache_path(root, &sha).ends_with(format!("{sha}.blob")));
    }

    #[test]
    fn cleanup_removes_cached_bytes_and_derived_thumbnail() {
        let root = test_root("cleanup");
        let sha = "c".repeat(64);
        let object = cache_path(&root, &sha);
        std::fs::create_dir_all(object.parent().unwrap()).unwrap();
        std::fs::write(&object, vec![1u8; 1024]).unwrap();
        let thumb = thumbnail_path(&root, &sha);
        std::fs::create_dir_all(thumb.parent().unwrap()).unwrap();
        std::fs::write(&thumb, b"derived").unwrap();
        assert_eq!(cleanup_cache(&root, 0).unwrap(), 0);
        assert!(!object.exists());
        assert!(!thumb.exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn thumbnail_can_be_regenerated_from_content_not_extension() {
        let root = test_root("thumb");
        let sha = "d".repeat(64);
        let source = cache_path(&root, &sha);
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(16, 12));
        image
            .save_with_format(&source, image::ImageFormat::Png)
            .unwrap();
        let target = thumbnail_path(&root, &sha);
        assert_eq!(generate_thumbnail(&source, &target).unwrap(), target);
        assert!(target.is_file());
        let _ = std::fs::remove_dir_all(root);
    }
}
