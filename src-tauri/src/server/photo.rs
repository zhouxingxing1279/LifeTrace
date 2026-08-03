use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use axum::{
    body::{to_bytes, Body},
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, Method, Request, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{any, get},
    Json, Router,
};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::{fs, net::TcpListener};
use uuid::Uuid;

use super::AppState;

#[derive(Clone)]
struct Pairing {
    expires_at: chrono::DateTime<Utc>,
    server: String,
}

pub struct Runtime {
    data_dir: PathBuf,
    pairings: Mutex<HashMap<String, Pairing>>,
    active: AtomicBool,
    allow_insecure_http: AtomicBool,
}

impl Runtime {
    pub fn new(_data_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            data_dir: _data_dir,
            pairings: Mutex::new(HashMap::new()),
            active: AtomicBool::new(false),
            allow_insecure_http: AtomicBool::new(false),
        })
    }

    pub fn status(&self) -> Value {
        let addresses = lan_addresses();
        let insecure = self.allow_insecure_http.load(Ordering::Relaxed);
        let protocol = if insecure { "http" } else { "https" };
        let port = if insecure { 3445 } else { 3443 };
        let urls = addresses
            .iter()
            .map(|address| format!("{protocol}://{address}:{port}"))
            .collect::<Vec<_>>();
        let certificate_path = self
            .data_dir
            .join(".local-certificates")
            .join("LifeTrace-Local-CA.cer");
        json!({
            "available": true, "active": self.active.load(Ordering::Relaxed), "managed": true,
            "port": port, "urls": urls, "photoSyncUrls": urls,
            "computerName": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "LifeTrace-PC".to_owned()),
            "bindAddress": "0.0.0.0", "mediaUrl": "http://127.0.0.1:3444",
            "certificateReady": certificate_path.is_file(), "certificateAddresses": addresses,
            "certificateExported": false, "certificateCommonName": "LifeTrace Local",
            "allowInsecureHttp": insecure, "transportProtocol": protocol
        })
    }

    pub fn mobile_upload_status(&self) -> Value {
        let urls = lan_addresses()
            .iter()
            .map(|address| format!("http://{address}:3445/fitness"))
            .collect::<Vec<_>>();
        json!({
            "available": true,
            "active": self.active.load(Ordering::Relaxed),
            "managed": true,
            "port": 3445,
            "urls": urls
        })
    }

    pub fn create_pairing(&self) -> Result<Value, String> {
        self.set_active(true);
        let code = format!("{:06}", Uuid::new_v4().as_u128() % 1_000_000);
        let expires = Utc::now() + Duration::minutes(5);
        let address = lan_addresses()
            .into_iter()
            .next()
            .unwrap_or_else(|| "127.0.0.1".to_owned());
        let insecure = self.allow_insecure_http.load(Ordering::Relaxed);
        let server = if insecure {
            format!("http://{address}:3445")
        } else {
            format!("https://{address}:3443")
        };
        self.pairings
            .lock()
            .map_err(|_| "配对状态锁已损坏".to_owned())?
            .insert(
                code.clone(),
                Pairing {
                    expires_at: expires,
                    server: server.clone(),
                },
            );
        let pairing = json!({
            "success": true, "pairCode": code, "expiresAt": expires.to_rfc3339(),
            "entryUrl": format!("{server}/api/photo-sync/shortcut-entry?code={code}")
        });
        let mut status = self.status();
        status
            .as_object_mut()
            .expect("status object")
            .insert("pairing".to_owned(), pairing);
        Ok(status)
    }

    pub fn cancel_pairing(&self, code: &str) {
        if let Ok(mut pairings) = self.pairings.lock() {
            pairings.remove(code);
        }
    }

    pub fn set_compatibility(&self, enabled: bool) {
        self.allow_insecure_http.store(enabled, Ordering::Relaxed);
        if let Ok(mut pairings) = self.pairings.lock() {
            pairings.clear();
        }
    }

    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Relaxed);
        if !active {
            if let Ok(mut pairings) = self.pairings.lock() {
                pairings.clear();
            }
        }
    }

    pub fn certificate_path(&self) -> PathBuf {
        self.data_dir
            .join(".local-certificates")
            .join("LifeTrace-Local-CA.cer")
    }

    fn certificate_files(&self) -> Result<(PathBuf, PathBuf), String> {
        let directory = self.data_dir.join(".local-certificates");
        std::fs::create_dir_all(&directory).map_err(|value| value.to_string())?;
        let certificate = directory.join("server-cert.pem");
        let key = directory.join("server-key.pem");
        let export = self.certificate_path();
        if certificate.is_file() && key.is_file() && export.is_file() {
            return Ok((certificate, key));
        }
        let mut names = vec![
            "localhost".to_owned(),
            "lifetrace.local".to_owned(),
            "127.0.0.1".to_owned(),
        ];
        names.extend(lan_addresses());
        let generated =
            rcgen::generate_simple_self_signed(names).map_err(|value| value.to_string())?;
        std::fs::write(&certificate, generated.cert.pem()).map_err(|value| value.to_string())?;
        std::fs::write(&key, generated.signing_key.serialize_pem())
            .map_err(|value| value.to_string())?;
        std::fs::write(&export, generated.cert.der().as_ref())
            .map_err(|value| value.to_string())?;
        Ok((certificate, key))
    }
}

