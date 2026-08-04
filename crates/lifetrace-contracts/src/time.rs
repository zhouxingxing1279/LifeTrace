//! Time representations.
//!
//! - Points in time: RFC3339 UTC (`UtcTimestamp`).
//! - Natural days: `YYYY-MM-DD` (`LocalDate`).

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

/// RFC3339 UTC timestamp (wire format, for example `2026-08-04T15:30:00Z`).
pub type UtcTimestamp = chrono::DateTime<chrono::Utc>;

/// A natural calendar day in `YYYY-MM-DD` (wire format).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema, TS)]
#[schemars(with = "String")]
#[ts(type = "string")]
pub struct LocalDate(String);

impl LocalDate {
    /// Build a local date from a `YYYY-MM-DD` string, validating the format.
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let raw = value.into();
        if chrono::NaiveDate::parse_from_str(&raw, "%Y-%m-%d").is_err() {
            return Err(format!("invalid local date (expected YYYY-MM-DD): {raw}"));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_naive(value: chrono::NaiveDate) -> Self {
        Self(value.format("%Y-%m-%d").to_string())
    }

    pub fn to_naive(&self) -> Option<chrono::NaiveDate> {
        chrono::NaiveDate::parse_from_str(&self.0, "%Y-%m-%d").ok()
    }
}

impl fmt::Display for LocalDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for LocalDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LocalDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_date_wire_format() {
        let date = LocalDate::new("2026-08-04").unwrap();
        let json = serde_json::to_string(&date).unwrap();
        assert_eq!(json, "\"2026-08-04\"");
        let back: LocalDate = serde_json::from_str(&json).unwrap();
        assert_eq!(back, date);
    }

    #[test]
    fn local_date_rejects_invalid_format() {
        assert!(LocalDate::new("2026-13-40").is_err());
        assert!(LocalDate::new("2026/08/04").is_err());
        let error = serde_json::from_str::<LocalDate>("\"2026/08/04\"");
        assert!(error.is_err());
    }

    #[test]
    fn timestamp_serializes_as_rfc3339() {
        let stamp: UtcTimestamp = "2026-08-04T15:30:00Z".parse().unwrap();
        let json = serde_json::to_string(&stamp).unwrap();
        let back: UtcTimestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(stamp, back);
    }
}
