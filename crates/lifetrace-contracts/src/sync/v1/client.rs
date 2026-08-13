//! Sync client identification.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::DeviceId;

/// Application id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppId(String);

impl AppId {
    pub const DESKTOP: &'static str = "lifetrace-desktop";
    pub const FINANCE_ANDROID: &'static str = "lifetrace-finance-android";
    pub const NOTES_ANDROID: &'static str = "lifetrace-notes-android";
    pub const ENGLISH_ANDROID: &'static str = "lifetrace-english-android";
    pub const HABITS_ANDROID: &'static str = "lifetrace-habits-android";
    /// Unmodified BeeCount Flutter/iOS/Android clients through the compatibility facade.
    pub const BEECOUNT: &'static str = "beecount-mobile";
    pub const WEB: &'static str = "lifetrace-web";

    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for AppId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AppId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl JsonSchema for AppId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("AppId")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = String::json_schema(generator);
        if let Some(object) = schema.as_object_mut() {
            object.insert(
                "description".to_owned(),
                serde_json::Value::String(
                    "Client application id. Known values: lifetrace-desktop, lifetrace-web, lifetrace-*-android."
                        .to_owned(),
                ),
            );
        }
        schema
    }
}

impl ts_rs::TS for AppId {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type AppId = string;".to_owned()
    }

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String {
        "AppId".to_owned()
    }

    fn visit_dependencies(_visitor: &mut impl ts_rs::TypeVisitor) {}

    fn inline() -> String {
        "string".to_owned()
    }

    fn inline_flattened() -> String {
        Self::inline()
    }

    fn output_path() -> Option<std::path::PathBuf> {
        None
    }
}

/// Client platform.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientPlatform(String);

impl ClientPlatform {
    pub const WINDOWS: &'static str = "windows";
    pub const ANDROID: &'static str = "android";
    pub const WEB: &'static str = "web";

    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClientPlatform {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ClientPlatform {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ClientPlatform {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl JsonSchema for ClientPlatform {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("ClientPlatform")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = String::json_schema(generator);
        if let Some(object) = schema.as_object_mut() {
            object.insert(
                "description".to_owned(),
                serde_json::Value::String(
                    "Client platform. Known values: windows, android, web. Unknown values are preserved."
                        .to_owned(),
                ),
            );
        }
        schema
    }
}

impl ts_rs::TS for ClientPlatform {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type ClientPlatform = string;".to_owned()
    }

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String {
        "ClientPlatform".to_owned()
    }

    fn visit_dependencies(_visitor: &mut impl ts_rs::TypeVisitor) {}

    fn inline() -> String {
        "string".to_owned()
    }

    fn inline_flattened() -> String {
        Self::inline()
    }

    fn output_path() -> Option<std::path::PathBuf> {
        None
    }
}

/// Sync client info carried by every request body (headers may also carry it;
/// the server validates consistency).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SyncClientInfo {
    pub app_id: AppId,
    /// Client app release version (for example `0.2.1`).
    pub client_version: String,
    pub platform: ClientPlatform,
    pub protocol_version: u32,
    pub schema_version: u32,
    pub device_id: DeviceId,
}
