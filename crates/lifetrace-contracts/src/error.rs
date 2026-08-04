//! Stable error codes and the uniform API error body.
//!
//! Error codes are published contract: once released their meaning MUST NOT
//! change. Unknown codes never fail a whole batch parse; they are preserved
//! as `Unknown(String)`.

use std::{borrow::Cow, fmt, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use crate::ids::RequestId;
use crate::json_value::JsonValue;

/// Stable protocol / API error codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    ProtocolUnsupported,
    SchemaUnsupported,
    ClientTooOld,
    AppIdUnsupported,
    AuthRequired,
    AuthInvalid,
    DeviceNotRegistered,
    DeviceRevoked,
    InvalidRequest,
    BatchTooLarge,
    PayloadTooLarge,
    UnknownEntityType,
    InvalidEntityPayload,
    DependencyMissing,
    ChangeIdReuse,
    BaseVersionMismatch,
    CursorInvalid,
    CursorExpired,
    SnapshotRequired,
    AtomicGroupFailed,
    RateLimited,
    TemporarilyUnavailable,
    InternalError,
    /// Forward-compatible: unknown code received from a newer server.
    Unknown(String),
}

impl ErrorCode {
    /// Wire string for a known code.
    pub fn wire_name(&self) -> &str {
        match self {
            ErrorCode::ProtocolUnsupported => "LIFETRACE_PROTOCOL_UNSUPPORTED",
            ErrorCode::SchemaUnsupported => "LIFETRACE_SCHEMA_UNSUPPORTED",
            ErrorCode::ClientTooOld => "LIFETRACE_CLIENT_TOO_OLD",
            ErrorCode::AppIdUnsupported => "LIFETRACE_APP_ID_UNSUPPORTED",
            ErrorCode::AuthRequired => "LIFETRACE_AUTH_REQUIRED",
            ErrorCode::AuthInvalid => "LIFETRACE_AUTH_INVALID",
            ErrorCode::DeviceNotRegistered => "LIFETRACE_DEVICE_NOT_REGISTERED",
            ErrorCode::DeviceRevoked => "LIFETRACE_DEVICE_REVOKED",
            ErrorCode::InvalidRequest => "LIFETRACE_INVALID_REQUEST",
            ErrorCode::BatchTooLarge => "LIFETRACE_BATCH_TOO_LARGE",
            ErrorCode::PayloadTooLarge => "LIFETRACE_PAYLOAD_TOO_LARGE",
            ErrorCode::UnknownEntityType => "LIFETRACE_UNKNOWN_ENTITY_TYPE",
            ErrorCode::InvalidEntityPayload => "LIFETRACE_INVALID_ENTITY_PAYLOAD",
            ErrorCode::DependencyMissing => "LIFETRACE_DEPENDENCY_MISSING",
            ErrorCode::ChangeIdReuse => "LIFETRACE_CHANGE_ID_REUSE",
            ErrorCode::BaseVersionMismatch => "LIFETRACE_BASE_VERSION_MISMATCH",
            ErrorCode::CursorInvalid => "LIFETRACE_CURSOR_INVALID",
            ErrorCode::CursorExpired => "LIFETRACE_CURSOR_EXPIRED",
            ErrorCode::SnapshotRequired => "LIFETRACE_SNAPSHOT_REQUIRED",
            ErrorCode::AtomicGroupFailed => "LIFETRACE_ATOMIC_GROUP_FAILED",
            ErrorCode::RateLimited => "LIFETRACE_RATE_LIMITED",
            ErrorCode::TemporarilyUnavailable => "LIFETRACE_TEMPORARILY_UNAVAILABLE",
            ErrorCode::InternalError => "LIFETRACE_INTERNAL_ERROR",
            ErrorCode::Unknown(value) => value,
        }
    }

