//! Strongly typed cloud and authentication configuration.

use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub environment: String,
    pub bind_addr: SocketAddr,
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

    /// Development-only fixed Bearer credential retained for in-process tests.
    pub dev_auth_enabled: bool,
    pub dev_auth_user_id: String,
    pub dev_auth_device_id: String,
    pub dev_auth_token: String,

    pub auth_registration_mode: String,
    pub auth_access_token_ttl_seconds: u64,
    pub auth_refresh_idle_ttl_seconds: u64,
    pub auth_refresh_absolute_ttl_seconds: u64,
    pub auth_web_idle_ttl_seconds: u64,
    pub auth_web_absolute_ttl_seconds: u64,
    pub auth_public_device_ttl_seconds: u64,
    pub auth_argon2_memory_kib: u32,
    pub auth_argon2_iterations: u32,
    pub auth_argon2_parallelism: u32,
    pub auth_password_min_length: usize,
    pub auth_password_max_bytes: usize,
    pub auth_password_blocklist_path: Option<String>,
    pub auth_password_pepper: Option<String>,
    pub auth_token_hash_pepper: Option<String>,
    pub auth_reset_token_ttl_seconds: u64,
    pub auth_login_account_limit: usize,
    pub auth_login_ip_limit: usize,
    pub auth_login_window_seconds: u64,
    pub auth_lockout_seconds: u64,
    pub auth_cookie_name: String,
    pub auth_cookie_same_site: String,
    pub auth_cookie_secure: bool,
    pub auth_trusted_proxy_cidrs: Vec<String>,
    pub auth_reset_notifier: String,
    pub public_web_base_url: Option<String>,

    pub snapshot_ttl_seconds: u64,
    pub maintenance_interval_seconds: u64,
    pub graceful_shutdown_seconds: u64,
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
            auth_registration_mode: "disabled".to_owned(),
            auth_access_token_ttl_seconds: 15 * 60,
            auth_refresh_idle_ttl_seconds: 30 * 24 * 60 * 60,
            auth_refresh_absolute_ttl_seconds: 90 * 24 * 60 * 60,
            auth_web_idle_ttl_seconds: 12 * 60 * 60,
            auth_web_absolute_ttl_seconds: 7 * 24 * 60 * 60,
            auth_public_device_ttl_seconds: 8 * 60 * 60,
            auth_argon2_memory_kib: 19_456,
            auth_argon2_iterations: 2,
            auth_argon2_parallelism: 1,
            auth_password_min_length: 15,
            auth_password_max_bytes: 512,
            auth_password_blocklist_path: None,
            auth_password_pepper: Some("development-password-pepper".to_owned()),
            auth_token_hash_pepper: Some("development-token-pepper".to_owned()),
            auth_reset_token_ttl_seconds: 30 * 60,
            auth_login_account_limit: 5,
            auth_login_ip_limit: 30,
            auth_login_window_seconds: 15 * 60,
            auth_lockout_seconds: 15 * 60,
            auth_cookie_name: "__Host-lifetrace_session".to_owned(),
            auth_cookie_same_site: "Lax".to_owned(),
            auth_cookie_secure: false,
            auth_trusted_proxy_cidrs: Vec::new(),
            auth_reset_notifier: "console".to_owned(),
            public_web_base_url: Some("http://127.0.0.1:8787".to_owned()),
            snapshot_ttl_seconds: 3600,
            maintenance_interval_seconds: 300,
            graceful_shutdown_seconds: 10,
            retention_entries: 1000,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut c = Self::default();
        c.environment = env_string("LIFETRACE_ENV", &c.environment).to_ascii_lowercase();
        if let Some(value) = env_var("LIFETRACE_BIND_ADDRESS") {
            if let Ok(addr) = value.parse() {
                c.bind_addr = addr;
            }
        }
        c.database_url = env_var("DATABASE_URL");
        c.database_min_connections = env_usize(
            "DATABASE_MIN_CONNECTIONS",
            c.database_min_connections as usize,
        ) as u32;
        c.database_max_connections = env_usize(
            "DATABASE_MAX_CONNECTIONS",
            c.database_max_connections as usize,
        ) as u32;
        c.migration_on_startup = env_bool("MIGRATION_ON_STARTUP", c.migration_on_startup);
        c.request_body_limit_bytes =
            env_usize("REQUEST_BODY_LIMIT_BYTES", c.request_body_limit_bytes);
        c.push_max_changes = env_usize("PUSH_MAX_CHANGES", c.push_max_changes);
        c.pull_max_changes = env_usize("PULL_MAX_CHANGES", c.pull_max_changes);
        c.snapshot_max_page_size = env_usize("SNAPSHOT_MAX_PAGE_SIZE", c.snapshot_max_page_size);
        c.maximum_atomic_group_size =
            env_usize("MAXIMUM_ATOMIC_GROUP_SIZE", c.maximum_atomic_group_size);
        c.cursor_signing_key = env_var("CURSOR_SIGNING_KEY").or(c.cursor_signing_key);
        c.page_token_signing_key = env_var("PAGE_TOKEN_SIGNING_KEY").or(c.page_token_signing_key);
        c.cors_allowed_origins = env_csv("CORS_ALLOWED_ORIGINS", c.cors_allowed_origins);
        c.dev_auth_enabled = env_bool("DEV_AUTH_ENABLED", c.dev_auth_enabled);
        c.dev_auth_user_id = env_string("DEV_AUTH_USER_ID", &c.dev_auth_user_id);
        c.dev_auth_device_id = env_string("DEV_AUTH_DEVICE_ID", &c.dev_auth_device_id);
        c.dev_auth_token = env_string("DEV_AUTH_TOKEN", &c.dev_auth_token);

        c.auth_registration_mode =
            env_string("AUTH_REGISTRATION_MODE", &c.auth_registration_mode).to_ascii_lowercase();
        c.auth_access_token_ttl_seconds = env_u64(
            "AUTH_ACCESS_TOKEN_TTL_SECONDS",
            c.auth_access_token_ttl_seconds,
        );
        c.auth_refresh_idle_ttl_seconds = env_u64(
            "AUTH_REFRESH_IDLE_TTL_SECONDS",
            c.auth_refresh_idle_ttl_seconds,
        );
        c.auth_refresh_absolute_ttl_seconds = env_u64(
            "AUTH_REFRESH_ABSOLUTE_TTL_SECONDS",
            c.auth_refresh_absolute_ttl_seconds,
        );
        c.auth_web_idle_ttl_seconds =
            env_u64("AUTH_WEB_IDLE_TTL_SECONDS", c.auth_web_idle_ttl_seconds);
        c.auth_web_absolute_ttl_seconds = env_u64(
            "AUTH_WEB_ABSOLUTE_TTL_SECONDS",
            c.auth_web_absolute_ttl_seconds,
        );
        c.auth_public_device_ttl_seconds = env_u64(
            "AUTH_PUBLIC_DEVICE_TTL_SECONDS",
            c.auth_public_device_ttl_seconds,
        );
        c.auth_argon2_memory_kib =
            env_usize("AUTH_ARGON2_MEMORY_KIB", c.auth_argon2_memory_kib as usize) as u32;
        c.auth_argon2_iterations =
            env_usize("AUTH_ARGON2_ITERATIONS", c.auth_argon2_iterations as usize) as u32;
        c.auth_argon2_parallelism = env_usize(
            "AUTH_ARGON2_PARALLELISM",
            c.auth_argon2_parallelism as usize,
        ) as u32;
        c.auth_password_min_length =
            env_usize("AUTH_PASSWORD_MIN_LENGTH", c.auth_password_min_length);
        c.auth_password_max_bytes = env_usize("AUTH_PASSWORD_MAX_BYTES", c.auth_password_max_bytes);
        c.auth_password_blocklist_path = env_var("AUTH_PASSWORD_BLOCKLIST_PATH");
        c.auth_password_pepper = env_var("AUTH_PASSWORD_PEPPER").or(c.auth_password_pepper);
        c.auth_token_hash_pepper = env_var("AUTH_TOKEN_HASH_PEPPER").or(c.auth_token_hash_pepper);
        c.auth_reset_token_ttl_seconds = env_u64(
            "AUTH_RESET_TOKEN_TTL_SECONDS",
            c.auth_reset_token_ttl_seconds,
        );
        c.auth_login_account_limit =
            env_usize("AUTH_LOGIN_ACCOUNT_LIMIT", c.auth_login_account_limit);
        c.auth_login_ip_limit = env_usize("AUTH_LOGIN_IP_LIMIT", c.auth_login_ip_limit);
        c.auth_login_window_seconds =
            env_u64("AUTH_LOGIN_WINDOW_SECONDS", c.auth_login_window_seconds);
        c.auth_lockout_seconds = env_u64("AUTH_LOCKOUT_SECONDS", c.auth_lockout_seconds);
        c.auth_cookie_name = env_string("AUTH_COOKIE_NAME", &c.auth_cookie_name);
        c.auth_cookie_same_site = env_string("AUTH_COOKIE_SAME_SITE", &c.auth_cookie_same_site);
        c.auth_cookie_secure = env_bool("AUTH_COOKIE_SECURE", c.auth_cookie_secure);
        c.auth_trusted_proxy_cidrs =
            env_csv("AUTH_TRUSTED_PROXY_CIDRS", c.auth_trusted_proxy_cidrs);
        c.auth_reset_notifier =
            env_string("AUTH_RESET_NOTIFIER", &c.auth_reset_notifier).to_ascii_lowercase();
        c.public_web_base_url = env_var("PUBLIC_WEB_BASE_URL").or(c.public_web_base_url);

        c.snapshot_ttl_seconds = env_u64("SNAPSHOT_TTL_SECONDS", c.snapshot_ttl_seconds);
        c.maintenance_interval_seconds = env_u64(
            "MAINTENANCE_INTERVAL_SECONDS",
            c.maintenance_interval_seconds,
        );
        c.graceful_shutdown_seconds =
            env_u64("GRACEFUL_SHUTDOWN_SECONDS", c.graceful_shutdown_seconds);
        c.retention_entries = env_usize("LIFETRACE_RETENTION_ENTRIES", c.retention_entries);
        c
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

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
        if !matches!(
            self.auth_registration_mode.as_str(),
            "disabled" | "invite" | "open"
        ) {
            return Err("AUTH_REGISTRATION_MODE must be disabled, invite or open".to_owned());
        }
        if self.auth_password_min_length < 15 || self.auth_password_max_bytes < 64 {
            return Err("password policy is weaker than the EPIC-04 minimum".to_owned());
        }
        if self.auth_refresh_idle_ttl_seconds > self.auth_refresh_absolute_ttl_seconds {
            return Err("refresh idle TTL must not exceed absolute TTL".to_owned());
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
            let password_pepper = self.auth_password_pepper.as_deref().unwrap_or_default();
            let token_pepper = self.auth_token_hash_pepper.as_deref().unwrap_or_default();
            if password_pepper.len() < 32
                || token_pepper.len() < 32
                || password_pepper.starts_with("development-")
                || token_pepper.starts_with("development-")
            {
                return Err("production requires non-default AUTH_PASSWORD_PEPPER and AUTH_TOKEN_HASH_PEPPER of at least 32 characters".to_owned());
            }
            if !self.auth_cookie_secure {
                return Err("production requires AUTH_COOKIE_SECURE=true".to_owned());
            }
            if !self
                .public_web_base_url
                .as_deref()
                .is_some_and(|value| value.starts_with("https://"))
            {
                return Err("production requires HTTPS PUBLIC_WEB_BASE_URL".to_owned());
            }
            if self.auth_reset_notifier == "console" {
                return Err(
                    "production must not use the console password reset notifier".to_owned(),
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
fn env_u64(name: &str, default: u64) -> u64 {
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
fn env_csv(name: &str, default: Vec<String>) -> Vec<String> {
    env_var(name)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_runtime_requires_postgres() {
        assert!(Config::default()
            .validate()
            .unwrap_err()
            .contains("DATABASE_URL"));
    }

    #[test]
    fn development_config_accepts_postgres() {
        let config = Config {
            database_url: Some("postgres://user:password@localhost/lifetrace".to_owned()),
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn production_fails_closed_without_auth_secrets() {
        let config = Config {
            environment: "production".to_owned(),
            database_url: Some("postgres://user:password@localhost/lifetrace".to_owned()),
            dev_auth_enabled: false,
            auth_cookie_secure: true,
            public_web_base_url: Some("https://lifetrace.example".to_owned()),
            auth_reset_notifier: "smtp".to_owned(),
            ..Config::default()
        };
        assert!(config.validate().unwrap_err().contains("PEPPER"));
    }
}
