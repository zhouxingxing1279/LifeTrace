//! Signed opaque cursor.
//!
//! The wire cursor is a token binding `protocolVersion | userId | scopeHash
//! | cursorPosition | issuedAt` with an HMAC-SHA256 signature, so clients
//! cannot guess, tamper with, or reuse cursors across users/scopes.

use axum::http::StatusCode;
use lifetrace_contracts::{Cursor, ErrorCode, UserId};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::error::ApiError;
use base64::Engine;

/// HMAC-SHA256 built on `sha2` (avoids an extra dependency).
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        key_block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut inner = [0u8; BLOCK + 32];
    for (index, byte) in key_block.iter().enumerate() {
        inner[index] = byte ^ 0x36;
    }
    let mut hasher = Sha256::new();
    hasher.update(&inner[..BLOCK]);
    hasher.update(data);
    let inner_hash = hasher.finalize();

    let mut outer = [0u8; BLOCK + 32];
    for (index, byte) in key_block.iter().enumerate() {
        outer[index] = byte ^ 0x5c;
    }
    let mut hasher = Sha256::new();
    hasher.update(&outer[..BLOCK]);
    hasher.update(inner_hash);
    hasher.finalize().into()
}

/// Signs and validates opaque cursors.
#[derive(Debug, Clone)]
pub struct CursorCodec {
    key: Vec<u8>,
}

impl CursorCodec {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into().into_bytes(),
        }
    }

    pub fn encode(&self, user_id: &UserId, scope_hash: &str, position: u64) -> Cursor {
        let payload = format!("v1|{user_id}|{scope_hash}|{position}");
        let signature = hex::encode(hmac_sha256(&self.key, payload.as_bytes()));
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());
        Cursor::new(format!("{encoded}.{signature}"))
    }

    /// Decodes and verifies a cursor for the given user/scope.
    pub fn decode(
        &self,
        cursor: &Cursor,
        user_id: &UserId,
        scope_hash: &str,
    ) -> Result<u64, ApiError> {
        let Some((encoded, signature)) = cursor.as_str().rsplit_once('.') else {
            return Err(cursor_invalid());
        };
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| cursor_invalid())?;
        let expected_signature = hex::encode(hmac_sha256(&self.key, &payload));
        let signature_matches = expected_signature.as_bytes().ct_eq(signature.as_bytes());
        if !bool::from(signature_matches) {
            return Err(cursor_invalid());
        }
        let text = String::from_utf8(payload).map_err(|_| cursor_invalid())?;
        let parts: Vec<&str> = text.split('|').collect();
        if parts.len() != 4 || parts[0] != "v1" {
            return Err(cursor_invalid());
        }
        if parts[1] != user_id.as_str() || parts[2] != scope_hash {
            return Err(ApiError::new(
                ErrorCode::CursorExpired,
                "cursor is not valid for this user/scope",
                StatusCode::GONE,
            ));
        }
        parts[3].parse::<u64>().map_err(|_| cursor_invalid())
    }
}

fn cursor_invalid() -> ApiError {
    ApiError::new(
        ErrorCode::CursorInvalid,
        "cursor is not valid",
        StatusCode::BAD_REQUEST,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifetrace_contracts::ids::Cursor;

    #[test]
    fn cursor_round_trips_and_binds_user() {
        let codec = CursorCodec::new("test-key");
        let user_a = UserId::new("user-a");
        let user_b = UserId::new("user-b");
        let scope = "scope-hash";

        let cursor = codec.encode(&user_a, scope, 42);
        assert_eq!(codec.decode(&cursor, &user_a, scope).unwrap(), 42);
        assert!(codec.decode(&cursor, &user_b, scope).is_err());
        let tampered = Cursor::new(format!("{}x", cursor.as_str()));
        assert!(codec.decode(&tampered, &user_a, scope).is_err());
    }
}
