use std::collections::BTreeSet;

use async_trait::async_trait;
use axum::http::StatusCode;
use chrono::Utc;
use lifetrace_contracts::auth::v1::{AppInstallationId, AuthSessionId};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::{ErrorCode, UserId};
use sqlx::{PgPool, Row};

use crate::auth::token::{TokenKind, TokenManager};
use crate::auth::{AuthCredential, AuthMethod, AuthProvider, AuthenticatedPrincipal};
use crate::error::ApiError;

#[derive(Clone)]
pub struct DatabaseAuthProvider {
    pool: PgPool,
    tokens: TokenManager,
}

impl DatabaseAuthProvider {
    pub fn new(pool: PgPool, tokens: TokenManager) -> Self {
        Self { pool, tokens }
    }

    fn error(code: ErrorCode, message: &'static str) -> ApiError {
        ApiError::new(code, message, StatusCode::UNAUTHORIZED)
    }
}

#[async_trait]
impl AuthProvider for DatabaseAuthProvider {
    async fn authenticate(
        &self,
        credential: AuthCredential<'_>,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        let raw = match credential {
            AuthCredential::Bearer(Some(value)) => value.strip_prefix("Bearer ").map(str::trim),
            AuthCredential::WebSession(Some(value)) => Some(value),
            _ => None,
        }
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Self::error(ErrorCode::AuthRequired, "authentication required"))?;

        let (kind, method) = match credential {
            AuthCredential::Bearer(_) => (TokenKind::Access, AuthMethod::AccessToken),
            AuthCredential::WebSession(_) => (TokenKind::WebSession, AuthMethod::WebSession),
        };
        let parsed = self
            .tokens
            .parse(kind, raw)
            .ok_or_else(|| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        let row = if kind == TokenKind::Access {
            sqlx::query(
                "SELECT at.token_hash, at.expires_at, at.revoked_at, s.id AS session_id, s.user_id, s.device_id, s.app_id, s.scopes, \
                        s.status AS session_status, s.idle_expires_at, s.absolute_expires_at, \
                        u.status AS user_status, u.auth_state, d.status AS device_status, d.revoked_at AS device_revoked_at, \
                        g.status AS grant_status, g.scopes AS grant_scopes \
                 FROM auth_access_tokens at \
                 JOIN auth_sessions s ON s.id = at.session_id \
                 JOIN cloud_users u ON u.id = s.user_id \
                 JOIN cloud_devices d ON d.id = s.device_id \
                 JOIN auth_app_grants g ON g.user_id = s.user_id AND g.app_id = s.app_id \
                 WHERE at.id = $1"
            ).bind(parsed.id).fetch_optional(&self.pool).await
        } else {
            sqlx::query(
                "SELECT ws.token_hash, ws.expires_at, ws.revoked_at, s.id AS session_id, s.user_id, s.device_id, s.app_id, s.scopes, \
                        s.status AS session_status, s.idle_expires_at, s.absolute_expires_at, \
                        u.status AS user_status, u.auth_state, d.status AS device_status, d.revoked_at AS device_revoked_at, \
                        g.status AS grant_status, g.scopes AS grant_scopes \
                 FROM auth_web_sessions ws \
                 JOIN auth_sessions s ON s.id = ws.session_id \
                 JOIN cloud_users u ON u.id = s.user_id \
                 JOIN cloud_devices d ON d.id = s.device_id \
                 JOIN auth_app_grants g ON g.user_id = s.user_id AND g.app_id = s.app_id \
                 WHERE ws.id = $1"
            ).bind(parsed.id).fetch_optional(&self.pool).await
        }.map_err(|error| ApiError::new(ErrorCode::TemporarilyUnavailable, error.to_string(), StatusCode::SERVICE_UNAVAILABLE))?
            .ok_or_else(|| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;

        let hash: Vec<u8> = row
            .try_get("token_hash")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        if !self.tokens.verify(kind, &parsed, &hash) {
            return Err(Self::error(ErrorCode::AuthInvalid, "invalid credential"));
        }
        let now = Utc::now();
        let expires_at: chrono::DateTime<Utc> = row
            .try_get("expires_at")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        if expires_at <= now {
            return Err(Self::error(
                if kind == TokenKind::Access {
                    ErrorCode::AuthAccessTokenExpired
                } else {
                    ErrorCode::AuthSessionRevoked
                },
                "credential expired",
            ));
        }
        if row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .ok()
            .flatten()
            .is_some()
        {
            return Err(Self::error(
                ErrorCode::AuthSessionRevoked,
                "credential revoked",
            ));
        }
        if row
            .try_get::<String, _>("session_status")
            .unwrap_or_default()
            != "active"
        {
            return Err(Self::error(
                ErrorCode::AuthSessionRevoked,
                "session revoked",
            ));
        }
        let idle: chrono::DateTime<Utc> = row
            .try_get("idle_expires_at")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        let absolute: chrono::DateTime<Utc> = row
            .try_get("absolute_expires_at")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        if idle <= now || absolute <= now {
            return Err(Self::error(
                ErrorCode::AuthSessionRevoked,
                "session expired",
            ));
        }
        if row.try_get::<String, _>("user_status").unwrap_or_default() != "active"
            || row.try_get::<String, _>("auth_state").unwrap_or_default() == "disabled"
        {
            return Err(Self::error(ErrorCode::AuthUserDisabled, "user disabled"));
        }
        if row
            .try_get::<String, _>("device_status")
            .unwrap_or_default()
            != "active"
            || row
                .try_get::<Option<chrono::DateTime<Utc>>, _>("device_revoked_at")
                .ok()
                .flatten()
                .is_some()
        {
            return Err(Self::error(ErrorCode::AuthDeviceRevoked, "device revoked"));
        }
        if row.try_get::<String, _>("grant_status").unwrap_or_default() != "active" {
            return Err(Self::error(
                ErrorCode::AuthAppRevoked,
                "application grant revoked",
            ));
        }
        let session_scopes: Vec<String> = row.try_get("scopes").unwrap_or_default();
        let grant_scopes: BTreeSet<String> = row
            .try_get::<Vec<String>, _>("grant_scopes")
            .unwrap_or_default()
            .into_iter()
            .collect();
        let scopes = session_scopes
            .into_iter()
            .filter(|scope| grant_scopes.contains(scope))
            .collect();
        let session_id: uuid::Uuid = row
            .try_get("session_id")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        let user_id: uuid::Uuid = row
            .try_get("user_id")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        let device_id: uuid::Uuid = row
            .try_get("device_id")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;
        let app_id: String = row
            .try_get("app_id")
            .map_err(|_| Self::error(ErrorCode::AuthInvalid, "invalid credential"))?;

        if kind == TokenKind::Access {
            let _ = sqlx::query("UPDATE auth_access_tokens SET last_used_at = now() WHERE id = $1")
                .bind(parsed.id)
                .execute(&self.pool)
                .await;
        }
        let _ = sqlx::query("UPDATE auth_sessions SET last_seen_at = now() WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await;

        Ok(AuthenticatedPrincipal {
            user_id: UserId::new(user_id.to_string()),
            session_id: AuthSessionId::new(session_id.to_string()),
            device_id: AppInstallationId::new(device_id.to_string()),
            app_id: AppId::new(app_id),
            scopes,
            auth_method: method,
        })
    }
}
