use std::collections::BTreeSet;

use lifetrace_contracts::auth::v1::{AppInstallationId, AuthSessionId};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::UserId;

use crate::auth::scope;
use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    Development,
    AccessToken,
    WebSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub user_id: UserId,
    pub session_id: AuthSessionId,
    pub device_id: AppInstallationId,
    pub app_id: AppId,
    pub scopes: BTreeSet<String>,
    pub auth_method: AuthMethod,
}

impl AuthenticatedPrincipal {
    pub fn require_scope(&self, required: &str) -> Result<(), ApiError> {
        scope::require(&self.scopes, required)
    }
}