    /// Map a wire string to a code; unknown strings are preserved.
    pub fn from_wire(value: &str) -> Self {
        match value {
            "LIFETRACE_PROTOCOL_UNSUPPORTED" => ErrorCode::ProtocolUnsupported,
            "LIFETRACE_SCHEMA_UNSUPPORTED" => ErrorCode::SchemaUnsupported,
            "LIFETRACE_CLIENT_TOO_OLD" => ErrorCode::ClientTooOld,
            "LIFETRACE_APP_ID_UNSUPPORTED" => ErrorCode::AppIdUnsupported,
            "LIFETRACE_AUTH_REQUIRED" => ErrorCode::AuthRequired,
            "LIFETRACE_AUTH_INVALID" => ErrorCode::AuthInvalid,
            "LIFETRACE_DEVICE_NOT_REGISTERED" => ErrorCode::DeviceNotRegistered,
            "LIFETRACE_DEVICE_REVOKED" => ErrorCode::DeviceRevoked,
            "LIFETRACE_INVALID_REQUEST" => ErrorCode::InvalidRequest,
            "LIFETRACE_BATCH_TOO_LARGE" => ErrorCode::BatchTooLarge,
            "LIFETRACE_PAYLOAD_TOO_LARGE" => ErrorCode::PayloadTooLarge,
            "LIFETRACE_UNKNOWN_ENTITY_TYPE" => ErrorCode::UnknownEntityType,
            "LIFETRACE_INVALID_ENTITY_PAYLOAD" => ErrorCode::InvalidEntityPayload,
            "LIFETRACE_DEPENDENCY_MISSING" => ErrorCode::DependencyMissing,
            "LIFETRACE_CHANGE_ID_REUSE" => ErrorCode::ChangeIdReuse,
            "LIFETRACE_BASE_VERSION_MISMATCH" => ErrorCode::BaseVersionMismatch,
            "LIFETRACE_CURSOR_INVALID" => ErrorCode::CursorInvalid,
            "LIFETRACE_CURSOR_EXPIRED" => ErrorCode::CursorExpired,
            "LIFETRACE_SNAPSHOT_REQUIRED" => ErrorCode::SnapshotRequired,
            "LIFETRACE_ATOMIC_GROUP_FAILED" => ErrorCode::AtomicGroupFailed,
            "LIFETRACE_RATE_LIMITED" => ErrorCode::RateLimited,
            "LIFETRACE_TEMPORARILY_UNAVAILABLE" => ErrorCode::TemporarilyUnavailable,
            "LIFETRACE_INTERNAL_ERROR" => ErrorCode::InternalError,
            other => ErrorCode::Unknown(other.to_owned()),
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.wire_name())
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(ErrorCode::from_wire(&raw))
    }
}

/// Field-level validation error inside `ApiErrorV1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FieldError {
    /// Field path, for example `changes[3].entityType`.
    pub field: String,
    /// Field-level error code (string; server-defined, forward compatible).
    pub code: String,
    pub message: String,
}

/// Uniform API error body returned by sync endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApiErrorV1 {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: Option<RequestId>,
    pub retryable: bool,
    #[serde(default)]
    pub field_errors: Vec<FieldError>,
    pub details: Option<JsonValue>,
}

impl JsonSchema for ErrorCode {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("ErrorCode")
    }

    fn json_schema(
        generator: &mut schemars::SchemaGenerator,
    ) -> schemars::Schema {
        let mut schema = String::json_schema(generator);
        if let Some(object) = schema.as_object_mut() {
            object.insert(
                "description".to_owned(),
                serde_json::Value::String(
                    "Stable error code. Unknown codes from newer servers are preserved verbatim."
                        .to_owned(),
                ),
            );
            object.insert(
                "enum".to_owned(),
                serde_json::Value::Array(
                    [
                        "LIFETRACE_PROTOCOL_UNSUPPORTED",
                        "LIFETRACE_SCHEMA_UNSUPPORTED",
                        "LIFETRACE_CLIENT_TOO_OLD",
                        "LIFETRACE_APP_ID_UNSUPPORTED",
                        "LIFETRACE_AUTH_REQUIRED",
                        "LIFETRACE_AUTH_INVALID",
                        "LIFETRACE_DEVICE_NOT_REGISTERED",
                        "LIFETRACE_DEVICE_REVOKED",
                        "LIFETRACE_INVALID_REQUEST",
                        "LIFETRACE_BATCH_TOO_LARGE",
                        "LIFETRACE_PAYLOAD_TOO_LARGE",
                        "LIFETRACE_UNKNOWN_ENTITY_TYPE",
                        "LIFETRACE_INVALID_ENTITY_PAYLOAD",
                        "LIFETRACE_DEPENDENCY_MISSING",
                        "LIFETRACE_CHANGE_ID_REUSE",
                        "LIFETRACE_BASE_VERSION_MISMATCH",
                        "LIFETRACE_CURSOR_INVALID",
                        "LIFETRACE_CURSOR_EXPIRED",
                        "LIFETRACE_SNAPSHOT_REQUIRED",
                        "LIFETRACE_ATOMIC_GROUP_FAILED",
                        "LIFETRACE_RATE_LIMITED",
                        "LIFETRACE_TEMPORARILY_UNAVAILABLE",
                        "LIFETRACE_INTERNAL_ERROR",
                    ]
                    .iter()
                    .map(|value| serde_json::Value::String((*value).to_owned()))
                    .collect(),
                ),
            );
        }
        schema
    }
}

