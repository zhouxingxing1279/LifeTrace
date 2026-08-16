use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::{redirect::Policy, Method};
use serde::{Deserialize, Serialize};
use tauri::State;
use url::Url;

use crate::sync::SyncDesktopState;

const MAX_API_REQUEST_BYTES: usize = 8 * 1024 * 1024;
const MAX_API_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_QUERY_BYTES: usize = 16 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudApiHttpRequest {
    path: String,
    query: Option<String>,
    method: String,
    body: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudApiHttpResponse {
    status: u16,
    body: String,
    content_type: Option<String>,
}

fn api_url(origin: &str, path: &str, query: Option<&str>) -> Result<Url, String> {
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
    if !path.starts_with("/api/v1/") || path.contains(['?', '#']) {
        return Err("桌面云 API 只允许访问 LifeTrace /api/v1/ 路径".to_owned());
    }
    if path.split('/').any(|segment| segment == "..") {
        return Err("桌面云 API 路径无效".to_owned());
    }
    if query.is_some_and(|value| value.len() > MAX_QUERY_BYTES || value.contains('#')) {
        return Err("桌面云 API 查询参数无效或过长".to_owned());
    }
    url.set_path(path);
    url.set_query(query.filter(|value| !value.is_empty()));
    Ok(url)
}

fn api_method(value: &str) -> Result<Method, String> {
    match value.to_ascii_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "PATCH" => Ok(Method::PATCH),
        "DELETE" => Ok(Method::DELETE),
        _ => Err("桌面云 API 只允许 GET/POST/PUT/PATCH/DELETE".to_owned()),
    }
}

#[tauri::command]
pub async fn cloud_api_http_request(
    state: State<'_, SyncDesktopState>,
    request: CloudApiHttpRequest,
) -> Result<CloudApiHttpResponse, String> {
    if request
        .body
        .as_ref()
        .is_some_and(|body| body.len() > MAX_API_REQUEST_BYTES)
    {
        return Err("桌面云 API 请求体超过安全上限".to_owned());
    }

    let (origin, access_token) = {
        let auth = state.auth.read().await;
        let token = auth
            .access_token
            .clone()
            .ok_or_else(|| "请先登录 LifeTrace 云端".to_owned())?;
        if auth.origin.is_empty() {
            return Err("云服务尚未配置".to_owned());
        }
        (auth.origin.clone(), token)
    };

    let url = api_url(&origin, &request.path, request.query.as_deref())?;
    let method = api_method(&request.method)?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(45))
        .redirect(Policy::none())
        .build()
        .map_err(|error| format!("无法初始化云端 API 客户端: {error}"))?;

    let mut builder = client
        .request(method, url)
        .header(ACCEPT, "application/json")
        .bearer_auth(access_token);
    if let Some(body) = request.body {
        builder = builder.header(CONTENT_TYPE, "application/json").body(body);
    }

    let response = builder
        .send()
        .await
        .map_err(|error| format!("无法连接 LifeTrace 云端 API: {error}"))?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if response
        .content_length()
        .is_some_and(|length| length > MAX_API_RESPONSE_BYTES as u64)
    {
        return Err("桌面云 API 响应超过安全上限".to_owned());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("读取 LifeTrace 云端 API 响应失败: {error}"))?;
    if bytes.len() > MAX_API_RESPONSE_BYTES {
        return Err("桌面云 API 响应超过安全上限".to_owned());
    }
    let body = String::from_utf8(bytes.to_vec())
        .map_err(|_| "桌面云 API 当前只接受 UTF-8 文本/JSON 响应".to_owned())?;

    Ok(CloudApiHttpResponse {
        status,
        body,
        content_type,
    })
}

#[cfg(test)]
mod tests {
    use super::{api_method, api_url};

    #[test]
    fn api_url_stays_on_configured_origin_and_v1_namespace() {
        let url = api_url(
            "https://life.example",
            "/api/v1/integrations/beecount/ledgers",
            Some("limit=20&offset=0"),
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://life.example/api/v1/integrations/beecount/ledgers?limit=20&offset=0"
        );
        assert!(api_url(
            "https://life.example",
            "https://evil.example/api/v1/x",
            None
        )
        .is_err());
        assert!(api_url("https://life.example", "/health", None).is_err());
        assert!(api_url("https://life.example", "/api/v1/../../secret", None).is_err());
    }

    #[test]
    fn api_method_rejects_non_application_verbs() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
            assert!(api_method(method).is_ok());
        }
        assert!(api_method("CONNECT").is_err());
        assert!(api_method("TRACE").is_err());
    }
}
