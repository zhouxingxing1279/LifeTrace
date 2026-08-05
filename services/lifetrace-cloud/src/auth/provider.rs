//! Auth provider trait.

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;

/// Extracts an authenticated principal from an `Authorization` header value.
///
/// Implementations are synchronous for the development/test scope of
/// EPIC-03; EPIC-04's token provider can switch this to async without
/// touching the sync business layer.
pub trait AuthProvider: Send + Sync {
    /// Returns the principal or a `LIFETRACE_AUTH_REQUIRED` /
    /// `LIFETRACE_AUTH_INVALID` error.
    fn authenticate(
        &self,
        authorization: Option<&str>,
    ) -> Result<AuthenticatedPrincipal, ApiError>;
}
