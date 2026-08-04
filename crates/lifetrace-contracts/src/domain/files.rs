//! File metadata DTO (binary content is out of scope for EPIC-02/12).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::domain::enums::FileStorageState;
use crate::ids::DeviceId;

/// `file.metadata`
///
/// Never contains object storage keys, presigned URLs or local absolute
/// paths; only stable metadata and a content hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FileMetadata {
    pub meta: EntityMeta,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub storage_state: FileStorageState,
    pub created_by_device: Option<DeviceId>,
}
