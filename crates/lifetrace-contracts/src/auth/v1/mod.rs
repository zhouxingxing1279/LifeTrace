//! LifeTrace authentication protocol v1.

mod account;
mod app_grant;
mod device;
mod login;
mod password;
mod scope;
mod session;
mod token;
mod web;

pub use account::*;
pub use app_grant::*;
pub use device::*;
pub use login::*;
pub use password::*;
pub use scope::*;
pub use session::*;
pub use token::*;
pub use web::*;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use ts_rs::TS;

macro_rules! auth_string_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema, TS)]
        #[schemars(with = "String")]
        #[ts(type = "string")]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Ok(Self(String::deserialize(deserializer)?))
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

auth_string_id!(
    AuthSessionId,
    "Server-side authentication session identifier."
);
auth_string_id!(AppGrantId, "User-to-application grant identifier.");
auth_string_id!(
    AppInstallationId,
    "Application installation/device identifier."
);
auth_string_id!(TokenFamilyId, "Refresh-token family identifier.");
