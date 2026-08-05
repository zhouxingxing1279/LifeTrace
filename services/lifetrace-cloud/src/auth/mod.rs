//! Authentication boundary (EPIC-03 scope).
//!
//! EPIC-03 only defines the boundary: an `AuthProvider` trait, an
//! `AuthenticatedPrincipal` and development/test implementations. Real
//! registration, password hashing and tokens belong to EPIC-04.

pub mod development;
pub mod extract;
pub mod principal;
pub mod provider;
pub mod testing;

pub use development::DevelopmentAuthProvider;
pub use principal::AuthenticatedPrincipal;
pub use provider::AuthProvider;
pub use testing::TestAuthProvider;