fn lan_addresses() -> Vec<String> {
    let mut addresses = Vec::new();
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("1.1.1.1:80").is_ok() {
            if let Ok(local) = socket.local_addr() {
                if let IpAddr::V4(address) = local.ip() {
                    if !address.is_loopback() {
                        addresses.push(address.to_string());
                    }
                }
            }
        }
    }
    addresses
}

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS photos(
           id TEXT PRIMARY KEY,content_hash TEXT NOT NULL UNIQUE,original_file_name TEXT NOT NULL,
           stored_file_name TEXT NOT NULL,original_path TEXT NOT NULL,thumbnail_path TEXT,
           media_type TEXT NOT NULL,mime_type TEXT,file_size INTEGER NOT NULL,width INTEGER,
           height INTEGER,duration_ms INTEGER,captured_at TEXT,imported_at TEXT NOT NULL,
           processing_status TEXT NOT NULL,processing_error TEXT,source_device_id TEXT,deleted_at TEXT
         );
         CREATE TABLE IF NOT EXISTS photo_sync_devices(
           id TEXT PRIMARY KEY,device_name TEXT NOT NULL,device_type TEXT NOT NULL,
           device_uuid TEXT NOT NULL UNIQUE,token_hash TEXT NOT NULL UNIQUE,status TEXT NOT NULL,
           paired_at TEXT NOT NULL,last_seen_at TEXT,revoked_at TEXT
         );
         CREATE TABLE IF NOT EXISTS photo_upload_tasks(
           id TEXT PRIMARY KEY,device_id TEXT NOT NULL,client_asset_id TEXT NOT NULL,
           original_file_name TEXT NOT NULL,media_type TEXT NOT NULL,mime_type TEXT,
           captured_at TEXT,expected_file_size INTEGER NOT NULL,received_file_size INTEGER NOT NULL DEFAULT 0,
           temporary_path TEXT NOT NULL,status TEXT NOT NULL,photo_id TEXT,created_at TEXT NOT NULL,
           updated_at TEXT NOT NULL,expires_at TEXT NOT NULL,error_code TEXT,error_message TEXT,
           is_duplicate INTEGER NOT NULL DEFAULT 0,UNIQUE(device_id,client_asset_id)
         );
         CREATE TABLE IF NOT EXISTS photo_device_assets(
           device_id TEXT NOT NULL,client_asset_id TEXT NOT NULL,photo_id TEXT NOT NULL,
           synced_at TEXT NOT NULL,UNIQUE(device_id,client_asset_id)
         );
         CREATE INDEX IF NOT EXISTS photos_captured_at_idx ON photos(captured_at);
         CREATE INDEX IF NOT EXISTS photo_tasks_status_idx ON photo_upload_tasks(status);",
    )
}

fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({ "success": false, "error": code, "message": message })),
    )
        .into_response()
}

pub async fn dashboard_get(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let page = query
        .get("page")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let page_size = query
        .get("pageSize")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(30)
        .clamp(1, 60);
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "数据库暂时不可用",
            )
        }
    };
    match dashboard(&connection, page, page_size) {
        Ok(value) => Json(value).into_response(),
        Err(message) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            &message,
        ),
    }
}

