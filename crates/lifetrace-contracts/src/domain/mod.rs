//! Public domain DTOs (wire format).
//!
//! Each DTO is the full authoritative snapshot of one entity type. The
//! database rows and the UI view models are separate layers; only these
//! structs are wire contracts.

pub mod english;
pub mod enums;
pub mod files;
pub mod finance;
pub mod habits;
pub mod links;
pub mod notes;
pub mod preferences;
pub mod reviews;
pub mod user;
pub mod workouts;

pub use english::*;
pub use enums::*;
pub use files::*;
pub use finance::*;
pub use habits::*;
pub use links::*;
pub use notes::*;
pub use preferences::*;
pub use reviews::*;
pub use user::*;
pub use workouts::*;
