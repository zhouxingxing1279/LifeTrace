//! Browser session and account-management endpoints using an HttpOnly cookie and CSRF.

use axum::extract::{Path, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use lifetrace_contracts::auth::v1::{
    AcceptedResponseV1, CsrfResponseV1, DeviceInstallationV1, DeviceListV1, Scope,
    SessionListV1, UpdateDeviceRequestV1, WebLoginRequestV1, WebSessionResponseV1,
};
use lifetrace_contracts::ErrorCode;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::auth::password::PasswordManager;
use crate::auth::security::{
    build_session_cookie, clear_session_cookie, cookie_value, PeerAddr, RequestContext,
};
use crate::auth::token::TokenKind;
use crate::auth::{AuthCredential, AuthenticatedPrincipal, AuthService};
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebRegisterRequest {
    email: String,
    password: String,
    display_name: Option<String>,
    invite_token: Option<String>,
    #[serde(default)]
    requested_scopes: Vec<Scope>,
    #[serde(default)]
    public_device: bool,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/web/session/register", post(register))
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

fn auth_error(code: ErrorCode, message: impl Into<String>, status: StatusCode) -> ApiError {
    ApiError::new(code, message, status)
}

fn database_error(error: sqlx::Error) -> ApiError {
    auth_error(
        ErrorCode::TemporarilyUnavailable,
        format!("authentication database operation failed: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

async fn consume_invite(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    raw: Option<&str>,
    normalized_email: &str,
) -> Result<(), ApiError> {
    if state.config.auth_registration_mode != "invite" {
        return Ok(());
    }
    let raw = raw.filter(|value| !value.trim().is_empty()).ok_or_else(|| {
        auth_error(
            ErrorCode::AuthInviteInvalid,
            "registration invite is required",
            StatusCode::FORBIDDEN,
        )
    })?;
    let tokens = state.auth_service.token_manager();
    let parsed = tokens.parse(TokenKind::Invite, raw).ok_or_else(|| {
        auth_error(
            ErrorCode::AuthInviteInvalid,
            "invalid registration invite",
            StatusCode::FORBIDDEN,
        )
    })?;
    let row = sqlx::query(
        "SELECT token_hash,email_normalized,expires_at,used_at,revoked_at FROM auth_registration_invites WHERE id=$1 FOR UPDATE",
    )
    .bind(parsed.id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(database_error)?
    .ok_or_else(|| {
        auth_error(
            ErrorCode::AuthInviteInvalid,
            "invalid registration invite",
            StatusCode::FORBIDDEN,
        )
    })?;
    let expected: Vec<u8> = row.try_get("token_hash").unwrap_or_default();
    let restricted_email: Option<String> = row.try_get("email_normalized").unwrap_or(None);
    let expires_at: DateTime<Utc> = row.try_get("expires_at").unwrap_or_else(|_| Utc::now());
    let used_at: Option<DateTime<Utc>> = row.try_get("used_at").unwrap_or(None);
    let revoked_at: Option<DateTime<Utc>> = row.try_get("revoked_at").unwrap_or(None);
    let unavailable = used_at.is_some()
        || revoked_at.is_some()
        || expires_at <= Utc::now()
        || restricted_email
            .as_deref()
            .is_some_and(|value| value != normalized_email)
        || !tokens.verify(TokenKind::Invite, &parsed, &expected);
    if unavailable {
        return Err(auth_error(
            ErrorCode::AuthInviteInvalid,
            "invalid registration invite",
            StatusCode::FORBIDDEN,
        ));
    }
    sqlx::query("UPDATE auth_registration_invites SET used_at=now() WHERE id=$1")
        .bind(parsed.id)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    Ok(())
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

async fn register(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<WebRegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if state.config.auth_registration_mode == "disabled" {
        return Err(auth_error(
            ErrorCode::AuthRegistrationDisabled,
            "registration is disabled",
            StatusCode::FORBIDDEN,
        ));
    }
    let normalized_email = AuthService::normalize_email(&request.email);
    if normalized_email.is_empty() || !normalized_email.contains('@') {
        return Err(auth_error(
            ErrorCode::InvalidRequest,
            "invalid email",
            StatusCode::BAD_REQUEST,
        ));
    }
    let display_name = request
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let password_hash = PasswordManager::new(&state.config).hash(&request.password)?;
    let user_id = Uuid::new_v4();
    let request_context = context(&state, &headers, peer);
    let mut tx = state.pool.begin().await.map_err(database_error)?;
    consume_invite(
        &state,
        &mut tx,
        request.invite_token.as_deref(),
        &normalized_email,
    )
    .await?;
    let inserted = sqlx::query(
        "INSERT INTO cloud_users (id,status,email,email_normalized,display_name,password_hash,password_version,password_changed_at,registration_source,auth_state) \
         VALUES ($1,'active',$2,$3,$4,$5,1,now(),$6,'active') ON CONFLICT (email_normalized) DO NOTHING",
    )
    .bind(user_id)
    .bind(request.email.trim())
    .bind(&normalized_email)
    .bind(&display_name)
    .bind(password_hash)
    .bind(if state.config.auth_registration_mode == "invite" {
        "invite"
    } else {
        "open"
    })
    .execute(&mut *tx)
    .await
    .map_err(database_error)?;
    if inserted.rows_affected() != 1 {
        return Err(auth_error(
            ErrorCode::InvalidRequest,
            "account already exists",
            StatusCode::CONFLICT,
        ));
    }
    sqlx::query(
        "INSERT INTO auth_audit_log (user_id,session_id,device_id,app_id,event_type,outcome,ip_address,user_agent,metadata) \
         VALUES ($1,NULL,NULL,'web','account.register','success',CAST($2 AS inet),$3,$4)",
    )
    .bind(user_id)
    .bind(request_context.ip.map(|value| value.to_string()))
    .bind(&request_context.user_agent)
    .bind(json!({"registrationMode": state.config.auth_registration_mode.clone()}))
    .execute(&mut *tx)
    .await
    .map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;

    let login_request = WebLoginRequestV1 {
        email: request.email,
        password: request.password,
        requested_scopes: request.requested_scopes,
        public_device: request.public_device,
    };
    let (body, raw_cookie) = state
        .auth_service
        .web_login(login_request, &request_context)
        .await?;
    let max_age = if body.session.public_device {
        state.config.auth_public_device_ttl_seconds
    } else {
        state.config.auth_web_absolute_ttl_seconds
    };
    let mut response = (StatusCode::CREATED, Json(body)).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        build_session_cookie(&state.config, &raw_cookie, max_age),
    );
    Ok(response)
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
