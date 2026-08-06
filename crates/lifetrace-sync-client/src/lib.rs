//! Platform-independent synchronization behavior for LifeTrace clients.
//!
//! This crate deliberately has no dependency on Tauri, SQLite, reqwest,
//! Windows APIs, Axum or SQLx. Platform adapters provide storage, transport,
//! clock and credential implementations through the traits in [`traits`].

pub mod conflict;
pub mod engine;
pub mod error;
pub mod pull;
pub mod push;
pub mod retry;
pub mod snapshot;
pub mod state;
pub mod traits;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;

pub use conflict::*;
pub use engine::*;
pub use error::*;
pub use retry::*;
pub use state::*;
pub use traits::*;