fn rows(
    connection: &Connection,
    sql: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(sql).map_err(|value| value.to_string())?;
    let names = statement
        .column_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let mapped = statement
        .query_map(values, |row| {
            let mut object = serde_json::Map::new();
            for (index, name) in names.iter().enumerate() {
                let value = row.get_ref(index)?;
                let json_value = match value {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(value) => json!(value),
                    rusqlite::types::ValueRef::Real(value) => json!(value),
                    rusqlite::types::ValueRef::Text(value) => json!(String::from_utf8_lossy(value)),
                    rusqlite::types::ValueRef::Blob(_) => Value::Null,
                };
                object.insert(name.clone(), json_value);
            }
            Ok(Value::Object(object))
        })
        .map_err(|value| value.to_string())?;
    mapped
        .map(|row| row.map_err(|value| value.to_string()))
        .collect()
}

fn dashboard(connection: &Connection, page: usize, page_size: usize) -> Result<Value, String> {
    let offset = (page - 1) * page_size;
    let photos = rows(connection, "SELECT p.id,p.original_file_name,p.media_type,p.mime_type,p.file_size,p.width,p.height,p.duration_ms,p.captured_at,p.imported_at,p.processing_status,p.processing_error,d.device_name FROM photos p LEFT JOIN photo_sync_devices d ON d.id=p.source_device_id WHERE p.deleted_at IS NULL ORDER BY COALESCE(p.captured_at,p.imported_at) DESC LIMIT ?1 OFFSET ?2", &[&(page_size as i64), &(offset as i64)])?;
    let devices = rows(connection, "SELECT id,device_name,device_type,status,paired_at,last_seen_at,revoked_at FROM photo_sync_devices ORDER BY paired_at DESC", &[])?;
    let tasks = rows(connection, "SELECT id,device_id,original_file_name,expected_file_size,received_file_size,status,photo_id,created_at,updated_at,error_code,error_message FROM photo_upload_tasks WHERE status NOT IN ('completed','expired') ORDER BY updated_at DESC LIMIT 100", &[])?;
    let total: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM photos WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|value| value.to_string())?;
    let summary = rows(connection, "SELECT COALESCE(SUM(CASE WHEN status='completed' THEN 1 ELSE 0 END),0) success_count,COALESCE(SUM(CASE WHEN is_duplicate=1 THEN 1 ELSE 0 END),0) duplicate_count,COALESCE(SUM(CASE WHEN status='failed' THEN 1 ELSE 0 END),0) failed_count,COALESCE(SUM(CASE WHEN status IN ('uploaded','processing') THEN 1 ELSE 0 END),0) processing_count,MAX(updated_at) last_sync_at FROM photo_upload_tasks", &[])?.into_iter().next().unwrap_or_else(|| json!({}));
    Ok(
        json!({ "photos": photos, "total": total, "page": page, "pageSize": page_size, "devices": devices, "tasks": tasks, "summary": summary }),
    )
}

pub async fn dashboard_post(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "数据库暂时不可用",
            )
        }
    };
    let result = match body.get("action").and_then(Value::as_str) {
        Some("revokeDevice") => connection.execute(
            "UPDATE photo_sync_devices SET status='revoked',revoked_at=?1 WHERE id=?2",
            params![
                Utc::now().to_rfc3339(),
                body.get("deviceId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            ],
        ),
        Some("retryProcessing") => connection.execute(
            "UPDATE photos SET processing_status='completed',processing_error=NULL WHERE id=?1",
            [body
                .get("photoId")
                .and_then(Value::as_str)
                .unwrap_or_default()],
        ),
        _ => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "UNSUPPORTED_ACTION",
                "不支持的照片同步操作",
            )
        }
    };
    match result {
        Ok(_) => Json(json!({ "success": true })).into_response(),
        Err(value) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            &value.to_string(),
        ),
    }
}

