//! Shared ID and version value objects.
//!
//! All IDs are strings on the wire. New IDs SHOULD be UUID v4, but legacy
//! non-UUID IDs (for example `piano`, `wechat-wallet`, `xunji-...`) MUST be
//! preserved verbatim; the newtypes therefore wrap `String`, not `Uuid`.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

macro_rules! string_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema, TS)]
        #[schemars(with = "String")]
        #[ts(type = "string")]
        pub struct $name(pub String);

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Ok(Self(raw))
            }
        }

        impl $name {
            /// Build an ID from any non-empty string.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the raw string representation.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

string_id!(UserId, "Stable user identifier (string, usually UUID v4).");
string_id!(DeviceId, "Stable device identifier (string, usually UUID v4).");
string_id!(
    EntityId,
    "Entity identifier. New IDs are UUID v4; legacy non-UUID IDs are preserved verbatim."
);
string_id!(
    ChangeId,
    "Idempotency key for a sync change: userId + changeId must be unique per payload."
);
string_id!(
    RequestId,
    "Request tracing id. It never replaces changeId for idempotency."
);
string_id!(
    ConflictId,
    "Stable id assigned by the server to a returned conflict."
);
string_id!(
    AtomicGroupId,
    "Groups changes that must be applied all-or-nothing in one push request."
);
string_id!(
    SnapshotId,
    "Stable id assigned by the server to a snapshot; all pages share one consistent view."
);

/// Opaque server cursor. Clients must never parse, add, subtract or guess it.
/// Wire format: string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema, TS)]
#[schemars(with = "String")]
#[ts(type = "string")]
pub struct Cursor(pub String);

impl Serialize for Cursor {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Cursor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl Cursor {
    /// Build a cursor from any non-empty opaque string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Server-authoritative entity version. Wire format: decimal string
/// (for example `"42"`) to avoid JavaScript safe-integer issues.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema, TS)]
#[schemars(with = "String")]
#[ts(type = "string")]
pub struct ServerVersion(pub String);

impl Serialize for ServerVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ServerVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl ServerVersion {
    /// Build a server version from a decimal string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Version `"0"` used by clients for brand-new local entities that have
    /// never been acknowledged by the server.
    pub fn zero() -> Self {
        Self("0".to_owned())
    }

    /// Build a server version from a `u64`.
    pub fn from_u64(value: u64) -> Self {
        Self(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse the decimal value (used by the reference implementation only;
    /// clients treat versions as opaque strings).
    pub fn to_u64(&self) -> Option<u64> {
        self.0.parse().ok()
    }
}

impl fmt::Display for ServerVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<u64> for ServerVersion {
    fn from(value: u64) -> Self {
        Self::from_u64(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_as_strings() {
        let id = UserId::new("local-user");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"local-user\"");
        let back: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn legacy_non_uuid_entity_ids_are_preserved() {
        let entity = EntityId::new("wechat-wallet");
        let json = serde_json::to_string(&entity).unwrap();
        assert_eq!(json, "\"wechat-wallet\"");
        let back: EntityId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "wechat-wallet");
    }

    #[test]
    fn server_version_is_a_wire_string() {
        let version = ServerVersion::from_u64(42);
        let json = serde_json::to_string(&version).unwrap();
        assert_eq!(json, "\"42\"");
        assert_eq!(version.to_u64(), Some(42));
        assert_eq!(ServerVersion::zero().to_u64(), Some(0));
    }

    #[test]
    fn cursor_is_an_opaque_wire_string() {
        let cursor = Cursor::new("10591");
        let json = serde_json::to_string(&cursor).unwrap();
        assert_eq!(json, "\"10591\"");
        let back: Cursor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "10591");
    }
}
