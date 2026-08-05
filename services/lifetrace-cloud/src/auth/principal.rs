//! Authenticated principal.

use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::{DeviceId, UserId};

/// The authenticated caller for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub app_id: AppId,
}
