use async_trait::async_trait;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;

#[derive(Debug, Clone, Copy)]
pub enum AuthCredential<'a> {
    Bearer(Option<&'a str>),
    WebSession(Option<&'a str>),
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(
        &self,
        credential: AuthCredential<'_>,
    ) -> Result<AuthenticatedPrincipal, ApiError>;
}
