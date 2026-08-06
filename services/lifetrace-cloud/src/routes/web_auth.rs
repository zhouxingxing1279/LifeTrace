//! Browser session and account-management endpoints using an HttpOnly cookie and CSRF.

use axum::extract::{Path, State};
use axum::http::header::SET_COOKIE;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use lifetrace_contracts::auth::v1::{
    AcceptedResponseV1, CsrfResponseV1, DeviceInstallationV1, DeviceListV1,
    SessionListV1, UpdateDeviceRequestV1, WebLoginRequestV1, WebSessionResponseV1,
};

use crate::auth::security::{
    build_session_cookie, clear_session_cookie, cookie_value, PeerAddr, RequestContext,
};
use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/web/session/login", post(login))
        .route("/api/v1/web/session", get(session))
        .route("/api/v1/web/session/rotate", post(rotate))
        .route("/api/v1/web/session/logout", post(logout))
        .route("/api/v1/web/csrf", get(csrf))
        .route("/api/v1/web/devices", get(list_devices))
        .route(
            "/api/v1/web/devices/{device_id}",
            patch(update_device),
        )
        .route(
            "/api/v1/web/devices/{device_id}/revoke",
            post(revoke_device),
        )
        .route("/api/v1/web/sessions", get(list_sessions))
        .route(
            "/api/v1/web/sessions/{session_id}",
            delete(revoke_session),
        )
}

fn context(state: &AppState, headers: &HeaderMap, peer: PeerAddr) -> RequestContext {
    RequestContext::from_headers(headers, peer.0, &state.config)
}

async fn web_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(AuthenticatedPrincipal, String), ApiError> {
    let raw = cookie_value(headers, &state.config.auth_cookie_name);
    let principal = state
        .auth
        .authenticate(AuthCredential::WebSession(raw.as_deref()))
        .await?;
    Ok((principal, raw.unwrap_or_default()))
}

fn csrf_header(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
}

async fn verified_web_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedPrincipal, ApiError> {
    let raw_session = cookie_value(headers, &state.config.auth_cookie_name).unwrap_or_default();
    let csrf = csrf_header(headers).unwrap_or_default();
    let origin = headers.get("origin").and_then(|value| value.to_str().ok());
    state
        .auth_service
        .verify_web_csrf(&raw_session, csrf, origin)
        .await
}

async fn login(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<WebLoginRequestV1>,
) -> Result<impl IntoResponse, ApiError> {
    let (body, raw_cookie) = state
        .auth_service
        .web_login(request, &context(&state, &headers, peer))
        .await?;
    let max_age = if body.session.public_device {
        state.config.auth_public_device_ttl_seconds
    } else {
        state.config.auth_web_absolute_ttl_seconds
    };
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        build_session_cookie(&state.config, &raw_cookie, max_age),
    );
    Ok(response)
}

async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WebSessionResponseV1>, ApiError> {
    let (principal, _) = web_principal(&state, &headers).await?;
    state.auth_service.web_session(&principal).await.map(Json)
}

async fn csrf(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CsrfResponseV1>, ApiError> {
    let (principal, _) = web_principal(&state, &headers).await?;
    let response = state.auth_service.web_session(&principal).await?;
    Ok(Json(CsrfResponseV1 {
        csrf_token: response.csrf_token,
    }))
}

async fn rotate(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let raw_session = cookie_value(&headers, &state.config.auth_cookie_name).unwrap_or_default();
    let principal = verified_web_principal(&state, &headers).await?;
    let (body, next_cookie) = state
        .auth_service
        .rotate_web_session(&principal, &raw_session, &context(&state, &headers, peer))
        .await?;
    let max_age = if body.session.public_device {
        state.config.auth_public_device_ttl_seconds
    } else {
        state.config.auth_web_absolute_ttl_seconds
    };
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        build_session_cookie(&state.config, &next_cookie, max_age),
    );
    Ok(response)
}

async fn logout(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let principal = verified_web_principal(&state, &headers).await?;
    let body: AcceptedResponseV1 = state
        .auth_service
        .web_logout(&principal, &context(&state, &headers, peer))
        .await?;
    let mut response = Json(body).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, clear_session_cookie(&state.config));
    Ok(response)
}

async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DeviceListV1>, ApiError> {
    let (principal, _) = web_principal(&state, &headers).await?;
    state.auth_service.list_devices(&principal).await.map(Json)
}

async fn update_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<UpdateDeviceRequestV1>,
) -> Result<Json<DeviceInstallationV1>, ApiError> {
    let principal = verified_web_principal(&state, &headers).await?;
    state
        .auth_service
        .update_device(&principal, &device_id, request)
        .await
        .map(Json)
}

async fn revoke_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    let principal = verified_web_principal(&state, &headers).await?;
    state
        .auth_service
        .revoke_device(&principal, &device_id, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionListV1>, ApiError> {
    let (principal, _) = web_principal(&state, &headers).await?;
    state.auth_service.list_sessions(&principal).await.map(Json)
}

async fn revoke_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    let principal = verified_web_principal(&state, &headers).await?;
    state
        .auth_service
        .revoke_session(&principal, &session_id, &context(&state, &headers, peer))
        .await
        .map(Json)
}
