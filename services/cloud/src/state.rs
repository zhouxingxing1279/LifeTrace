//! Shared application state.

use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::auth::{AuthProvider, AuthService, DatabaseAuthProvider, DevelopmentAuthProvider};
use crate::beecount_adapter::BeeCountAdapter;
use crate::beecount_realtime::BeeCountRealtimeHub;
use crate::config::Config;
use crate::postgres_repository::PostgresRepository;
use crate::repository::{MemoryRepository, SyncRepository};
use crate::sync::cursor_codec::CursorCodec;
use crate::sync::page_token::PageTokenCodec;

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("PostgreSQL connection failed: {0}")]
    Pool(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

/// Cloneable shared state for handlers.
#[derive(Clone)]
pub struct AppState {
    /// A real SQLx pool is always present. Production and configured cloud
    /// environments use it for every sync and authentication operation; the
    /// memory repository is retained only for in-process protocol tests.
    pub pool: PgPool,
    pub database_enabled: bool,
    pub store: Arc<dyn SyncRepository>,
    pub config: Arc<Config>,
    pub auth: Arc<dyn AuthProvider>,
    pub auth_service: Arc<AuthService>,
    pub cursor_codec: Arc<CursorCodec>,
    pub page_token_codec: Arc<PageTokenCodec>,
    pub beecount_adapter: Option<Arc<BeeCountAdapter>>,
    pub beecount_realtime: Arc<BeeCountRealtimeHub>,
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

        let database_enabled = config.database_url.is_some();
        let database_url = config.database_url.clone().unwrap_or_else(|| {
            "postgres://lifetrace:lifetrace_test_password@127.0.0.1:5433/lifetrace_test".to_owned()
        });
        let pool = PgPoolOptions::new()
            .min_connections(if database_enabled {
                config.database_min_connections
            } else {
                0
            })
            .max_connections(config.database_max_connections.max(1))
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy(&database_url)
            .expect("DATABASE_URL must be a valid PostgreSQL URL");

        let store: Arc<dyn SyncRepository> = if database_enabled {
            Arc::new(PostgresRepository::new(
                pool.clone(),
                config.clone(),
                cursor_codec.clone(),
                page_token_codec.clone(),
            ))
        } else {
            Arc::new(MemoryRepository::new(
                config.clone(),
                cursor_codec.clone(),
                page_token_codec.clone(),
            ))
        };

        let auth_service = Arc::new(AuthService::new(pool.clone(), config.clone()));
        let auth: Arc<dyn AuthProvider> = if database_enabled {
            Arc::new(DatabaseAuthProvider::new(
                pool.clone(),
                auth_service.token_manager(),
            ))
        } else {
            Arc::new(DevelopmentAuthProvider::new(
                config.dev_auth_enabled,
                config.dev_auth_token.clone(),
                lifetrace_contracts::UserId::new(config.dev_auth_user_id.clone()),
                config.dev_auth_device_id.clone(),
            ))
        };
        let beecount_adapter = if config.beecount_adapter_enabled {
            Some(Arc::new(
                BeeCountAdapter::from_config(&config)
                    .expect("BeeCount adapter configuration must be validated"),
            ))
        } else {
            None
        };

        Self {
            pool,
            database_enabled,
            store,
            config: Arc::new(config),
            auth,
            auth_service,
            cursor_codec: Arc::new(cursor_codec),
            page_token_codec: Arc::new(page_token_codec),
            beecount_adapter,
            beecount_realtime: Arc::new(BeeCountRealtimeHub::default()),
        }
    }

    /// Connect to PostgreSQL and execute embedded SQLx migrations before the
    /// server starts accepting traffic.
    pub async fn initialize(&self) -> Result<(), StartupError> {
        if !self.database_enabled {
            return Ok(());
        }
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        if self.config.migration_on_startup {
            sqlx::migrate!().run(&self.pool).await?;
        }
        Ok(())
    }
}
