//! Development auth provider.
//!
//! Uses a fixed Bearer token mapped to a fixed user/device. Hard rules:
//! - Disabled in production (`LIFETRACE_ENV=production` + DEV_AUTH => startup
//!   fails).
//! - The token is compared in constant time and never logged.

use subtle::ConstantTimeEq;

use crate::auth::{AuthenticatedPrincipal, AuthProvider};
use crate::error::ApiError;
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::{DeviceId, ErrorCode, UserId};

#[derive(Debug, Clone)]
pub struct DevelopmentAuthProvider {
    enabled: bool,
    token: String,
    user_id: UserId,
    device_id: DeviceId,
    app_id: AppId,
}

impl DevelopmentAuthProvider {
    pub fn new(
        enabled: bool,
        token: String,
        user_id: UserId,
        device_id: DeviceId,
    ) -> Self {
        Self {
            enabled,
            token,
            user_id,
            device_id,
            app_id: AppId::new(AppId::DESKTOP),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn bearer_token(authorization: Option<&str>) -> Option<&str> {
        authorization
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}

impl AuthProvider for DevelopmentAuthProvider {
    fn authenticate(
        &self,
        authorization: Option<&str>,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        if !self.enabled {
            return Err(ApiError::new(
                ErrorCode::AuthRequired,
                "authentication is not configured",
                axum::http::StatusCode::UNAUTHORIZED,
            ));
        }
        let provided = Self::bearer_token(authorization).ok_or_else(|| {
            ApiError::new(
                ErrorCode::AuthRequired,
                "missing Bearer token",
                axum::http::StatusCode::UNAUTHORIZED,
            )
        })?;
        let expected = self.token.as_bytes();
        let actual = provided.as_bytes();
        let valid = expected.len() == actual.len()
            && bool::from(expected.ct_eq(actual));
        if !valid {
            return Err(ApiError::new(
                ErrorCode::AuthInvalid,
                "invalid token",
                axum::http::StatusCode::UNAUTHORIZED,
            ));
        }
        Ok(AuthenticatedPrincipal {
            user_id: self.user_id.clone(),
            device_id: self.device_id.clone(),
            app_id: self.app_id.clone(),
        })
    }
}
