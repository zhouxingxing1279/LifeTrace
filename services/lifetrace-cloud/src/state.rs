//! Shared application state.

use std::sync::{Arc, RwLock};

use crate::auth::{AuthProvider, DevelopmentAuthProvider};
use crate::config::Config;
use crate::store::Store;
use crate::sync::cursor_codec::CursorCodec;
use crate::sync::page_token::PageTokenCodec;

/// Cloneable shared state for handlers.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<Store>>,
    pub config: Arc<Config>,
    pub auth: Arc<dyn AuthProvider>,
    pub cursor_codec: Arc<CursorCodec>,
    pub page_token_codec: Arc<PageTokenCodec>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let cursor_codec = CursorCodec::new(
            config
                .cursor_signing_key
                .clone()
                .unwrap_or_else(|| "dev-cursor-key".to_owned()),
        );
        let page_token_codec = PageTokenCodec::new(
            config
                .page_token_signing_key
                .clone()
                .unwrap_or_else(|| "dev-page-token-key".to_owned()),
        );
        let store = Store::new(
            config.clone(),
            cursor_codec.clone(),
            page_token_codec.clone(),
        );
        let auth = DevelopmentAuthProvider::new(
            config.dev_auth_enabled,
            config.dev_auth_token.clone(),
            lifetrace_contracts::UserId::new(config.dev_auth_user_id.clone()),
            lifetrace_contracts::DeviceId::new(config.dev_auth_device_id.clone()),
        );
        Self {
            store: Arc::new(RwLock::new(store)),
            config: Arc::new(config),
            auth: Arc::new(auth),
            cursor_codec: Arc::new(cursor_codec),
            page_token_codec: Arc::new(page_token_codec),
        }
    }
}
