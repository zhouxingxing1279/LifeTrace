//! Native account, token, session, device and application-grant endpoints.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use lifetrace_contracts::auth::v1::*;

use crate::auth::security::{PeerAddr, RequestContext};
use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/auth/capabilities", get(capabilities))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/password/forgot", post(forgot_password))
        .route("/api/v1/auth/password/reset", post(reset_password))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/logout-all", post(logout_all))
        .route("/api/v1/auth/password/change", post(change_password))
        .route("/api/v1/auth/sessions", get(list_sessions))
        .route("/api/v1/auth/sessions/{session_id}", delete(revoke_session))
        .route("/api/v1/auth/devices", get(list_devices))
        .route("/api/v1/auth/devices/{device_id}", patch(update_device))
        .route(
            "/api/v1/auth/devices/{device_id}/revoke",
            post(revoke_device),
        )
        .route(
            "/api/v1/auth/device-groups/{device_group_id}/revoke",
            post(revoke_device_group),
        )
        .route("/api/v1/auth/apps", get(list_grants))
        .route(
            "/api/v1/auth/apps/{app_id}/grant",
            patch(update_grant).delete(revoke_grant),
        )
}

fn context(state: &AppState, headers: &HeaderMap, peer: PeerAddr) -> RequestContext {
    RequestContext::from_headers(headers, peer.0, &state.config)
}

async fn capabilities(State(state): State<AppState>) -> Json<AuthCapabilitiesV1> {
    Json(state.auth_service.capabilities())
}

async fn register(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<RegisterRequestV1>,
) -> Result<(StatusCode, Json<TokenResponseV1>), ApiError> {
    let result = state
        .auth_service
        .register(request, &context(&state, &headers, peer))
        .await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn login(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<LoginRequestV1>,
) -> Result<Json<TokenResponseV1>, ApiError> {
    state
        .auth_service
        .login(request, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn refresh(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<RefreshRequestV1>,
) -> Result<Json<TokenResponseV1>, ApiError> {
    state
        .auth_service
        .refresh(request, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn forgot_password(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<ForgotPasswordRequestV1>,
) -> Result<(StatusCode, Json<AcceptedResponseV1>), ApiError> {
    let result = state
        .auth_service
        .forgot_password(request, &context(&state, &headers, peer))
        .await?;
    Ok((StatusCode::ACCEPTED, Json(result)))
}

async fn reset_password(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<ResetPasswordRequestV1>,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .reset_password(request, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn me(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<AuthUserV1>, ApiError> {
    state.auth_service.me(&principal).await.map(Json)
}

async fn logout(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .logout(&principal, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn logout_all(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .logout_all(&principal, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn change_password(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequestV1>,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .change_password(&principal, request, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn list_sessions(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<SessionListV1>, ApiError> {
    state.auth_service.list_sessions(&principal).await.map(Json)
}

async fn revoke_session(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(session_id): Path<String>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .revoke_session(&principal, &session_id, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn list_devices(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<DeviceListV1>, ApiError> {
    state.auth_service.list_devices(&principal).await.map(Json)
}

async fn update_device(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(device_id): Path<String>,
    Json(request): Json<UpdateDeviceRequestV1>,
) -> Result<Json<DeviceInstallationV1>, ApiError> {
    state
        .auth_service
        .update_device(&principal, &device_id, request)
        .await
        .map(Json)
}

async fn revoke_device(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(device_id): Path<String>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .revoke_device(&principal, &device_id, &context(&state, &headers, peer))
        .await
        .map(Json)
}

async fn revoke_device_group(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(device_group_id): Path<String>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .revoke_device_group(
            &principal,
            &device_group_id,
            &context(&state, &headers, peer),
        )
        .await
        .map(Json)
}

async fn list_grants(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<AppGrantListV1>, ApiError> {
    state.auth_service.list_grants(&principal).await.map(Json)
}

async fn update_grant(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(app_id): Path<String>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<UpdateAppGrantRequestV1>,
) -> Result<Json<AppGrantV1>, ApiError> {
    state
        .auth_service
        .update_grant(
            &principal,
            &app_id,
            request,
            &context(&state, &headers, peer),
        )
        .await
        .map(Json)
}

async fn revoke_grant(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(app_id): Path<String>,
    peer: PeerAddr,
    headers: HeaderMap,
) -> Result<Json<AcceptedResponseV1>, ApiError> {
    state
        .auth_service
        .revoke_grant(&principal, &app_id, &context(&state, &headers, peer))
        .await
        .map(Json)
}