pub async fn media(
    State(state): State<AppState>,
    AxumPath((photo_id, kind)): AxumPath<(String, String)>,
) -> Response {
    let media = {
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        connection.query_row(
            "SELECT original_path,thumbnail_path,mime_type,original_file_name FROM photos WHERE id=?1 AND deleted_at IS NULL",
            [&photo_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, String>(3)?)),
        ).optional()
    };
    let Ok(Some((original, thumbnail, mime, file_name))) = media else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let relative = if kind == "thumbnail" {
        thumbnail.unwrap_or(original)
    } else {
        original
    };
    let path = state.data_dir.join("photos").join(relative);
    match fs::read(path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (
                    header::CONTENT_TYPE,
                    if kind == "thumbnail" {
                        "image/jpeg".to_owned()
                    } else {
                        mime.unwrap_or_else(|| "application/octet-stream".to_owned())
                    },
                ),
                (
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", file_name.replace('"', "")),
                ),
                (header::CACHE_CONTROL, "private, max-age=86400".to_owned()),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn authenticate(
    connection: &Connection,
    headers: &HeaderMap,
) -> Result<(String, String), Response> {
    let Some(token) = bearer(headers) else {
        return Err(json_error(
            StatusCode::UNAUTHORIZED,
            "DEVICE_TOKEN_INVALID",
            "设备令牌无效",
        ));
    };
    let hashed = hash_token(token);
    let device = connection
        .query_row(
            "SELECT id,device_name,status FROM photo_sync_devices WHERE token_hash=?1",
            [&hashed],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| {
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "设备验证失败",
            )
        })?;
    let Some((device_id, name, status)) = device else {
        return Err(json_error(
            StatusCode::UNAUTHORIZED,
            "DEVICE_TOKEN_INVALID",
            "设备令牌无效",
        ));
    };
    if status != "active" {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "DEVICE_REVOKED",
            "设备授权已撤销",
        ));
    }
    connection
        .execute(
            "UPDATE photo_sync_devices SET last_seen_at=?1 WHERE id=?2",
            params![Utc::now().to_rfc3339(), device_id],
        )
        .ok();
    Ok((device_id, name))
}

fn clean_name(value: &str) -> String {
    let file_name = Path::new(value)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("photo");
    file_name
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect()
}

async fn shortcut_entry(runtime: &Runtime, code: &str) -> Response {
    let pairing = runtime
        .pairings
        .lock()
        .ok()
        .and_then(|pairings| pairings.get(code).cloned());
    let Some(pairing) = pairing.filter(|pairing| pairing.expires_at > Utc::now()) else {
        return json_error(
            StatusCode::BAD_REQUEST,
            "PAIR_CODE_INVALID",
            "配对码无效或已经过期",
        );
    };
    let config = json!({
        "action": "pair", "server": pairing.server, "pairCode": code,
        "stableServer": pairing.server, "expiresAt": pairing.expires_at.to_rfc3339()
    });
    let shortcut = format!(
        "shortcuts://run-shortcut?name={}&input=text&text={}",
        url::form_urlencoded::byte_serialize("LifeTrace照片同步".as_bytes()).collect::<String>(),
        url::form_urlencoded::byte_serialize(config.to_string().as_bytes()).collect::<String>()
    );
    Html(format!(
        r#"<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width">
        <title>LifeTrace 照片同步</title><style>body{{font-family:system-ui;text-align:center;padding:48px 20px;background:#f4f7f5;color:#173d30}}a{{display:inline-block;padding:14px 22px;border-radius:12px;background:#1f6b4f;color:white;text-decoration:none;font-weight:700}}</style>
        <h1>LifeTrace 照片同步</h1><p>打开快捷指令完成设备配对。</p><a href="{shortcut}">打开快捷指令</a>"#
    )).into_response()
}

// Embedded so the phone only downloads a small upload UI; all parsing remains on the PC.
const MOBILE_UPLOAD_PAGE: &str = include_str!("mobile-upload.html");

fn is_mobile_upload_route(method: &Method, path: &str) -> bool {
    (*method == Method::GET && matches!(path, "/" | "/fitness" | "/api/health"))
        || (*method == Method::POST && matches!(path, "/api/imports" | "/api/xunji/parse"))
}

async fn proxy_mobile_upload(request: Request<Body>) -> Response {
    let (parts, body) = request.into_parts();
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or(parts.uri.path());
    let url = format!("http://127.0.0.1:3103{path_and_query}");
    let mut outgoing = reqwest::Client::new().request(parts.method, url);
    for (name, value) in &parts.headers {
        if !matches!(name.as_str(), "host" | "content-length" | "connection") {
            outgoing = outgoing.header(name, value);
        }
    }
    let upstream = match outgoing
        .body(reqwest::Body::wrap_stream(body.into_data_stream()))
        .send()
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                "UPLOAD_SERVICE_UNAVAILABLE",
                &format!("电脑上传服务暂时不可用：{error}"),
            )
        }
    };
    let status = upstream.status();
    let headers = upstream.headers().clone();
    let mut response = Response::builder().status(status);
    for (name, value) in &headers {
        if !matches!(
            name.as_str(),
            "connection" | "content-length" | "transfer-encoding"
        ) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|_| {
            json_error(
                StatusCode::BAD_GATEWAY,
                "UPLOAD_RESPONSE_INVALID",
                "电脑上传服务返回了无效响应",
            )
        })
}