impl TS for ErrorCode {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type ErrorCode = string;".to_owned()
    }

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String {
        "ErrorCode".to_owned()
    }

    fn visit_dependencies(_visitor: &mut impl ts_rs::TypeVisitor) {}

    fn inline() -> String {
        "string".to_owned()
    }

    fn inline_flattened() -> String {
        Self::inline()
    }

    fn output_path() -> Option<PathBuf> {
        None
    }
}

impl ApiErrorV1 {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: None,
            retryable: false,
            field_errors: Vec::new(),
            details: None,
        }
    }
}

impl std::error::Error for ErrorCode {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_round_trips_known_and_unknown() {
        let known = ErrorCode::ChangeIdReuse;
        let json = serde_json::to_string(&known).unwrap();
        assert_eq!(json, "\"LIFETRACE_CHANGE_ID_REUSE\"");
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ErrorCode::ChangeIdReuse);

        let unknown_json = "\"LIFETRACE_NEW_CODE_99\"";
        let unknown: ErrorCode = serde_json::from_str(unknown_json).unwrap();
        assert_eq!(
            unknown,
            ErrorCode::Unknown("LIFETRACE_NEW_CODE_99".to_owned())
        );
        assert_eq!(serde_json::to_string(&unknown).unwrap(), unknown_json);
    }

    #[test]
    fn all_required_codes_exist_and_are_stable() {
        let required = [
            "LIFETRACE_PROTOCOL_UNSUPPORTED",
            "LIFETRACE_SCHEMA_UNSUPPORTED",
            "LIFETRACE_CLIENT_TOO_OLD",
            "LIFETRACE_DEVICE_NOT_REGISTERED",
            "LIFETRACE_INVALID_REQUEST",
            "LIFETRACE_BATCH_TOO_LARGE",
            "LIFETRACE_PAYLOAD_TOO_LARGE",
            "LIFETRACE_UNKNOWN_ENTITY_TYPE",
            "LIFETRACE_INVALID_ENTITY_PAYLOAD",
            "LIFETRACE_DEPENDENCY_MISSING",
            "LIFETRACE_CHANGE_ID_REUSE",
            "LIFETRACE_BASE_VERSION_MISMATCH",
            "LIFETRACE_CURSOR_INVALID",
            "LIFETRACE_CURSOR_EXPIRED",
            "LIFETRACE_SNAPSHOT_REQUIRED",
            "LIFETRACE_ATOMIC_GROUP_FAILED",
            "LIFETRACE_INTERNAL_ERROR",
        ];
        for wire in required {
            let code = ErrorCode::from_wire(wire);
            assert_eq!(code.wire_name(), wire, "wire name must round trip");
            assert!(!matches!(code, ErrorCode::Unknown(_)), "must be a known code");
        }
    }

    #[test]
    fn api_error_uses_camel_case_wire_fields() {
        let error = ApiErrorV1::new(ErrorCode::CursorExpired, "cursor expired");
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "LIFETRACE_CURSOR_EXPIRED");
        assert_eq!(json["retryable"], false);
        assert!(json["requestId"].is_null());
        assert_eq!(json["fieldErrors"], serde_json::json!([]));
    }
}
