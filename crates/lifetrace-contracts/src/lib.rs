//! LifeTrace public domain contract and sync protocol v1.
//!
//! This crate is the authoritative source for:
//! - shared value objects (IDs, money, timestamps, cursors, versions)
//! - public domain DTOs (wire format)
//! - the entity type registry
//! - the sync protocol v1 wire DTOs
//! - stable error codes
//!
//! Constraints:
//! - No dependency on Tauri, Axum, rusqlite, React, or any database/HTTP
//!   implementation. This crate only defines contracts.
//! - Rust types are the source of truth; JSON Schema and TypeScript are
//!   generated from them (see `tools/contract-exporter`).
//! - Wire JSON uses camelCase. Timestamps are RFC3339 UTC. Natural days are
//!   `YYYY-MM-DD`. Money is integer cents (`amountCents`), never floats.
//! - `Cursor`, `serverVersion` and `baseServerVersion` are strings on the
//!   wire to avoid JavaScript safe-integer issues.

pub mod common;
pub mod domain;
pub mod error;
pub mod ids;
pub mod json_value;
pub mod money;
pub mod registry;
pub mod sync;
pub mod time;

pub use common::EntityMeta;
pub use error::{ApiErrorV1, ErrorCode, FieldError};
pub use ids::{
    AtomicGroupId, ChangeId, ConflictId, Cursor, DeviceId, EntityId, RequestId, ServerVersion,
    SnapshotId, UserId,
};
pub use json_value::JsonValue;
pub use money::{CurrencyCode, MoneyAmount};
pub use registry::{EntityOwnership, EntityRef, EntityType, SyncMode, ConflictMode};
pub use time::{LocalDate, UtcTimestamp};

/// Current protocol version implemented by this crate.
pub const PROTOCOL_VERSION: u32 = 1;

/// Current overall domain schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// Minimum domain schema version a v1 client may use.
pub const MINIMUM_SCHEMA_VERSION: u32 = 1;
