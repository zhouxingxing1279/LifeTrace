//! Storage abstraction for the sync protocol.
//!
//! Production uses [`PostgresRepository`]. The in-memory adapter remains
//! available for fast protocol unit tests only; production configuration
//! requires PostgreSQL.

use async_trait::async_trait;
use lifetrace_contracts::sync::v1::{
    CapabilitiesResponseV1, EntitySnapshotV1, PullRequestV1, PullResponseV1, PushRequestV1,
    PushResponseV1, SnapshotRequestV1, SnapshotResponseV1,
};
use lifetrace_contracts::{EntityId, EntityType, ServerVersion, UserId};
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::store::Store;
use crate::sync::cursor_codec::CursorCodec;
use crate::sync::page_token::PageTokenCodec;
use crate::Config;

#[derive(Debug, Clone)]
pub struct StoredEntityRecord {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub server_version: u64,
    pub payload: lifetrace_contracts::json_value::JsonValue,
    pub deleted: bool,
}

#[async_trait]
pub trait SyncRepository: Send + Sync {
    async fn capabilities(&self) -> Result<CapabilitiesResponseV1, ApiError>;

    async fn push(
        &self,
        user_id: &UserId,
        request: &PushRequestV1,
    ) -> Result<PushResponseV1, ApiError>;

    async fn pull(
        &self,
        user_id: &UserId,
        request: &PullRequestV1,
    ) -> Result<PullResponseV1, ApiError>;

    async fn snapshot(
        &self,
        user_id: &UserId,
        request: &SnapshotRequestV1,
    ) -> Result<SnapshotResponseV1, ApiError>;

    async fn list_entities(
        &self,
        user_id: &UserId,
        entity_type: &str,
    ) -> Result<Vec<EntitySnapshotV1>, ApiError>;

    async fn entity(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<StoredEntityRecord>, ApiError>;

    async fn current_version(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &EntityId,
    ) -> Result<Option<u64>, ApiError>;

    async fn change_count(&self, user_id: &UserId) -> Result<usize, ApiError>;
}

pub struct MemoryRepository {
    store: RwLock<Store>,
}

impl MemoryRepository {
    pub fn new(config: Config, cursor_codec: CursorCodec, page_token_codec: PageTokenCodec) -> Self {
        Self {
            store: RwLock::new(Store::new(config, cursor_codec, page_token_codec)),
        }
    }
}

#[async_trait]
impl SyncRepository for MemoryRepository {
    async fn capabilities(&self) -> Result<CapabilitiesResponseV1, ApiError> {
        Ok(self.store.read().await.capabilities())
    }

    async fn push(
        &self,
        user_id: &UserId,
        request: &PushRequestV1,
    ) -> Result<PushResponseV1, ApiError> {
        self.store.write().await.push(user_id, request)
    }

    async fn pull(
        &self,
        user_id: &UserId,
        request: &PullRequestV1,
    ) -> Result<PullResponseV1, ApiError> {
        self.store.read().await.pull(user_id, request)
    }

    async fn snapshot(
        &self,
        user_id: &UserId,
        request: &SnapshotRequestV1,
    ) -> Result<SnapshotResponseV1, ApiError> {
        self.store.write().await.snapshot(user_id, request)
    }

    async fn list_entities(
        &self,
        user_id: &UserId,
        entity_type: &str,
    ) -> Result<Vec<EntitySnapshotV1>, ApiError> {
        Ok(self.store.read().await.list_entities(user_id, entity_type))
    }

    async fn entity(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<StoredEntityRecord>, ApiError> {
        Ok(self
            .store
            .read()
            .await
            .entity(user_id, entity_type, entity_id)
            .map(|entity| StoredEntityRecord {
                entity_type: entity.entity_type.clone(),
                entity_id: entity.entity_id.clone(),
                server_version: entity.server_version,
                payload: entity.payload.clone(),
                deleted: entity.deleted,
            }))
    }

    async fn current_version(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &EntityId,
    ) -> Result<Option<u64>, ApiError> {
        Ok(self
            .store
            .read()
            .await
            .current_version(user_id, entity_type, entity_id))
    }

    async fn change_count(&self, user_id: &UserId) -> Result<usize, ApiError> {
        Ok(self.store.read().await.change_count(user_id))
    }
}

#[allow(dead_code)]
fn _server_version_type(_: &ServerVersion) {}
