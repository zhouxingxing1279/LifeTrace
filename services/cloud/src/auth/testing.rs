use crate::auth::{AuthCredential, AuthProvider, AuthenticatedPrincipal};
use crate::error::ApiError;
use async_trait::async_trait;
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct TestAuthProvider {
    principal: AuthenticatedPrincipal,
}
impl TestAuthProvider {
    pub fn new(principal: AuthenticatedPrincipal) -> Self {
        Self { principal }
    }
}

#[async_trait]
impl AuthProvider for TestAuthProvider {
    async fn authenticate(
        &self,
        _credential: AuthCredential<'_>,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        Ok(self.principal.clone())
    }
}

#[allow(dead_code)]
fn _assert_scopes_send_sync(_: BTreeSet<String>) {}
