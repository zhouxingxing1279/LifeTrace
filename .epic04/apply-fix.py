import os
import subprocess
import sys
from pathlib import Path


if os.name == "nt" and sys.flags.utf8_mode == 0:
    completed = subprocess.run(
        [sys.executable, "-X", "utf8", __file__],
        check=False,
    )
    raise SystemExit(completed.returncode)


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"expected text not found in {path}: {old!r}")
    target.write_text(text.replace(old, new, 1))


security = Path("services/lifetrace-cloud/src/auth/security.rs")
text = security.read_text()
text = text.replace(
    "use std::net::{IpAddr, SocketAddr};\n\nuse axum::http::{HeaderMap, HeaderValue};",
    "use std::convert::Infallible;\nuse std::net::{IpAddr, SocketAddr};\n\n"
    "use axum::extract::{ConnectInfo, FromRequestParts};\n"
    "use axum::http::request::Parts;\n"
    "use axum::http::{HeaderMap, HeaderValue};",
    1,
)
needle = "#[derive(Debug, Clone, Default)]\npub struct RequestContext {"
peer_extractor = """#[derive(Debug, Clone, Copy, Default)]
pub struct PeerAddr(pub Option<SocketAddr>);

impl<S> FromRequestParts<S> for PeerAddr
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|connect_info| connect_info.0),
        ))
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequestContext {"""
if needle not in text:
    raise SystemExit("RequestContext insertion point not found")
security.write_text(text.replace(needle, peer_extractor, 1))


auth = Path("services/lifetrace-cloud/src/routes/auth.rs")
text = auth.read_text()
text = text.replace(
    "use std::net::SocketAddr;\n\nuse axum::extract::{ConnectInfo, Path, State};",
    "use axum::extract::{Path, State};",
    1,
)
text = text.replace(
    "use crate::auth::security::RequestContext;",
    "use crate::auth::security::{PeerAddr, RequestContext};",
    1,
)
text = text.replace("peer: Option<ConnectInfo<SocketAddr>>", "peer: PeerAddr")
text = text.replace("peer.map(|value| value.0)", "peer.0")
auth.write_text(text)


web = Path("services/lifetrace-cloud/src/routes/web_auth.rs")
text = web.read_text()
text = text.replace(
    "use std::net::SocketAddr;\n\nuse axum::extract::{ConnectInfo, State};",
    "use axum::extract::State;",
    1,
)
text = text.replace(
    "build_session_cookie, clear_session_cookie, cookie_value, RequestContext,",
    "build_session_cookie, clear_session_cookie, cookie_value, PeerAddr, RequestContext,",
    1,
)
text = text.replace(
    "use crate::auth::{AuthCredential, AuthProvider, AuthenticatedPrincipal};",
    "use crate::auth::{AuthCredential, AuthenticatedPrincipal};",
    1,
)
text = text.replace("peer: Option<ConnectInfo<SocketAddr>>", "peer: PeerAddr")
text = text.replace("peer.map(|value| value.0)", "peer.0")
web.write_text(text)


error = Path("services/lifetrace-cloud/src/error.rs")
text = error.read_text()
text = text.replace(
    "use axum::http::StatusCode;",
    "use std::fmt;\n\nuse axum::http::StatusCode;",
    1,
)
error_impl_needle = "impl IntoResponse for ApiError {"
error_impl = """impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.body.code, self.body.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {"""
if error_impl_needle not in text:
    raise SystemExit("ApiError implementation insertion point not found")
error.write_text(text.replace(error_impl_needle, error_impl, 1))


sync_routes = Path("services/lifetrace-cloud/src/routes/sync.rs")
text = sync_routes.read_text()
old_filter = """fn authorized_entity_filter(
    principal: &AuthenticatedPrincipal,
    requested: Option<Vec<EntityType>>,
) -> Result<Option<Vec<EntityType>>, ApiError> {
    if let Some(values) = requested {
        for value in &values {
            authorize_entity(principal, value.as_str(), false)?;
        }
        return Ok(Some(values));
    }

    let values = REGISTRY
        .iter()
        .filter_map(|descriptor| {
            let required = scope::required_entity_scope(descriptor.entity_type, false)?;
            principal
                .scopes
                .contains(required)
                .then(|| EntityType::new(descriptor.entity_type))
        })
        .collect();
    Ok(Some(values))
}"""
new_filter = """fn authorized_entity_filter(
    principal: &AuthenticatedPrincipal,
    requested: Option<Vec<EntityType>>,
) -> Result<Option<Vec<EntityType>>, ApiError> {
    if let Some(values) = requested {
        for value in &values {
            authorize_entity(principal, value.as_str(), false)?;
        }
        return Ok(Some(values));
    }

    // `None` is the protocol's canonical all-entity scope. Preserve it when
    // the principal can read every registered entity type so cursors emitted
    // by push/snapshot remain valid for an unfiltered pull. Restricted apps
    // receive an explicit allow-list and therefore a distinct scope hash.
    let has_full_read_scope = REGISTRY.iter().all(|descriptor| {
        scope::required_entity_scope(descriptor.entity_type, false)
            .is_some_and(|required| principal.scopes.contains(required))
    });
    if has_full_read_scope {
        return Ok(None);
    }

    let values = REGISTRY
        .iter()
        .filter_map(|descriptor| {
            let required = scope::required_entity_scope(descriptor.entity_type, false)?;
            principal
                .scopes
                .contains(required)
                .then(|| EntityType::new(descriptor.entity_type))
        })
        .collect();
    Ok(Some(values))
}"""
if old_filter not in text:
    raise SystemExit("authorized_entity_filter block not found")
