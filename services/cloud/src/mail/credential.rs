use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("mail credential key is not configured")]
    MissingKey,
    #[error("mail credential key must be base64-encoded 32 bytes")]
    InvalidKey,
    #[error("mail credential encryption failed")]
    Encrypt,
    #[error("mail credential decryption failed")]
    Decrypt,
}

#[derive(Clone)]
pub struct CredentialCipher {
    key: [u8; 32],
}

impl CredentialCipher {
    pub fn from_env() -> Result<Self, CredentialError> {
        let encoded = std::env::var("MAIL_CREDENTIAL_KEY").map_err(|_| CredentialError::MissingKey)?;
        Self::from_base64(&encoded)
    }

    pub fn from_base64(encoded: &str) -> Result<Self, CredentialError> {
        let decoded = STANDARD.decode(encoded.trim()).map_err(|_| CredentialError::InvalidKey)?;
        let key: [u8; 32] = decoded.try_into().map_err(|_| CredentialError::InvalidKey)?;
        Ok(Self { key })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<(Vec<u8>, Vec<u8>), CredentialError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| CredentialError::InvalidKey)?;
        let mut nonce_bytes = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
            .map_err(|_| CredentialError::Encrypt)?;
        Ok((ciphertext, nonce_bytes.to_vec()))
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> Result<String, CredentialError> {
        if nonce.len() != 12 {
            return Err(CredentialError::Decrypt);
        }
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| CredentialError::InvalidKey)?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| CredentialError::Decrypt)?;
        String::from_utf8(plaintext).map_err(|_| CredentialError::Decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> String {
        STANDARD.encode([7_u8; 32])
    }

    #[test]
    fn encrypt_round_trip_does_not_store_plaintext() {
        let cipher = CredentialCipher::from_base64(&key()).expect("cipher");
        let secret = "authorization-code-not-for-logs";
        let (encrypted, nonce) = cipher.encrypt(secret).expect("encrypt");
        assert_ne!(encrypted, secret.as_bytes());
        assert_eq!(cipher.decrypt(&encrypted, &nonce).expect("decrypt"), secret);
    }

    #[test]
    fn rejects_short_key() {
        assert!(matches!(
            CredentialCipher::from_base64(&STANDARD.encode([1_u8; 8])),
            Err(CredentialError::InvalidKey)
        ));
    }
}