#[axum::debug_handler]
async fn lan_dispatch(State(state): State<AppState>, request: Request<Body>) -> Response {
    if !state.photo_runtime.active.load(Ordering::Relaxed) {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "PHOTO_SYNC_DISABLED",
            "局域网上传功能尚未开启",
        );
    }
    let request_method = request.method().clone();
    let request_path = request.uri().path().to_owned();
    if request_method == Method::GET && matches!(request_path.as_str(), "/" | "/fitness") {
        return Html(MOBILE_UPLOAD_PAGE).into_response();
    }
    if request_method == Method::GET && request_path == "/api/health" {
        return Json(json!({
            "ok": true,
            "service": "lifetrace-upload",
            "runtime": "tauri-rust",
            "checkedAt": Utc::now().to_rfc3339()
        }))
        .into_response();
    }
    if request_method == Method::POST
        && matches!(request_path.as_str(), "/api/imports" | "/api/xunji/parse")
    {
        return proxy_mobile_upload(request).await;
    }
    let (parts, body) = request.into_parts();
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;
    let path = uri.path();
    if method == Method::GET && path == "/api/photo-sync/shortcut-entry" {
        let params: HashMap<String, String> =
            url::form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
                .into_owned()
                .collect();
        return shortcut_entry(
            &state.photo_runtime,
            params.get("code").map(String::as_str).unwrap_or_default(),
        )
        .await;
    }
    if method == Method::POST && path == "/api/photo-sync/pair" {
        let payload = match to_bytes(body, 128 * 1024).await.and_then(|bytes| {
            serde_json::from_slice::<Value>(&bytes).map_err(|error| axum::Error::new(error))
        }) {
            Ok(value) => value,
            Err(_) => {
                return json_error(StatusCode::BAD_REQUEST, "PAIR_CODE_INVALID", "配对请求无效")
            }
        };
        let code = payload
            .get("pairCode")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let pairing = state
            .photo_runtime
            .pairings
            .lock()
            .ok()
            .and_then(|mut pairings| pairings.remove(code));
        if pairing.is_none_or(|pairing| pairing.expires_at <= Utc::now()) {
            return json_error(
                StatusCode::BAD_REQUEST,
                "PAIR_CODE_INVALID",
                "配对码无效、已使用或已过期",
            );
        }
        let device_uuid = payload
            .get("deviceId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if device_uuid.len() < 8 || device_uuid.len() > 100 {
            return json_error(
                StatusCode::BAD_REQUEST,
                "PAIR_CODE_INVALID",
                "设备标识格式无效",
            );
        }
        let token =
            Uuid::new_v4().as_simple().to_string() + &Uuid::new_v4().as_simple().to_string();
        let device_id = format!("device_{}", Uuid::new_v4());
        let stamp = Utc::now().to_rfc3339();
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "数据库暂时不可用",
                )
            }
        };
        let existing: Option<String> = connection
            .query_row(
                "SELECT id FROM photo_sync_devices WHERE device_uuid=?1",
                [device_uuid],
                |row| row.get(0),
            )
            .optional()
            .unwrap_or(None);
        let stored_id = existing.unwrap_or(device_id);
        let result = connection.execute(
            "INSERT INTO photo_sync_devices(id,device_name,device_type,device_uuid,token_hash,status,paired_at,last_seen_at)
             VALUES(?1,?2,'iphone',?3,?4,'active',?5,?5)
             ON CONFLICT(device_uuid) DO UPDATE SET device_name=excluded.device_name,token_hash=excluded.token_hash,status='active',paired_at=excluded.paired_at,last_seen_at=excluded.last_seen_at,revoked_at=NULL",
            params![
                stored_id,
                payload.get("deviceName").and_then(Value::as_str).unwrap_or("iPhone").chars().take(100).collect::<String>(),
                device_uuid,
                hash_token(&token),
                stamp
            ],
        );
        return match result {
            Ok(_) => Json(json!({
                "success": true,
                "serverName": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "LifeTrace-PC".to_owned()),
                "deviceToken": token, "deviceId": stored_id
            })).into_response(),
            Err(value) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", &value.to_string()),
        };
    }

    let (device_id, _) = {
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "数据库暂时不可用",
                )
            }
        };
        match authenticate(&connection, &headers) {
            Ok(value) => value,
            Err(response) => return response,
        }
    };
    if method == Method::GET && path == "/api/photo-sync/health" {
        return Json(json!({
            "success": true, "serverName": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "LifeTrace-PC".to_owned()),
            "serverVersion": "2.0.0-rust", "photoSyncEnabled": true, "currentTime": Utc::now().to_rfc3339()
        })).into_response();
    }
    if method == Method::POST && path == "/api/photo-sync/assets" {
        let payload = match to_bytes(body, 128 * 1024).await.and_then(|bytes| {
            serde_json::from_slice::<Value>(&bytes).map_err(|error| axum::Error::new(error))
        }) {
            Ok(value) => value,
            Err(_) => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "UNSUPPORTED_MEDIA_TYPE",
                    "资源信息无效",
                )
            }
        };
        let asset_id = payload
            .get("clientAssetId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let expected = payload.get("fileSize").and_then(Value::as_i64).unwrap_or(0);
        if asset_id.is_empty() || expected <= 0 || expected > 500 * 1024 * 1024 {
            return json_error(
                StatusCode::BAD_REQUEST,
                "UPLOAD_FILE_TOO_LARGE",
                "文件大小或资源编号无效",
            );
        }
        let file_name = clean_name(
            payload
                .get("fileName")
                .and_then(Value::as_str)
                .unwrap_or("photo"),
        );
        let media_type = payload
            .get("mediaType")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(media_type, "image" | "video") {
            return json_error(
                StatusCode::BAD_REQUEST,
                "UNSUPPORTED_MEDIA_TYPE",
                "不支持此媒体类型",
            );
        }
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "数据库暂时不可用",
                )
            }
        };
        let existing: Option<String> = connection.query_row(
            "SELECT photo_id FROM photo_device_assets WHERE device_id=?1 AND client_asset_id=?2",
            params![device_id, asset_id], |row| row.get(0),
        ).optional().unwrap_or(None);
        if let Some(photo_id) = existing {
            return Json(json!({ "success": true, "alreadyExists": true, "photoId": photo_id }))
                .into_response();
        }
        let upload_id = format!("upl_{}", Uuid::new_v4());
        let stamp = Utc::now();
        let temporary = format!(".upload/{upload_id}.part");
        let result = connection.execute(
            "INSERT INTO photo_upload_tasks(id,device_id,client_asset_id,original_file_name,media_type,mime_type,captured_at,expected_file_size,received_file_size,temporary_path,status,created_at,updated_at,expires_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,0,?9,'created',?10,?10,?11)
             ON CONFLICT(device_id,client_asset_id) DO UPDATE SET status='created',received_file_size=0,updated_at=excluded.updated_at,expires_at=excluded.expires_at",
            params![upload_id, device_id, asset_id, file_name, media_type, payload.get("mimeType").and_then(Value::as_str), payload.get("capturedAt").and_then(Value::as_str), expected, temporary, stamp.to_rfc3339(), (stamp + Duration::hours(24)).to_rfc3339()],
        );
        return match result {
            Ok(_) => {
                Json(json!({ "success": true, "uploadId": upload_id, "alreadyExists": false }))
                    .into_response()
            }
            Err(value) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                &value.to_string(),
            ),
        };
    }

    let segments = path.trim_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() == 5 && segments[..3] == ["api", "photo-sync", "assets"] {
        let upload_id = segments[3];
        if method == Method::PUT && segments[4] == "content" {
            return upload_content(&state, &device_id, upload_id, body).await;
        }
        if method == Method::POST && segments[4] == "complete" {
            return complete_upload(&state, &device_id, upload_id).await;
        }
    }
    json_error(
        StatusCode::NOT_FOUND,
        "UPLOAD_NOT_FOUND",
        "照片同步接口不存在",
    )
}

