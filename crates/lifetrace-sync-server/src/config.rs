//! Server configuration from environment variables.

use std::net::SocketAddr;

/// Runtime configuration for the sync server.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind, for example `127.0.0.1:8787`.
    pub bind_addr: SocketAddr,
    /// Maximum push batch size (capabilities are derived from this).
    pub max_push_batch_size: usize,
    /// Maximum pull batch size.
    pub max_pull_batch_size: usize,
    /// Maximum request body bytes.
    pub max_request_bytes: usize,
    /// Maximum snapshot page size.
    pub max_snapshot_page_size: usize,
    /// Maximum atomic group size.
    pub max_atomic_group_size: usize,
    /// Change-log retention in entries (older cursors expire -> snapshot required).
    pub retention_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8787".parse().expect("static addr"),
            max_push_batch_size: 500,
            max_pull_batch_size: 200,
            max_request_bytes: 4 * 1024 * 1024,
            max_snapshot_page_size: 200,
            max_atomic_group_size: 50,
            retention_entries: 1000,
        }
    }
}

impl Config {
    /// Load configuration from the environment, falling back to defaults.
    ///
    /// Supported variables:
    /// - `LIFETRACE_SERVER_BIND` (default `127.0.0.1:8787`)
    /// - `LIFETRACE_MAX_PUSH_BATCH_SIZE`
    /// - `LIFETRACE_MAX_PULL_BATCH_SIZE`
    /// - `LIFETRACE_MAX_REQUEST_BYTES`
    /// - `LIFETRACE_MAX_SNAPSHOT_PAGE_SIZE`
    /// - `LIFETRACE_MAX_ATOMIC_GROUP_SIZE`
    /// - `LIFETRACE_RETENTION_ENTRIES`
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Some(value) = std::env::var("LIFETRACE_SERVER_BIND").ok().filter(|v| !v.is_empty()) {
            if let Ok(addr) = value.parse() {
                config.bind_addr = addr;
            }
        }
        config.max_push_batch_size = env_usize("LIFETRACE_MAX_PUSH_BATCH_SIZE", config.max_push_batch_size);
        config.max_pull_batch_size = env_usize("LIFETRACE_MAX_PULL_BATCH_SIZE", config.max_pull_batch_size);
        config.max_request_bytes = env_usize("LIFETRACE_MAX_REQUEST_BYTES", config.max_request_bytes);
        config.max_snapshot_page_size =
            env_usize("LIFETRACE_MAX_SNAPSHOT_PAGE_SIZE", config.max_snapshot_page_size);
        config.max_atomic_group_size =
            env_usize("LIFETRACE_MAX_ATOMIC_GROUP_SIZE", config.max_atomic_group_size);
        config.retention_entries = env_usize("LIFETRACE_RETENTION_ENTRIES", config.retention_entries);
        config
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
