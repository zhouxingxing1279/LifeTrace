use std::collections::BTreeSet;

use async_trait::async_trait;
use subtle::ConstantTimeEq;

use crate::auth::{scope, AuthCredential, AuthMethod, AuthProvider, AuthenticatedPrincipal};
use crate::error::ApiError;
use lifetrace_contracts::auth::v1::{AppInstallationId, AuthSessionId};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::{ErrorCode, UserId};

#[derive(Debug, Clone)]
pub struct DevelopmentAuthProvider {
    enabled: bool,
    token: String,
    user_id: UserId,
    device_id: AppInstallationId,
    app_id: AppId,
}

impl DevelopmentAuthProvider {
    pub fn new(
        enabled: bool,
        token: String,
        user_id: UserId,
        device_id: impl Into<String>,
    ) -> Self {
        Self {
            enabled,
            token,
            user_id,
            device_id: AppInstallationId::new(device_id),
            app_id: AppId::new(AppId::DESKTOP),
        }
    }

    fn bearer_token(credential: AuthCredential<'_>) -> Option<&str> {
        match credential {
            AuthCredential::Bearer(value) => value
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            AuthCredential::WebSession(_) => None,
        }
    }
}

#[async_trait]
impl AuthProvider for DevelopmentAuthProvider {
    async fn authenticate(
        &self,
        credential: AuthCredential<'_>,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        use axum::http::StatusCode;
        if !self.enabled {
            return Err(ApiError::new(
                ErrorCode::AuthRequired,
                "authentication is not configured",
                StatusCode::UNAUTHORIZED,
            ));
        }
        let provided = Self::bearer_token(credential).ok_or_else(|| {
            ApiError::new(
                ErrorCode::AuthRequired,
                "missing Bearer token",
                StatusCode::UNAUTHORIZED,
            )
        })?;
        let valid = self.token.len() == provided.len()
            && bool::from(self.token.as_bytes().ct_eq(provided.as_bytes()));
        if !valid {
            return Err(ApiError::new(
                ErrorCode::AuthInvalid,
                "invalid token",
                StatusCode::UNAUTHORIZED,
            ));
        }
        Ok(AuthenticatedPrincipal {
            user_id: self.user_id.clone(),
            session_id: AuthSessionId::new("development-session"),
            device_id: self.device_id.clone(),
            app_id: self.app_id.clone(),
            scopes: scope::ALL_SCOPES
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<BTreeSet<_>>(),
            auth_method: AuthMethod::Development,
        })
    }
}
