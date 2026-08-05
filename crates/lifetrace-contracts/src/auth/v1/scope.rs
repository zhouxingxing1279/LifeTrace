use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One OAuth-style authorization scope.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
#[schemars(with = "String")]
#[ts(type = "string")]
pub struct Scope(pub String);

impl Scope {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Scope collection returned by authentication endpoints.
pub type ScopeSet = Vec<Scope>;
