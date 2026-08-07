//! Signed snapshot page tokens.
//!
//! Tokens bind `userId | snapshotId | offset` with HMAC-SHA256, so clients
//! cannot skip pages or reuse tokens across users/snapshots.

use axum::http::StatusCode;
use base64::Engine;
use lifetrace_contracts::{ErrorCode, SnapshotId, UserId};
use subtle::ConstantTimeEq;

use crate::error::ApiError;
use crate::sync::cursor_codec::hmac_sha256;

#[derive(Debug, Clone)]
pub struct PageTokenCodec {
    key: Vec<u8>,
}

impl PageTokenCodec {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into().into_bytes(),
        }
    }

    pub fn encode(&self, user_id: &UserId, snapshot_id: &SnapshotId, offset: usize) -> String {
        let payload = format!("v1|{user_id}|{snapshot_id}|{offset}");
        let signature = hex::encode(hmac_sha256(&self.key, payload.as_bytes()));
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());
        format!("{encoded}.{signature}")
    }

    pub fn decode(
        &self,
        token: &str,
        user_id: &UserId,
        snapshot_id: &SnapshotId,
    ) -> Result<usize, ApiError> {
        let Some((encoded, signature)) = token.rsplit_once('.') else {
            return Err(token_invalid());
        };
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| token_invalid())?;
        let expected_signature = hex::encode(hmac_sha256(&self.key, &payload));
        if !bool::from(expected_signature.as_bytes().ct_eq(signature.as_bytes())) {
            return Err(token_invalid());
        }
        let text = String::from_utf8(payload).map_err(|_| token_invalid())?;
        let parts: Vec<&str> = text.split('|').collect();
        if parts.len() != 4
            || parts[0] != "v1"
            || parts[1] != user_id.as_str()
            || parts[2] != snapshot_id.as_str()
        {
            return Err(token_invalid());
        }
        parts[3].parse::<usize>().map_err(|_| token_invalid())
    }
}

fn token_invalid() -> ApiError {
    ApiError::new(
        ErrorCode::CursorInvalid,
        "invalid page token",
        StatusCode::BAD_REQUEST,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_token_round_trips_and_binds() {
        let codec = PageTokenCodec::new("test-key");
        let user = UserId::new("user-a");
        let snapshot = SnapshotId::new("snapshot-1");
        let token = codec.encode(&user, &snapshot, 25);
        assert_eq!(codec.decode(&token, &user, &snapshot).unwrap(), 25);
        assert!(codec
            .decode(&token, &UserId::new("user-b"), &snapshot)
            .is_err());
        assert!(codec
            .decode(&token, &user, &SnapshotId::new("snapshot-2"))
            .is_err());
    }
}
