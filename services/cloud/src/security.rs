//! EPIC-17 shared HTTP security policy and safe diagnostic redaction.

use axum::http::{header::HeaderName, HeaderValue};
use serde_json::Value;
use tower_http::set_header::SetResponseHeaderLayer;

pub const CONTENT_SECURITY_POLICY: &str =
    "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'";
pub const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=()";
pub const STRICT_TRANSPORT_SECURITY: &str = "max-age=63072000; includeSubDomains";

/// Apply headers that are safe for every API response. LifeTrace Cloud is an
/// API service; it does not serve arbitrary script content, so the CSP can be
/// deliberately restrictive.
pub fn response_security_layers() -> Vec<SetResponseHeaderLayer<HeaderValue>> {
    vec![
        SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ),
        SetResponseHeaderLayer::overriding(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ),
        SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ),
        SetResponseHeaderLayer::overriding(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(PERMISSIONS_POLICY),
        ),
        SetResponseHeaderLayer::overriding(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(CONTENT_SECURITY_POLICY),
        ),
        SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-store"),
        ),
    ]
}

pub fn hsts_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static(STRICT_TRANSPORT_SECURITY),
    )
}

fn sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '_'], "");
    normalized.contains("password")
        || normalized.contains("authorization")
        || normalized == "cookie"
        || normalized == "setcookie"
        || normalized.contains("accesstoken")
        || normalized.contains("refreshtoken")
        || normalized.contains("csrftoken")
        || normalized.contains("tokenhash")
        || normalized.contains("apikey")
        || normalized.contains("secret")
        || normalized.contains("credentialciphertext")
}

/// Return a diagnostic-safe copy of structured metadata. This is intended for
/// audit/error metadata before it is written to logs; the original value is
/// never mutated.
pub fn redact_sensitive_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let safe = if sensitive_key(key) {
                        Value::String("[REDACTED]".to_owned())
                    } else {
                        redact_sensitive_json(value)
                    };
                    (key.clone(), safe)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_sensitive_json).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_nested_authentication_secrets() {
        let safe = redact_sensitive_json(&json!({
            "authorization": "Bearer top-secret",
            "profile": {
                "password": "secret-password",
                "access_token": "access",
                "displayName": "Alice"
            },
            "items": [{"apiKey": "key"}]
        }));
        assert_eq!(safe["authorization"], "[REDACTED]");
        assert_eq!(safe["profile"]["password"], "[REDACTED]");
        assert_eq!(safe["profile"]["access_token"], "[REDACTED]");
        assert_eq!(safe["profile"]["displayName"], "Alice");
        assert_eq!(safe["items"][0]["apiKey"], "[REDACTED]");
        assert!(!safe.to_string().contains("top-secret"));
        assert!(!safe.to_string().contains("secret-password"));
    }
}
