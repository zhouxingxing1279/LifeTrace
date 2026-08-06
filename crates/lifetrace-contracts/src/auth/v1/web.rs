use super::{AuthSessionV1, AuthUserV1, Scope};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WebLoginRequestV1 {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub requested_scopes: Vec<Scope>,
    #[serde(default)]
    pub public_device: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WebRegisterRequestV1 {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    pub invite_token: Option<String>,
    #[serde(default)]
    pub requested_scopes: Vec<Scope>,
    #[serde(default)]
    pub public_device: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WebSessionResponseV1 {
    pub user: AuthUserV1,
    pub session: AuthSessionV1,
    pub csrf_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CsrfResponseV1 {
    pub csrf_token: String,
}
