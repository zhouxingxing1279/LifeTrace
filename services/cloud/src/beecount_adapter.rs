//! Read-only transport adapter for BeeCount Cloud.

use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT};
use hyper::{Method, Request, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use url::Url;

use crate::config::Config;

#[derive(Debug, thiserror::Error)]
pub enum BeeCountAdapterError {
    #[error("BeeCount adapter is not configured")]
    NotConfigured,
    #[error("BeeCount authentication failed")]
    Authentication,
    #[error("BeeCount service account requires interactive two-factor authentication")]
    TwoFactorRequired,
    #[error("BeeCount upstream is unavailable")]
    Unavailable,
    #[error("BeeCount upstream rejected the request")]
    UpstreamRejected,
    #[error("BeeCount upstream returned an invalid response")]
    InvalidResponse,
    #[error("BeeCount upstream response exceeded the configured limit")]
    ResponseTooLarge,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    #[serde(default)]
    requires_2fa: bool,
    access_token: Option<String>,
    expires_in: Option<u64>,
}

struct CachedToken {
    value: String,
    expires_at: Instant,
}

type HttpClient = Client<HttpConnector, Full<Bytes>>;

pub struct BeeCountAdapter {
    client: HttpClient,
    base_url: Url,
    email: String,
    password: String,
    bound_lifetrace_user_id: String,
    timeout: Duration,
    max_response_bytes: usize,
    token: Mutex<Option<CachedToken>>,
}

impl BeeCountAdapter {
    pub fn from_config(config: &Config) -> Result<Self, BeeCountAdapterError> {
        if !config.beecount_adapter_enabled {
            return Err(BeeCountAdapterError::NotConfigured);
        }
        let base_url = Url::parse(&config.beecount_adapter_base_url)
            .map_err(|_| BeeCountAdapterError::NotConfigured)?;
        let mut connector = HttpConnector::new();
        connector.enforce_http(true);
        let client = Client::builder(TokioExecutor::new()).build(connector);
        Ok(Self {
            client,
            base_url,
            email: config
                .beecount_adapter_email
                .clone()
                .ok_or(BeeCountAdapterError::NotConfigured)?,
            password: config
                .beecount_adapter_password
                .as_ref()
                .map(|value| value.expose().to_owned())
                .ok_or(BeeCountAdapterError::NotConfigured)?,
            bound_lifetrace_user_id: config
                .beecount_adapter_lifetrace_user_id
                .clone()
                .ok_or(BeeCountAdapterError::NotConfigured)?,
            timeout: Duration::from_secs(config.beecount_adapter_timeout_seconds),
            max_response_bytes: config.beecount_adapter_max_response_bytes,
            token: Mutex::new(None),
        })
    }

    pub fn bound_lifetrace_user_id(&self) -> &str {
        &self.bound_lifetrace_user_id
    }

    pub async fn version(&self) -> Result<Value, BeeCountAdapterError> {
        self.request_public(Method::GET, "api/v1/version", &[])
            .await
    }

    pub async fn ledgers(&self) -> Result<Value, BeeCountAdapterError> {
        self.request_authenticated(Method::GET, "api/v1/read/ledgers", &[])
            .await
    }

    pub async fn ledger_snapshot(
        &self,
        ledger_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<RawBeeCountSnapshot, BeeCountAdapterError> {
        let ledger_path = self.ledger_path(ledger_id, "")?;
        let transactions_path = self.ledger_path(ledger_id, "/transactions")?;
        let budgets_path = self.ledger_path(ledger_id, "/budgets")?;
        let query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
        let ledger_query = vec![("ledger_id", ledger_id.to_owned())];

        let (ledger, transactions, accounts, categories, tags, budgets) = tokio::try_join!(
            self.request_authenticated(Method::GET, &ledger_path, &[]),
            self.request_authenticated(Method::GET, &transactions_path, &query),
            self.request_authenticated(
                Method::GET,
                "api/v1/read/workspace/accounts",
                &ledger_query,
            ),
            self.request_authenticated(
                Method::GET,
                "api/v1/read/workspace/categories",
                &ledger_query,
            ),
            self.request_authenticated(Method::GET, "api/v1/read/workspace/tags", &ledger_query,),
            self.request_authenticated(Method::GET, &budgets_path, &[]),
        )?;

        Ok(RawBeeCountSnapshot {
            ledger,
            transactions,
            accounts,
            categories,
            tags,
            budgets,
        })
    }

    fn ledger_path(&self, ledger_id: &str, suffix: &str) -> Result<String, BeeCountAdapterError> {
        let mut url = self.base_url.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| BeeCountAdapterError::InvalidResponse)?;
            segments.clear();
            for segment in ["api", "v1", "read", "ledgers", ledger_id] {
                segments.push(segment);
            }
            if !suffix.is_empty() {
                segments.push(suffix.trim_start_matches('/'));
            }
        }
        Ok(url.path().trim_start_matches('/').to_owned())
    }

    async fn request_authenticated(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<Value, BeeCountAdapterError> {
        let token = self.access_token().await?;
        let first = self
            .send(method.clone(), path, query, Some(&token), None)
            .await?;
        if first.0 != StatusCode::UNAUTHORIZED {
            return self.decode(first.0, first.1);
        }

        self.invalidate_token(&token).await;
        let token = self.access_token().await?;
        let second = self.send(method, path, query, Some(&token), None).await?;
        self.decode(second.0, second.1)
    }

    async fn request_public(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<Value, BeeCountAdapterError> {
        let response = self.send(method, path, query, None, None).await?;
        self.decode(response.0, response.1)
    }

    async fn access_token(&self) -> Result<String, BeeCountAdapterError> {
        // Keep the lock while logging in so concurrent snapshot requests share one attempt.
        let mut cached = self.token.lock().await;
        if let Some(token) = cached.as_ref() {
            if token.expires_at > Instant::now() + Duration::from_secs(30) {
                return Ok(token.value.clone());
            }
        }

        let body = serde_json::to_vec(&json!({
            "email": &self.email,
            "password": &self.password,
            "device_id": "lifetrace-finance-adapter",
            "device_name": "LifeTrace Finance Adapter",
            "platform": "server",
            "app_version": "0.1",
            "client_type": "web"
        }))
        .map_err(|_| BeeCountAdapterError::InvalidResponse)?;
        let (status, bytes) = self
            .send(Method::POST, "api/v1/auth/login", &[], None, Some(body))
            .await?;
        if !status.is_success() {
            return Err(BeeCountAdapterError::Authentication);
        }
        let login: LoginResponse =
            serde_json::from_slice(&bytes).map_err(|_| BeeCountAdapterError::InvalidResponse)?;
        if login.requires_2fa {
            return Err(BeeCountAdapterError::TwoFactorRequired);
        }
        let value = login
            .access_token
            .filter(|value| !value.is_empty())
            .ok_or(BeeCountAdapterError::InvalidResponse)?;
        let lifetime = login.expires_in.unwrap_or(300).max(60);
        *cached = Some(CachedToken {
            value: value.clone(),
            expires_at: Instant::now() + Duration::from_secs(lifetime),
        });
        Ok(value)
    }

    async fn invalidate_token(&self, rejected: &str) {
        let mut cached = self.token.lock().await;
        if cached
            .as_ref()
            .is_some_and(|current| current.value == rejected)
        {
            *cached = None;
        }
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        bearer: Option<&str>,
        json_body: Option<Vec<u8>>,
    ) -> Result<(StatusCode, Vec<u8>), BeeCountAdapterError> {
        let mut url = self.url(path)?;
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }
        let mut request = Request::builder()
            .method(method)
            .uri(url.as_str())
            .header(USER_AGENT, "LifeTrace-BeeCount-Read-Adapter/0.1");
        if let Some(token) = bearer {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match json_body {
            Some(bytes) => {
                request = request.header(CONTENT_TYPE, "application/json");
                Full::new(Bytes::from(bytes))
            }
            None => Full::new(Bytes::new()),
        };
        let request = request
            .body(body)
            .map_err(|_| BeeCountAdapterError::InvalidResponse)?;
        let operation = async {
            let response = self
                .client
                .request(request)
                .await
                .map_err(|_| BeeCountAdapterError::Unavailable)?;
            let status = response.status();
            let bytes = self.read_limited(response).await?;
            Ok((status, bytes))
        };
        tokio::time::timeout(self.timeout, operation)
            .await
            .map_err(|_| BeeCountAdapterError::Unavailable)?
    }

    async fn read_limited(
        &self,
        response: hyper::Response<hyper::body::Incoming>,
    ) -> Result<Vec<u8>, BeeCountAdapterError> {
        if response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok())
            .is_some_and(|length| length > self.max_response_bytes)
        {
            return Err(BeeCountAdapterError::ResponseTooLarge);
        }
        let mut body = response.into_body();
        let mut output = Vec::new();
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| BeeCountAdapterError::Unavailable)?;
            if let Some(chunk) = frame.data_ref() {
                let new_length = output
                    .len()
                    .checked_add(chunk.len())
                    .ok_or(BeeCountAdapterError::ResponseTooLarge)?;
                if new_length > self.max_response_bytes {
                    return Err(BeeCountAdapterError::ResponseTooLarge);
                }
                output.extend_from_slice(chunk);
            }
        }
        Ok(output)
    }

    fn decode(&self, status: StatusCode, bytes: Vec<u8>) -> Result<Value, BeeCountAdapterError> {
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(BeeCountAdapterError::Authentication);
        }
        if !status.is_success() {
            return Err(BeeCountAdapterError::UpstreamRejected);
        }
        serde_json::from_slice(&bytes).map_err(|_| BeeCountAdapterError::InvalidResponse)
    }

    fn url(&self, path: &str) -> Result<Url, BeeCountAdapterError> {
        self.base_url
            .join(path.trim_start_matches('/'))
            .map_err(|_| BeeCountAdapterError::InvalidResponse)
    }
}

pub struct RawBeeCountSnapshot {
    pub ledger: Value,
    pub transactions: Value,
    pub accounts: Value,
    pub categories: Value,
    pub tags: Value,
    pub budgets: Value,
}
