use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::config::Config;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Access,
    Refresh,
    WebSession,
    PasswordReset,
    Csrf,
    Invite,
}

impl TokenKind {
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Access => "lt_at_",
            Self::Refresh => "lt_rt_",
            Self::WebSession => "lt_ws_",
            Self::PasswordReset => "lt_pr_",
            Self::Csrf => "lt_cs_",
            Self::Invite => "lt_iv_",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedToken {
    pub id: Uuid,
    pub raw: String,
    pub hash: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ParsedToken<'a> {
    pub id: Uuid,
    pub secret: &'a str,
}

#[derive(Clone)]
pub struct TokenManager {
    pepper: Vec<u8>,
}

impl TokenManager {
    pub fn new(config: &Config) -> Self {
        Self {
            pepper: config
                .auth_token_hash_pepper
                .clone()
                .unwrap_or_default()
                .into_bytes(),
        }
    }

    pub fn generate(&self, kind: TokenKind) -> GeneratedToken {
        let id = Uuid::new_v4();
        let mut bytes = [0_u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let secret = URL_SAFE_NO_PAD.encode(bytes);
        let raw = format!("{}{}.{}", kind.prefix(), id, secret);
        let hash = self.hash(kind, id, &secret);
        GeneratedToken { id, raw, hash }
    }

    pub fn parse<'a>(&self, kind: TokenKind, raw: &'a str) -> Option<ParsedToken<'a>> {
        if raw.len() > 160 || !raw.starts_with(kind.prefix()) {
            return None;
        }
        let body = &raw[kind.prefix().len()..];
        let (id, secret) = body.split_once('.')?;
        if secret.len() < 40
            || !secret
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return None;
        }
        Some(ParsedToken {
            id: Uuid::parse_str(id).ok()?,
            secret,
        })
    }

    pub fn hash(&self, kind: TokenKind, id: Uuid, secret: &str) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(&self.pepper).expect("HMAC accepts arbitrary key length");
        mac.update(kind.prefix().as_bytes());
        mac.update(id.as_bytes());
        mac.update(secret.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    pub fn verify(&self, kind: TokenKind, parsed: &ParsedToken<'_>, expected: &[u8]) -> bool {
        let actual = self.hash(kind, parsed.id, parsed.secret);
        actual.len() == expected.len() && bool::from(actual.ct_eq(expected))
    }

    pub fn redacted(raw: &str) -> String {
        let prefix = raw.get(..5).unwrap_or("token");
        format!("{prefix}<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trip_and_hash_validation() {
        let manager = TokenManager::new(&Config::default());
        let token = manager.generate(TokenKind::Refresh);
        let parsed = manager.parse(TokenKind::Refresh, &token.raw).unwrap();
        assert_eq!(parsed.id, token.id);
        assert!(manager.verify(TokenKind::Refresh, &parsed, &token.hash));
        assert!(manager.parse(TokenKind::Access, &token.raw).is_none());
        assert!(!TokenManager::redacted(&token.raw).contains(parsed.secret));
    }
}
