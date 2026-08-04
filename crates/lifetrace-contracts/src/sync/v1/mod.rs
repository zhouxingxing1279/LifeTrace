//! Sync protocol v1 DTOs.
//!
//! Endpoints (defined in OpenAPI; not implemented by this crate):
//! - `GET  /api/v1/sync/capabilities`
//! - `POST /api/v1/sync/push`
//! - `POST /api/v1/sync/pull`
//! - `POST /api/v1/sync/snapshot`

pub mod capability;
pub mod change;
pub mod client;
pub mod conflict;
pub mod pull;
pub mod push;
pub mod snapshot;
pub mod tombstone;

pub use capability::*;
pub use change::*;
pub use client::*;
pub use conflict::*;
pub use pull::*;
pub use push::*;
pub use snapshot::*;
pub use tombstone::*;
