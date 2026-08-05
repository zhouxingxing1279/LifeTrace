use super::AppInstallationId;
use crate::sync::v1::AppId;
use crate::UtcTimestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DeviceInstallationV1 {
    pub id: AppInstallationId,
    pub external_device_id: String,
    pub device_group_id: Option<String>,
    pub device_name: String,
    pub app_id: AppId,
    pub platform: String,
    pub status: String,
    pub client_version: Option<String>,
    pub first_seen_at: UtcTimestamp,
    pub last_seen_at: UtcTimestamp,
    pub last_login_at: Option<UtcTimestamp>,
    pub last_sync_at: Option<UtcTimestamp>,
    pub revoked_at: Option<UtcTimestamp>,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DeviceListV1 {
    pub devices: Vec<DeviceInstallationV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UpdateDeviceRequestV1 {
    pub device_name: String,
}