async fn upload_content(
    state: &AppState,
    device_id: &str,
    upload_id: &str,
    body: Body,
) -> Response {
    let task = {
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "数据库暂时不可用",
                )
            }
        };
        connection.query_row(
            "SELECT device_id,expected_file_size,temporary_path,status FROM photo_upload_tasks WHERE id=?1",
            [upload_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
        ).optional()
    };
    let Ok(Some((owner, expected, temporary, status))) = task else {
        return json_error(StatusCode::NOT_FOUND, "UPLOAD_NOT_FOUND", "上传任务不存在");
    };
    if owner != device_id {
        return json_error(
            StatusCode::FORBIDDEN,
            "UPLOAD_NOT_OWNED",
            "上传任务不属于当前设备",
        );
    }
    if matches!(status.as_str(), "completed" | "processing") {
        return json_error(
            StatusCode::CONFLICT,
            "UPLOAD_ALREADY_COMPLETED",
            "上传任务已经完成",
        );
    }
    let bytes = match to_bytes(body, (expected as usize).min(500 * 1024 * 1024) + 1).await {
        Ok(value) if value.len() as i64 <= expected => value,
        _ => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "UPLOAD_FILE_TOO_LARGE",
                "上传内容超过任务限制",
            )
        }
    };
    let path = state.data_dir.join("photos").join(&temporary);
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).await.is_err() {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERVER_STORAGE_ERROR",
                "无法创建上传目录",
            );
        }
    }
    if fs::write(&path, &bytes).await.is_err() {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "SERVER_STORAGE_ERROR",
            "无法保存上传内容",
        );
    }
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "数据库暂时不可用",
            )
        }
    };
    connection.execute(
        "UPDATE photo_upload_tasks SET status='uploaded',received_file_size=?1,updated_at=?2,error_code=NULL,error_message=NULL WHERE id=?3",
        params![bytes.len() as i64, Utc::now().to_rfc3339(), upload_id],
    ).ok();
    Json(json!({ "success": true, "uploadId": upload_id, "receivedFileSize": bytes.len() }))
        .into_response()
}

