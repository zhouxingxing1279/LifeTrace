use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{redirect::Policy, Method};
use serde::{Deserialize, Serialize};
use url::Url;

const CREDENTIAL_TARGET: &str = "LifeTrace/cloud/lifetrace-desktop/refresh-token";
const MAX_AUTH_RESPONSE_BYTES: usize = 1024 * 1024;
const AUTH_PATHS: &[&str] = &[
    "/api/v1/auth/capabilities",
    "/api/v1/auth/login",
    "/api/v1/auth/register",
    "/api/v1/auth/password/forgot",
    "/api/v1/auth/password/change",
    "/api/v1/auth/refresh",
    "/api/v1/auth/logout",
    "/api/v1/auth/logout-all",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAuthHttpRequest {
    origin: String,
    path: String,
    method: String,
    body: Option<String>,
    authorization: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAuthHttpResponse {
    status: u16,
    body: String,
}

fn auth_url(origin: &str, path: &str) -> Result<Url, String> {
    let mut url = Url::parse(origin.trim()).map_err(|_| "云服务地址格式无效".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("云服务地址必须是有效的 HTTP 或 HTTPS 地址".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("云服务地址不能包含用户名或密码".to_owned());
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err("云服务地址只能填写服务器根地址".to_owned());
    }
    if !AUTH_PATHS.contains(&path) {
        return Err("桌面端拒绝访问未授权的云认证路径".to_owned());
    }
    url.set_path(path);
    Ok(url)
}

#[tauri::command]
pub async fn cloud_auth_http_request(
    request: CloudAuthHttpRequest,
) -> Result<CloudAuthHttpResponse, String> {
    let url = auth_url(&request.origin, &request.path)?;
    let method = match request.method.to_ascii_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        _ => return Err("桌面端云认证只允许 GET 或 POST".to_owned()),
    };
    if request.path == "/api/v1/auth/capabilities" && method != Method::GET {
        return Err("云认证 capabilities 只允许 GET".to_owned());
    }
    if request.path != "/api/v1/auth/capabilities" && method != Method::POST {
        return Err("该云认证接口只允许 POST".to_owned());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none())
        .build()
        .map_err(|error| format!("无法初始化云端网络客户端: {error}"))?;
    let mut builder = client
        .request(method, url)
        .header(ACCEPT, "application/json");
    if let Some(body) = request.body {
        if body.len() > 64 * 1024 {
            return Err("云认证请求体超过安全上限".to_owned());
        }
        builder = builder.header(CONTENT_TYPE, "application/json").body(body);
    }
    if let Some(authorization) = request.authorization {
        if authorization.len() > 8192 || !authorization.starts_with("Bearer ") {
            return Err("云认证 Authorization 头无效".to_owned());
        }
        builder = builder.header(AUTHORIZATION, authorization);
    }

    let response = builder
        .send()
        .await
        .map_err(|error| format!("无法连接 LifeTrace 云端: {error}"))?;
    let status = response.status().as_u16();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_AUTH_RESPONSE_BYTES as u64)
    {
        return Err("云认证响应超过安全上限".to_owned());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("读取云认证响应失败: {error}"))?;
    if bytes.len() > MAX_AUTH_RESPONSE_BYTES {
        return Err("云认证响应超过安全上限".to_owned());
    }
    let body =
        String::from_utf8(bytes.to_vec()).map_err(|_| "云认证响应不是有效 UTF-8".to_owned())?;
    Ok(CloudAuthHttpResponse { status, body })
}

pub(crate) fn credential_set_internal(refresh_token: &str) -> Result<(), String> {
    if refresh_token.is_empty() || refresh_token.len() > 4096 {
        return Err("invalid refresh token length".to_owned());
    }
    platform::set(refresh_token)
}

pub(crate) fn credential_get_internal() -> Result<Option<String>, String> {
    platform::get()
}

pub(crate) fn credential_clear_internal() -> Result<(), String> {
    platform::clear()
}

#[tauri::command]
pub fn cloud_credential_set(refresh_token: String) -> Result<(), String> {
    if refresh_token.is_empty() || refresh_token.len() > 4096 {
        return Err("invalid refresh token length".to_owned());
    }
    credential_set_internal(&refresh_token)
}

#[tauri::command]
pub fn cloud_credential_get() -> Result<Option<String>, String> {
    credential_get_internal()
}

#[tauri::command]
pub fn cloud_credential_clear() -> Result<(), String> {
    credential_clear_internal()
}

#[cfg(windows)]
mod platform {
    use std::mem::zeroed;
    use std::ptr::{null_mut, slice_from_raw_parts};

    use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    use super::CREDENTIAL_TARGET;

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn set(secret: &str) -> Result<(), String> {
        let target = wide(CREDENTIAL_TARGET);
        let username = wide("LifeTrace Desktop");
        let mut bytes = secret.as_bytes().to_vec();
        let mut credential: CREDENTIALW = unsafe { zeroed() };
        credential.Type = CRED_TYPE_GENERIC;
        credential.TargetName = target.as_ptr() as *mut u16;
        credential.CredentialBlobSize = bytes.len() as u32;
        credential.CredentialBlob = bytes.as_mut_ptr();
        credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
        credential.UserName = username.as_ptr() as *mut u16;
        let success = unsafe { CredWriteW(&credential, 0) };
        bytes.fill(0);
        if success == 0 {
            Err(format!(
                "Windows Credential Manager write failed: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(())
        }
    }

    pub fn get() -> Result<Option<String>, String> {
        let target = wide(CREDENTIAL_TARGET);
        let mut pointer: *mut CREDENTIALW = null_mut();
        let success = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut pointer) };
        if success == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(None);
            }
            return Err(format!("Windows Credential Manager read failed: {error}"));
        }
        if pointer.is_null() {
            return Ok(None);
        }
        let credential = unsafe { &*pointer };
        let blob = unsafe {
            &*slice_from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        };
        let result = String::from_utf8(blob.to_vec())
            .map(Some)
            .map_err(|_| "stored cloud credential is not valid UTF-8".to_owned());
        unsafe { CredFree(pointer.cast()) };
        result
    }

    pub fn clear() -> Result<(), String> {
        let target = wide(CREDENTIAL_TARGET);
        let success = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if success == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(());
            }
            Err(format!("Windows Credential Manager delete failed: {error}"))
        } else {
            Ok(())
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn set(_: &str) -> Result<(), String> {
        Err(
            "secure cloud credential storage is available only in the Windows desktop build"
                .to_owned(),
        )
    }

    pub fn get() -> Result<Option<String>, String> {
        Ok(None)
    }

    pub fn clear() -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::auth_url;

    #[test]
    fn credential_target_is_stable_and_contains_no_secret() {
        assert_eq!(
            super::CREDENTIAL_TARGET,
            "LifeTrace/cloud/lifetrace-desktop/refresh-token"
        );
    }

    #[test]
    fn native_auth_transport_only_accepts_server_origin_and_known_paths() {
        let url = auth_url("https://8-148-75-45.sslip.io", "/api/v1/auth/login").unwrap();
        assert_eq!(
            url.as_str(),
            "https://8-148-75-45.sslip.io/api/v1/auth/login"
        );
        assert!(auth_url("https://8-148-75-45.sslip.io/api", "/api/v1/auth/login").is_err());
        assert!(auth_url("https://8-148-75-45.sslip.io", "/api/v1/admin/users").is_err());
        assert!(auth_url("file:///tmp/lifetrace", "/api/v1/auth/login").is_err());
    }
}
