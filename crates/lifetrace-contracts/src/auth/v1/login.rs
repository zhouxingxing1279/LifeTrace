use super::Scope;
use crate::sync::v1::AppId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LoginRequestV1 {
    pub email: String,
    pub password: String,
    pub app_id: AppId,
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub client_version: Option<String>,
    #[serde(default)]
    pub requested_scopes: Vec<Scope>,
    #[serde(default)]
    pub public_device: bool,
}
