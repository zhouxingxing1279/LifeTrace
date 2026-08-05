//! Fixed test auth provider (used by integration tests).

use crate::auth::{AuthenticatedPrincipal, AuthProvider};
use crate::error::ApiError;
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::{DeviceId, UserId};

/// Always authenticates to the configured principal. Not for production.
#[derive(Debug, Clone)]
pub struct TestAuthProvider {
    principal: AuthenticatedPrincipal,
}

impl TestAuthProvider {
    pub fn new(user_id: &str, device_id: &str) -> Self {
        Self {
            principal: AuthenticatedPrincipal {
                user_id: UserId::new(user_id),
                device_id: DeviceId::new(device_id),
                app_id: AppId::new(AppId::DESKTOP),
            },
        }
    }

    pub fn principal(&self) -> &AuthenticatedPrincipal {
        &self.principal
    }
}

impl AuthProvider for TestAuthProvider {
    fn authenticate(
        &self,
        authorization: Option<&str>,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        match authorization {
            Some(value) if value.starts_with("Bearer ") => Ok(self.principal.clone()),
            _ => Err(ApiError::new(
                lifetrace_contracts::ErrorCode::AuthRequired,
                "authentication required",
                axum::http::StatusCode::UNAUTHORIZED,
            )),
        }
    }
}
