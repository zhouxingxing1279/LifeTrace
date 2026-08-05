//! Strongly-typed cloud configuration (per EPIC-03 plan section 7).

use std::net::SocketAddr;

/// Runtime configuration for the cloud sync server.
#[derive(Debug, Clone)]
pub struct Config {
    /// `development` or `production`.
    pub environment: String,
    pub bind_addr: SocketAddr,
    /// PostgreSQL URL. The executable cloud runtime requires this value;
    /// `Config::default()` without a URL is reserved for in-process tests.
    pub database_url: Option<String>,
    pub database_min_connections: u32,
    pub database_max_connections: u32,
    pub migration_on_startup: bool,

    pub request_body_limit_bytes: usize,
    pub push_max_changes: usize,
    pub pull_max_changes: usize,
    pub snapshot_max_page_size: usize,
    pub maximum_atomic_group_size: usize,

    pub cursor_signing_key: Option<String>,
    pub page_token_signing_key: Option<String>,
    pub cors_allowed_origins: Vec<String>,

    pub dev_auth_enabled: bool,
    pub dev_auth_user_id: String,
    pub dev_auth_device_id: String,
    pub dev_auth_token: String,

    pub snapshot_ttl_seconds: u64,
    pub maintenance_interval_seconds: u64,
    pub graceful_shutdown_seconds: u64,

    /// Maximum retained change-log entries per user.
    pub retention_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            environment: "development".to_owned(),
            bind_addr: "127.0.0.1:8787".parse().expect("static addr"),
            database_url: None,
            database_min_connections: 2,
            database_max_connections: 10,
            migration_on_startup: true,
            request_body_limit_bytes: 4 * 1024 * 1024,
            push_max_changes: 500,
            pull_max_changes: 200,
            snapshot_max_page_size: 200,
            maximum_atomic_group_size: 50,
            cursor_signing_key: Some("dev-cursor-key".to_owned()),
            page_token_signing_key: Some("dev-page-token-key".to_owned()),
            cors_allowed_origins: Vec::new(),
            dev_auth_enabled: true,
            dev_auth_user_id: "dev-user".to_owned(),
            dev_auth_device_id: "dev-device".to_owned(),
            dev_auth_token: "dev-token".to_owned(),
            snapshot_ttl_seconds: 3600,
            maintenance_interval_seconds: 300,
            graceful_shutdown_seconds: 10,
            retention_entries: 1000,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        config.environment =
            env_string("LIFETRACE_ENV", &config.environment).to_ascii_lowercase();
        if let Some(value) = env_var("LIFETRACE_BIND_ADDRESS") {
            if let Ok(addr) = value.parse() {
                config.bind_addr = addr;
            }
        }
        config.database_url = env_var("DATABASE_URL");
        config.database_min_connections = env_usize(
            "DATABASE_MIN_CONNECTIONS",
            config.database_min_connections as usize,
        ) as u32;
        config.database_max_connections = env_usize(
            "DATABASE_MAX_CONNECTIONS",
            config.database_max_connections as usize,
        ) as u32;
        config.migration_on_startup =
            env_bool("MIGRATION_ON_STARTUP", config.migration_on_startup);
        config.request_body_limit_bytes =
            env_usize("REQUEST_BODY_LIMIT_BYTES", config.request_body_limit_bytes);
        config.push_max_changes = env_usize("PUSH_MAX_CHANGES", config.push_max_changes);
        config.pull_max_changes = env_usize("PULL_MAX_CHANGES", config.pull_max_changes);
        config.snapshot_max_page_size =
            env_usize("SNAPSHOT_MAX_PAGE_SIZE", config.snapshot_max_page_size);
        config.maximum_atomic_group_size =
            env_usize("MAXIMUM_ATOMIC_GROUP_SIZE", config.maximum_atomic_group_size);
        config.cursor_signing_key = env_var("CURSOR_SIGNING_KEY").or(config.cursor_signing_key);
        config.page_token_signing_key =
            env_var("PAGE_TOKEN_SIGNING_KEY").or(config.page_token_signing_key);
        if let Some(origins) = env_var("CORS_ALLOWED_ORIGINS") {
            config.cors_allowed_origins = origins
                .split(',')
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .collect();
        }
        config.dev_auth_enabled = env_bool("DEV_AUTH_ENABLED", config.dev_auth_enabled);
        config.dev_auth_user_id = env_string("DEV_AUTH_USER_ID", &config.dev_auth_user_id);
        config.dev_auth_device_id = env_string("DEV_AUTH_DEVICE_ID", &config.dev_auth_device_id);
        config.dev_auth_token = env_string("DEV_AUTH_TOKEN", &config.dev_auth_token);
        config.snapshot_ttl_seconds =
            env_usize("SNAPSHOT_TTL_SECONDS", config.snapshot_ttl_seconds as usize) as u64;
        config.maintenance_interval_seconds = env_usize(
            "MAINTENANCE_INTERVAL_SECONDS",
            config.maintenance_interval_seconds as usize,
        ) as u64;
        config.graceful_shutdown_seconds = env_usize(
            "GRACEFUL_SHUTDOWN_SECONDS",
            config.graceful_shutdown_seconds as usize,
        ) as u64;
        config.retention_entries =
            env_usize("LIFETRACE_RETENTION_ENTRIES", config.retention_entries);
        config
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// Startup validation. The standalone cloud runtime never falls back to
    /// in-memory persistence; the memory adapter is available only to tests
    /// that construct `AppState` directly without calling this method.
    pub fn validate(&self) -> Result<(), String> {
        if self.database_url.is_none() {
            return Err("cloud runtime requires DATABASE_URL".to_owned());
        }
        if self.database_min_connections > self.database_max_connections {
            return Err(
                "DATABASE_MIN_CONNECTIONS must not exceed DATABASE_MAX_CONNECTIONS".to_owned(),
            );
        }
        if self.database_max_connections == 0 {
            return Err("DATABASE_MAX_CONNECTIONS must be greater than zero".to_owned());
        }
        if self.is_production() {
            if self.dev_auth_enabled {
                return Err("production must not enable DEV_AUTH".to_owned());
            }
            if self.cursor_signing_key.is_none() || self.page_token_signing_key.is_none() {
                return Err(
                    "production requires CURSOR_SIGNING_KEY and PAGE_TOKEN_SIGNING_KEY".to_owned(),
                );
            }
        }
        Ok(())
    }
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn env_string(name: &str, default: &str) -> String {
    env_var(name).unwrap_or_else(|| default.to_owned())
}

fn env_usize(name: &str, default: usize) -> usize {
    env_var(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_bool(name: &str, default: bool) -> bool {
    env_var(name)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_runtime_requires_postgres() {
        let error = Config::default().validate().unwrap_err();
        assert!(error.contains("DATABASE_URL"));
    }

    #[test]
    fn development_config_accepts_postgres() {
        let mut config = Config::default();
        config.database_url = Some("postgres://user:password@localhost/lifetrace".to_owned());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn invalid_pool_bounds_are_rejected() {
        let mut config = Config::default();
        config.database_url = Some("postgres://user:password@localhost/lifetrace".to_owned());
        config.database_min_connections = 11;
        config.database_max_connections = 10;
        assert!(config.validate().is_err());
    }
}