async fn complete_upload(state: &AppState, device_id: &str, upload_id: &str) -> Response {
    let task = {
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "数据库暂时不可用",
                )
            }
        };
        connection.query_row(
            "SELECT device_id,client_asset_id,original_file_name,media_type,mime_type,captured_at,expected_file_size,received_file_size,temporary_path,status,photo_id FROM photo_upload_tasks WHERE id=?1",
            [upload_id],
            |row| Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                row.get::<_, String>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?, row.get::<_, i64>(7)?, row.get::<_, String>(8)?,
                row.get::<_, String>(9)?, row.get::<_, Option<String>>(10)?,
            )),
        ).optional()
    };
    let Ok(Some((
        owner,
        asset_id,
        original_name,
        media_type,
        mime_type,
        captured_at,
        expected,
        received,
        temporary,
        status,
        previous_photo,
    ))) = task
    else {
        return json_error(StatusCode::NOT_FOUND, "UPLOAD_NOT_FOUND", "上传任务不存在");
    };
    if owner != device_id {
        return json_error(
            StatusCode::FORBIDDEN,
            "UPLOAD_NOT_OWNED",
            "上传任务不属于当前设备",
        );
    }
    if status == "completed" {
        return Json(json!({ "success": true, "duplicate": true, "photoId": previous_photo, "processingStatus": "completed" })).into_response();
    }
    if status != "uploaded" || expected != received {
        return json_error(
            StatusCode::BAD_REQUEST,
            "UPLOAD_SIZE_MISMATCH",
            "文件尚未完整上传",
        );
    }
    let root = state.data_dir.join("photos");
    let temporary_path = root.join(&temporary);
    let bytes = match fs::read(&temporary_path).await {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERVER_STORAGE_ERROR",
                "上传临时文件不存在",
            )
        }
    };
    let content_hash = format!("{:x}", Sha256::digest(&bytes));
    let duplicate = {
        let connection = match state.database.lock() {
            Ok(value) => value,
            Err(_) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "数据库暂时不可用",
                )
            }
        };
        connection.query_row(
            "SELECT id,processing_status FROM photos WHERE content_hash=?1 AND deleted_at IS NULL",
            [&content_hash], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ).optional().unwrap_or(None)
    };
    if let Some((photo_id, processing_status)) = duplicate {
        {
            let connection = match state.database.lock() {
                Ok(value) => value,
                Err(_) => {
                    return json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "DATABASE_ERROR",
                        "数据库暂时不可用",
                    )
                }
            };
            connection.execute(
            "INSERT INTO photo_device_assets(device_id,client_asset_id,photo_id,synced_at) VALUES(?1,?2,?3,?4)
             ON CONFLICT(device_id,client_asset_id) DO UPDATE SET photo_id=excluded.photo_id,synced_at=excluded.synced_at",
            params![device_id, asset_id, photo_id, Utc::now().to_rfc3339()],
        ).ok();
            connection.execute(
            "UPDATE photo_upload_tasks SET status='completed',photo_id=?1,is_duplicate=1,updated_at=?2 WHERE id=?3",
            params![photo_id, Utc::now().to_rfc3339(), upload_id],
        ).ok();
            drop(connection);
        }
        fs::remove_file(temporary_path).await.ok();
        return Json(json!({ "success": true, "duplicate": true, "photoId": photo_id, "processingStatus": processing_status })).into_response();
    }

    let extension = Path::new(&original_name)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| value.len() <= 8)
        .unwrap_or(if media_type == "video" { "mp4" } else { "jpg" })
        .to_lowercase();
    let photo_id = format!("photo_{}", Uuid::new_v4());
    let stored_name = format!("{photo_id}.{extension}");
    let originals = root.join("originals");
    if fs::create_dir_all(&originals).await.is_err() {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "SERVER_STORAGE_ERROR",
            "无法创建照片目录",
        );
    }
    let original_path = originals.join(&stored_name);
    if fs::rename(&temporary_path, &original_path).await.is_err() {
        if fs::copy(&temporary_path, &original_path).await.is_err() {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERVER_STORAGE_ERROR",
                "无法归档照片",
            );
        }
        fs::remove_file(&temporary_path).await.ok();
    }

    let mut width = None;
    let mut height = None;
    let mut thumbnail_relative = None;
    if media_type == "image" {
        if let Ok(image) = image::load_from_memory(&bytes) {
            width = Some(image.width() as i64);
            height = Some(image.height() as i64);
            let thumbnail = image.thumbnail(640, 640);
            let thumb_dir = root.join("thumbnails");
            fs::create_dir_all(&thumb_dir).await.ok();
            let thumb_name = format!("{photo_id}.jpg");
            let thumb_path = thumb_dir.join(&thumb_name);
            if thumbnail
                .save_with_format(&thumb_path, image::ImageFormat::Jpeg)
                .is_ok()
            {
                thumbnail_relative = Some(format!("thumbnails/{thumb_name}"));
            }
        }
    }
    let original_relative = format!("originals/{stored_name}");
    let imported_at = Utc::now().to_rfc3339();
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "数据库暂时不可用",
            )
        }
    };
    let transaction = match connection.unchecked_transaction() {
        Ok(value) => value,
        Err(value) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                &value.to_string(),
            )
        }
    };
    let result = (|| -> rusqlite::Result<()> {
        transaction.execute(
            "INSERT INTO photos(id,content_hash,original_file_name,stored_file_name,original_path,thumbnail_path,media_type,mime_type,file_size,width,height,captured_at,imported_at,processing_status,source_device_id)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'completed',?14)",
            params![photo_id, content_hash, original_name, stored_name, original_relative, thumbnail_relative, media_type, mime_type, received, width, height, captured_at, imported_at, device_id],
        )?;
        transaction.execute(
            "INSERT INTO photo_device_assets(device_id,client_asset_id,photo_id,synced_at) VALUES(?1,?2,?3,?4)",
            params![device_id, asset_id, photo_id, imported_at],
        )?;
        transaction.execute(
            "UPDATE photo_upload_tasks SET status='completed',photo_id=?1,updated_at=?2,error_code=NULL,error_message=NULL WHERE id=?3",
            params![photo_id, imported_at, upload_id],
        )?;
        transaction.commit()
    })();
    match result {
        Ok(_) => Json(json!({ "success": true, "duplicate": false, "photoId": photo_id, "processingStatus": "completed" })).into_response(),
        Err(value) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", &value.to_string()),
    }
}

pub async fn serve_lan(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(state.data_dir.join("photos").join(".upload")).await?;
    let (certificate, key) = state
        .photo_runtime
        .certificate_files()
        .map_err(std::io::Error::other)?;
    let app = Router::new().fallback(any(lan_dispatch)).with_state(state);
    let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(certificate, key).await?;
    axum_server::bind_rustls(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3443)), tls)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

#[axum::debug_handler]
async fn compatibility_dispatch(State(state): State<AppState>, request: Request<Body>) -> Response {
    let mobile_upload_route = is_mobile_upload_route(request.method(), request.uri().path());
    if !mobile_upload_route
        && !state
            .photo_runtime
            .allow_insecure_http
            .load(Ordering::Relaxed)
    {
        return json_error(
            StatusCode::FORBIDDEN,
            "HTTPS_REQUIRED",
            "HTTP 兼容模式尚未开启",
        );
    }
    lan_dispatch(State(state), request).await
}

pub async fn serve_compatibility(
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .fallback(any(compatibility_dispatch))
        .with_state(state);
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3445))).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn serve_media(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .route("/photo-sync/media/{photo_id}/{kind}", get(media))
        .with_state(state);
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 3444))).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
