use std::sync::Arc;

use async_trait::async_trait;
use lifetrace_contracts::auth::v1::{RefreshRequestV1, TokenResponseV1};
use lifetrace_contracts::error::ApiErrorV1;
use lifetrace_contracts::sync::v1::{
    AppId, CapabilitiesResponseV1, PullRequestV1, PullResponseV1, PushRequestV1, PushResponseV1,
    SnapshotRequestV1, SnapshotResponseV1,
};
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};

use lifetrace_sync_client::{FailureClass, SyncError, SyncTransport};

use crate::cloud_auth;

#[derive(Debug, Clone, Default)]
pub struct AuthContext {
    pub origin: String,
    pub access_token: Option<String>,
    pub cloud_user_id: Option<String>,
    pub device_id: String,
}

#[derive(Clone)]
pub struct HttpSyncTransport {
    client: Client,
    auth: Arc<RwLock<AuthContext>>,
    refresh_flight: Arc<Mutex<()>>,
}

impl HttpSyncTransport {
    pub fn new(auth: Arc<RwLock<AuthContext>>) -> Result<Self, String> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            auth,
            refresh_flight: Arc::new(Mutex::new(())),
        })
    }

    pub async fn clear_session(&self) {
        let mut auth = self.auth.write().await;
        auth.access_token = None;
        auth.cloud_user_id = None;
    }

    async fn url(&self, path: &str) -> Result<String, SyncError> {
        let auth = self.auth.read().await;
        if auth.origin.is_empty() {
            return Err(SyncError::new(
                "SYNC_CLOUD_ORIGIN_MISSING",
                "cloud origin is not configured",
                FailureClass::AuthRequired,
            ));
        }
        Ok(format!("{}{}", auth.origin, path))
    }

    async fn send_json<B: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        authenticated: bool,
    ) -> Result<R, SyncError> {
        let url = self.url(path).await?;
        let execute = |token: Option<String>| {
            let mut request = self.client.post(&url).json(body);
            if let Some(token) = token {
                request = request.bearer_auth(token);
            }
            request
        };
        let token = if authenticated {
            self.auth.read().await.access_token.clone()
        } else {
            None
        };
        if authenticated && token.is_none() {
            self.refresh().await?;
        }
        let token = if authenticated {
            self.auth.read().await.access_token.clone()
        } else {
            None
        };
        let mut response = execute(token).send().await.map_err(Self::network_error)?;
        if authenticated && response.status() == StatusCode::UNAUTHORIZED {
            self.auth.write().await.access_token = None;
            self.refresh().await?;
            response = execute(self.auth.read().await.access_token.clone())
                .send()
                .await
                .map_err(Self::network_error)?;
        }
        Self::parse(response).await
    }

    async fn capabilities_get(&self) -> Result<CapabilitiesResponseV1, SyncError> {
        let response = self
            .client
            .get(self.url("/api/v1/sync/capabilities").await?)
            .send()
            .await
            .map_err(Self::network_error)?;
        Self::parse(response).await
    }

    async fn refresh(&self) -> Result<(), SyncError> {
        let _guard = self.refresh_flight.lock().await;
        // Another waiter may already have refreshed while this caller waited.
        if self.auth.read().await.access_token.is_some() {
            return Ok(());
        }
        let refresh_token = cloud_auth::credential_get_internal()
            .map_err(|message| {
                SyncError::new("SYNC_CREDENTIAL_READ", message, FailureClass::AuthRequired)
            })?
            .ok_or_else(|| {
                SyncError::new(
                    "SYNC_REFRESH_TOKEN_MISSING",
                    "refresh token is unavailable",
                    FailureClass::AuthRequired,
                )
            })?;
        let auth = self.auth.read().await.clone();
        let response = self
            .client
            .post(format!("{}/api/v1/auth/refresh", auth.origin))
            .json(&RefreshRequestV1 {
                refresh_token,
                app_id: AppId::new(AppId::DESKTOP),
                device_id: auth.device_id,
            })
            .send()
            .await
            .map_err(Self::network_error)?;
        if response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::FORBIDDEN
        {
            let _ = cloud_auth::credential_clear_internal();
            self.clear_session().await;
            return Err(SyncError::new(
                "SYNC_AUTH_REQUIRED",
                "cloud session expired or was revoked",
                FailureClass::AuthRequired,
            ));
        }
        let tokens: TokenResponseV1 = Self::parse(response).await?;
        if let Some(refresh) = &tokens.refresh_token {
            cloud_auth::credential_set_internal(refresh).map_err(|message| {
                SyncError::new("SYNC_CREDENTIAL_WRITE", message, FailureClass::Permanent)
            })?;
        }
        let mut current = self.auth.write().await;
        current.access_token = Some(tokens.access_token);
        current.cloud_user_id = Some(tokens.user.id.as_str().to_owned());
        Ok(())
    }

    fn network_error(error: reqwest::Error) -> SyncError {
        if error.is_connect() || error.is_timeout() {
            SyncError::new("SYNC_OFFLINE", error.to_string(), FailureClass::Offline)
        } else {
            SyncError::new("SYNC_NETWORK", error.to_string(), FailureClass::Transient)
        }
    }

    async fn parse<R: DeserializeOwned>(response: Response) -> Result<R, SyncError> {
        let status = response.status();
        if status.is_success() {
            return response.json().await.map_err(|error| {
                SyncError::new(
                    "SYNC_RESPONSE_INVALID",
                    error.to_string(),
                    FailureClass::Permanent,
                )
            });
        }
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let payload = response.json::<ApiErrorV1>().await.ok();
        let code = payload
            .as_ref()
            .map(|value| value.code.wire_name().to_owned())
            .unwrap_or_else(|| format!("HTTP_{}", status.as_u16()));
        let message = payload
            .map(|value| value.message)
            .unwrap_or_else(|| status.to_string());
        let class = match status.as_u16() {
            401 => FailureClass::AuthRequired,
            403 => FailureClass::PermissionDenied,
            413 => FailureClass::PayloadTooLarge,
            426 => FailureClass::UpgradeRequired,
            429 => FailureClass::RateLimited {
                retry_after_seconds: retry_after,
            },
            500..=599 => FailureClass::Transient,
            _ => FailureClass::Permanent,
        };
        Err(SyncError::new(code, message, class))
    }
}

#[async_trait]
impl SyncTransport for HttpSyncTransport {
    async fn capabilities(&self) -> Result<CapabilitiesResponseV1, SyncError> {
        self.capabilities_get().await
    }
    async fn push(&self, request: PushRequestV1) -> Result<PushResponseV1, SyncError> {
        self.send_json("/api/v1/sync/push", &request, true).await
    }
    async fn pull(&self, request: PullRequestV1) -> Result<PullResponseV1, SyncError> {
        self.send_json("/api/v1/sync/pull", &request, true).await
    }
    async fn snapshot(&self, request: SnapshotRequestV1) -> Result<SnapshotResponseV1, SyncError> {
        self.send_json("/api/v1/sync/snapshot", &request, true)
            .await
    }
}
