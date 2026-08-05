//! Money representation.
//!
//! Money is integer cents on the wire (`amountCents`), never floating point.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

/// ISO 4217 currency code (for example `CNY`, `USD`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema, TS)]
#[schemars(with = "String")]
#[ts(type = "string")]
pub struct CurrencyCode(String);

impl Serialize for CurrencyCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CurrencyCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl CurrencyCode {
    /// Build a currency code; must be 3 uppercase ASCII letters.
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let raw = value.into();
        if raw.len() != 3 || !raw.chars().all(|character| character.is_ascii_uppercase()) {
            return Err(format!(
                "invalid currency code (expected 3 uppercase letters): {raw}"
            ));
        }
        Ok(Self(raw))
    }

    /// The default Chinese Yuan code.
    pub fn cny() -> Self {
        Self("CNY".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A monetary amount: integer cents plus currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MoneyAmount {
    /// Amount in the smallest currency unit (cents). Never use floats.
    pub amount_cents: i64,
    pub currency: CurrencyCode,
}

impl MoneyAmount {
    pub fn new(amount_cents: i64, currency: CurrencyCode) -> Self {
        Self {
            amount_cents,
            currency,
        }
    }

    pub fn cny(amount_cents: i64) -> Self {
        Self {
            amount_cents,
            currency: CurrencyCode::cny(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_uses_amount_cents_on_the_wire() {
        let money = MoneyAmount::cny(3250);
        let json = serde_json::to_string(&money).unwrap();
        assert_eq!(json, r#"{"amountCents":3250,"currency":"CNY"}"#);
        let back: MoneyAmount = serde_json::from_str(&json).unwrap();
        assert_eq!(back, money);
    }

    #[test]
    fn currency_code_is_validated() {
        assert!(CurrencyCode::new("CNY").is_ok());
        assert!(CurrencyCode::new("cny").is_err());
        assert!(CurrencyCode::new("CN").is_err());
    }
}
