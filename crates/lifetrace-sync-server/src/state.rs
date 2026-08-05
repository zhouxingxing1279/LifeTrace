//! Shared application state.

use std::sync::{Arc, RwLock};

use crate::config::Config;
use crate::routes::devices::DeviceRegistry;
use crate::store::Store;

/// Cloneable shared state for handlers.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<Store>>,
    pub config: Arc<Config>,
    pub devices: Arc<RwLock<DeviceRegistry>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            store: Arc::new(RwLock::new(Store::new(config.clone()))),
            config: Arc::new(config),
            devices: Arc::new(RwLock::new(DeviceRegistry::default())),
        }
    }
}
