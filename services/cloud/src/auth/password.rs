use std::collections::HashSet;
use std::fs;

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::{Algorithm, Argon2, Params, Version};
use axum::http::StatusCode;
use lifetrace_contracts::ErrorCode;

use crate::config::Config;
use crate::error::ApiError;

#[derive(Clone)]
pub struct PasswordManager {
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    minimum_chars: usize,
    maximum_bytes: usize,
    pepper: Vec<u8>,
    blocked: HashSet<String>,
}

impl PasswordManager {
    pub fn new(config: &Config) -> Self {
        let mut blocked: HashSet<String> = [
            "password",
            "password123",
            "123456789012345",
            "qwertyuiopasdfg",
            "lifetracelifetrace",
            "letmeinletmeinletmein",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        if let Some(path) = &config.auth_password_blocklist_path {
            if let Ok(text) = fs::read_to_string(path) {
                blocked.extend(
                    text.lines()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| value.to_lowercase()),
                );
            }
        }
        Self {
            memory_kib: config.auth_argon2_memory_kib,
            iterations: config.auth_argon2_iterations,
            parallelism: config.auth_argon2_parallelism,
            minimum_chars: config.auth_password_min_length,
            maximum_bytes: config.auth_password_max_bytes,
            pepper: config
                .auth_password_pepper
                .clone()
                .unwrap_or_default()
                .into_bytes(),
            blocked,
        }
    }

    fn argon2(&self) -> Result<Argon2<'_>, ApiError> {
        let params = Params::new(self.memory_kib, self.iterations, self.parallelism, None)
            .map_err(|error| {
                ApiError::new(
                    ErrorCode::InternalError,
                    error.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        Argon2::new_with_secret(&self.pepper, Algorithm::Argon2id, Version::V0x13, params).map_err(
            |error| {
                ApiError::new(
                    ErrorCode::InternalError,
                    error.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            },
        )
    }

    pub fn validate(&self, password: &str) -> Result<(), ApiError> {
        if password.chars().count() < self.minimum_chars {
            return Err(ApiError::new(
                ErrorCode::AuthPasswordPolicyFailed,
                format!(
                    "password must contain at least {} Unicode characters",
                    self.minimum_chars
                ),
                StatusCode::BAD_REQUEST,
            ));
        }
        if password.len() > self.maximum_bytes {
            return Err(ApiError::new(
                ErrorCode::AuthPasswordPolicyFailed,
                format!(
                    "password exceeds maximum byte length {}",
                    self.maximum_bytes
                ),
                StatusCode::BAD_REQUEST,
            ));
        }
        if self.blocked.contains(&password.to_lowercase()) {
            return Err(ApiError::new(
                ErrorCode::AuthPasswordPolicyFailed,
                "password is too common",
                StatusCode::BAD_REQUEST,
            ));
        }
        Ok(())
    }

    pub fn hash(&self, password: &str) -> Result<String, ApiError> {
        self.validate(password)?;
        let salt = SaltString::generate(&mut OsRng);
        self.argon2()?
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| {
                ApiError::new(
                    ErrorCode::InternalError,
                    error.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })
    }

    pub fn verify(&self, password: &str, encoded: &str) -> bool {
        let Ok(hash) = PasswordHash::new(encoded) else {
            return false;
        };
        self.argon2()
            .is_ok_and(|argon2| argon2.verify_password(password.as_bytes(), &hash).is_ok())
    }

    pub fn needs_rehash(&self, encoded: &str) -> bool {
        let Ok(hash) = PasswordHash::new(encoded) else {
            return true;
        };
        let decimal = |name| hash.params.get_decimal(name);
        decimal("m") != Some(self.memory_kib)
            || decimal("t") != Some(self.iterations)
            || decimal("p") != Some(self.parallelism)
            || hash.algorithm.as_str() != "argon2id"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_spaces_and_unicode() {
        let manager = PasswordManager::new(&Config::default());
        let password = " 这是一个足够长的 密码 phrase ";
        let hash = manager.hash(password).unwrap();
        assert!(manager.verify(password, &hash));
        assert!(!manager.verify(password.trim(), &hash));
    }

    #[test]
    fn rejects_short_and_common_passwords() {
        let manager = PasswordManager::new(&Config::default());
        assert!(manager.hash("Abc12!xy").is_err());
        assert!(manager.hash("Abc123!xy").is_ok());
        assert!(manager.hash("123456789012345").is_err());
    }
}