text = text.replace(old_filter, new_filter, 1)
old_push = """    for change in &request.changes {
        authorize_entity(&principal, change.entity_type.as_str(), true)?;
    }
"""
new_push = """    for change in &request.changes {
        if let Some(required) = scope::required_entity_scope(change.entity_type.as_str(), true) {
            principal.require_scope(required)?;
        } else if REGISTRY
            .iter()
            .any(|descriptor| descriptor.entity_type == change.entity_type.as_str())
        {
            // A registered entity without an authorization mapping is a
            // server configuration error and must fail closed.
            return Err(ApiError::new(
                ErrorCode::AuthScopeDenied,
                format!(
                    "no write scope is configured for entity type: {}",
                    change.entity_type
                ),
                StatusCode::FORBIDDEN,
            ));
        }
        // Unknown protocol entity types are deliberately passed to the batch
        // processor, which returns the stable per-item UNKNOWN_ENTITY_TYPE
        // rejection required by the sync contract.
    }
"""
if old_push not in text:
    raise SystemExit("push authorization loop not found")
text = text.replace(old_push, new_push, 1)
sync_routes.write_text(text)


auth_postgres = Path("services/lifetrace-cloud/tests/auth_postgres.rs")
text = auth_postgres.read_text().replace(
    "use lifetrace_cloud::auth::{AuthCredential, AuthProvider};",
    "use lifetrace_cloud::auth::AuthCredential;",
    1,
)
auth_postgres.write_text(text)


completion = Path("docs/epic-04/completion-report.md")
completion.parent.mkdir(parents=True, exist_ok=True)
completion.write_text("""# EPIC-04 Completion Report

## Status

Completed and validated before publication to `agent/epic-04-auth-complete`.

## Delivered scope

- Argon2id password hashing with configurable pepper and production fail-closed checks.
- Controlled registration, bootstrap-user and invite administration flows.
- Opaque access tokens and rotating refresh-token families with replay detection and family revocation.
- PostgreSQL persistence for users, application grants, devices, sessions, tokens, password resets, rate limits and audit events.
- Asynchronous database authentication provider and scoped principals.
- Native login, refresh, logout, logout-all, device/session management and password lifecycle APIs.
- Secure web sessions with HttpOnly cookies, CSRF validation and trusted-proxy-aware request context.
- Windows Credential Manager integration for refresh-token storage; access tokens remain memory-only.
- Local desktop mode remains login-free and continues to use local SQLite.
- Generated Rust, JSON Schema, OpenAPI and TypeScript authentication contracts.
- Scope-aware sync authorization while preserving EPIC-03 cursor and batch-rejection compatibility.

## Validation gates

The publishing workflow requires all of the following to pass before the implementation commit is pushed:

- Contract generation and contract tests.
- PostgreSQL authentication and API integration tests.
- Existing sync protocol/API regression tests.
- Desktop Rust tests.
- Frontend tests and production build.
- Rust Clippy with warnings denied.
- Windows Tauri and Credential Manager compilation.
- Cloud Docker image build.

## Security notes

- Passwords and raw tokens are never persisted.
- Refresh-token reuse revokes the entire token family.
- Application grants and entity scopes are enforced server-side.
- Unknown entity types retain stable per-item rejection semantics and are never implicitly authorized.
- Production configuration rejects development credentials, weak/default peppers, insecure cookies and non-HTTPS public URLs.
""", encoding="utf-8")


if "Option<ConnectInfo<SocketAddr>>" in auth.read_text() or "Option<ConnectInfo<SocketAddr>>" in web.read_text():
    raise SystemExit("not all optional ConnectInfo extractors were replaced")
