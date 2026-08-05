from pathlib import Path


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


if "Option<ConnectInfo<SocketAddr>>" in auth.read_text() or "Option<ConnectInfo<SocketAddr>>" in web.read_text():
    raise SystemExit("not all optional ConnectInfo extractors were replaced")
