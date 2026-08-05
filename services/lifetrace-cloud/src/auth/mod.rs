//! EPIC-04 authentication, authorization and security boundary.

pub mod database;
pub mod development;
pub mod extract;
pub mod password;
pub mod principal;
pub mod provider;
pub mod scope;
pub mod security;
pub mod service;
pub mod testing;
pub mod token;

pub use database::DatabaseAuthProvider;
pub use development::DevelopmentAuthProvider;
pub use principal::{AuthMethod, AuthenticatedPrincipal};
pub use provider::{AuthCredential, AuthProvider};
pub use service::AuthService;
pub use testing::TestAuthProvider;
