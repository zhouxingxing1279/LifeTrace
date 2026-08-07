use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use lifetrace_contracts::auth::v1::*;
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::{ErrorCode, UserId};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::auth::password::PasswordManager;
use crate::auth::scope;
use crate::auth::security::{csrf_matches, origin_allowed, RequestContext};
use crate::auth::token::{GeneratedToken, TokenKind, TokenManager};
use crate::auth::AuthenticatedPrincipal;
use crate::config::Config;
use crate::error::ApiError;

#[derive(Clone)]
pub struct AuthService {
    pool: PgPool,
    config: Arc<Config>,
    passwords: PasswordManager,
    tokens: TokenManager,
    dummy_hash: Arc<String>,
    development_reset_token: Arc<Mutex<Option<String>>>,
}

#[derive(Clone)]
struct LoginInput {
    email: String,
    password: String,
    app_id: String,
    external_device_id: String,
    device_name: String,
    platform: String,
    client_version: Option<String>,
    requested_scopes: Vec<Scope>,
    public_device: bool,
}

#[derive(Clone)]
struct VerifiedLogin {
    user_id: Uuid,
    user: AuthUserV1,
    device_id: Uuid,
    app_id: String,
    scopes: Vec<String>,
}

impl AuthService {
    pub fn new(pool: PgPool, config: Config) -> Self {
        let passwords = PasswordManager::new(&config);
        let dummy_hash = passwords
            .hash("LifeTrace dummy password phrase 2026")
            .unwrap_or_else(|_| String::new());
        Self {
            pool,
            config: Arc::new(config.clone()),
            passwords,
            tokens: TokenManager::new(&config),
            dummy_hash: Arc::new(dummy_hash),
            development_reset_token: Arc::new(Mutex::new(None)),
        }
    }

    pub fn token_manager(&self) -> TokenManager {
        self.tokens.clone()
    }

    pub fn capabilities(&self) -> AuthCapabilitiesV1 {
        AuthCapabilitiesV1 {
            registration_mode: self.config.auth_registration_mode.clone(),
            password_min_length: self.config.auth_password_min_length as u32,
            password_max_bytes: self.config.auth_password_max_bytes as u32,
            access_token_ttl_seconds: self.config.auth_access_token_ttl_seconds,
            refresh_idle_ttl_seconds: self.config.auth_refresh_idle_ttl_seconds,
            refresh_absolute_ttl_seconds: self.config.auth_refresh_absolute_ttl_seconds,
            web_session_enabled: true,
            supported_apps: [
                AppId::DESKTOP,
                AppId::FINANCE_ANDROID,
                AppId::NOTES_ANDROID,
                AppId::ENGLISH_ANDROID,
                AppId::HABITS_ANDROID,
                AppId::WEB,
            ]
            .into_iter()
            .map(AppId::new)
            .collect(),
        }
    }

    pub fn normalize_email(value: &str) -> String {
        value.trim().to_lowercase()
    }

    fn error(code: ErrorCode, message: impl Into<String>, status: StatusCode) -> ApiError {
        ApiError::new(code, message, status)
    }

    fn db(error: sqlx::Error) -> ApiError {
        Self::error(
            ErrorCode::TemporarilyUnavailable,
            format!("authentication database operation failed: {error}"),
            StatusCode::SERVICE_UNAVAILABLE,
        )
    }

    fn uuid(value: &str, code: ErrorCode) -> Result<Uuid, ApiError> {
        Uuid::parse_str(value)
            .map_err(|_| Self::error(code, "invalid identifier", StatusCode::BAD_REQUEST))
    }

    fn hash_email(&self, normalized: &str) -> Vec<u8> {
        let mut digest = Sha256::new();
        digest.update(
            self.config
                .auth_token_hash_pepper
                .as_deref()
                .unwrap_or_default()
                .as_bytes(),
        );
        digest.update(normalized.as_bytes());
        digest.finalize().to_vec()
    }

    fn ip(context: &RequestContext) -> Option<String> {
        context.ip.map(|value| value.to_string())
    }

    // Each argument maps one-to-one to a security-audit column. Keeping the
    // fields explicit makes omissions visible at every call site and avoids
    // accepting partially populated, loosely typed metadata structures.
    #[allow(clippy::too_many_arguments)]
    async fn audit<'e, E>(
        &self,
        executor: E,
        user_id: Option<Uuid>,
        session_id: Option<Uuid>,
        device_id: Option<Uuid>,
        app_id: Option<&str>,
        event_type: &str,
        outcome: &str,
        context: &RequestContext,
        metadata: Value,
    ) -> Result<(), ApiError>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            "INSERT INTO auth_audit_log (user_id, session_id, device_id, app_id, event_type, outcome, ip_address, user_agent, metadata) \
             VALUES ($1,$2,$3,$4,$5,$6,CAST($7 AS inet),$8,$9)"
        )
        .bind(user_id).bind(session_id).bind(device_id).bind(app_id).bind(event_type).bind(outcome)
        .bind(Self::ip(context)).bind(&context.user_agent).bind(metadata)
        .execute(executor).await.map_err(Self::db)?;
        Ok(())
    }

    async fn record_login_attempt(
        &self,
        normalized: &str,
        context: &RequestContext,
        succeeded: bool,
        reason: Option<&str>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "INSERT INTO auth_login_attempts (email_hash, ip_address, succeeded, failure_reason) VALUES ($1,CAST($2 AS inet),$3,$4)"
        ).bind(self.hash_email(normalized)).bind(Self::ip(context)).bind(succeeded).bind(reason)
            .execute(&self.pool).await.map_err(Self::db)?;
        Ok(())
    }

    async fn check_rate_limit(
        &self,
        normalized: &str,
        context: &RequestContext,
    ) -> Result<(), ApiError> {
        let since = Utc::now() - Duration::seconds(self.config.auth_login_window_seconds as i64);
        let account_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::BIGINT FROM auth_login_attempts WHERE email_hash=$1 AND succeeded=FALSE AND attempted_at >= $2"
        ).bind(self.hash_email(normalized)).bind(since).fetch_one(&self.pool).await.map_err(Self::db)?;
        let ip_count: i64 = if let Some(ip) = Self::ip(context) {
            sqlx::query_scalar(
                "SELECT COUNT(*)::BIGINT FROM auth_login_attempts WHERE ip_address=CAST($1 AS inet) AND succeeded=FALSE AND attempted_at >= $2"
            ).bind(ip).bind(since).fetch_one(&self.pool).await.map_err(Self::db)?
        } else {
            0
        };
        if account_count as usize >= self.config.auth_login_account_limit
            || ip_count as usize >= self.config.auth_login_ip_limit
        {
            return Err(Self::error(
                ErrorCode::AuthRateLimited,
                "too many authentication attempts",
                StatusCode::TOO_MANY_REQUESTS,
            ));
        }
        Ok(())
    }

    async fn consume_invite(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        raw: Option<&str>,
        normalized: &str,
    ) -> Result<(), ApiError> {
        if self.config.auth_registration_mode != "invite" {
            return Ok(());
        }
        let raw = raw.ok_or_else(|| {
            Self::error(
                ErrorCode::AuthInviteInvalid,
                "registration invite is required",
                StatusCode::FORBIDDEN,
            )
        })?;
        let parsed = self.tokens.parse(TokenKind::Invite, raw).ok_or_else(|| {
            Self::error(
                ErrorCode::AuthInviteInvalid,
                "invalid registration invite",
                StatusCode::FORBIDDEN,
            )
        })?;
        let row = sqlx::query(
            "SELECT token_hash, email_normalized, expires_at, used_at, revoked_at FROM auth_registration_invites WHERE id=$1 FOR UPDATE"
        ).bind(parsed.id).fetch_optional(&mut **tx).await.map_err(Self::db)?
            .ok_or_else(|| Self::error(ErrorCode::AuthInviteInvalid, "invalid registration invite", StatusCode::FORBIDDEN))?;
        let hash: Vec<u8> = row.try_get("token_hash").map_err(|_| {
            Self::error(
                ErrorCode::InternalError,
                "invalid invite record",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        let expires: DateTime<Utc> = row.try_get("expires_at").unwrap_or_else(|_| Utc::now());
        let restricted: Option<String> = row.try_get("email_normalized").ok();
        let unavailable = row
            .try_get::<Option<DateTime<Utc>>, _>("used_at")
            .ok()
            .flatten()
            .is_some()
            || row
                .try_get::<Option<DateTime<Utc>>, _>("revoked_at")
                .ok()
                .flatten()
                .is_some()
            || expires <= Utc::now()
            || restricted
                .as_deref()
                .is_some_and(|value| value != normalized)
            || !self.tokens.verify(TokenKind::Invite, &parsed, &hash);
        if unavailable {
            return Err(Self::error(
                ErrorCode::AuthInviteInvalid,
                "invalid registration invite",
                StatusCode::FORBIDDEN,
            ));
        }
        sqlx::query("UPDATE auth_registration_invites SET used_at=now() WHERE id=$1")
            .bind(parsed.id)
            .execute(&mut **tx)
            .await
            .map_err(Self::db)?;
        Ok(())
    }

    pub async fn register(
        &self,
        request: RegisterRequestV1,
        context: &RequestContext,
    ) -> Result<TokenResponseV1, ApiError> {
        if self.config.auth_registration_mode == "disabled" {
            return Err(Self::error(
                ErrorCode::AuthRegistrationDisabled,
                "registration is disabled",
                StatusCode::FORBIDDEN,
            ));
        }
        if !scope::supported_app(request.app_id.as_str()) || request.app_id.as_str() == AppId::WEB {
            return Err(Self::error(
                ErrorCode::AppIdUnsupported,
                "unsupported registration application",
                StatusCode::BAD_REQUEST,
            ));
        }
        let normalized = Self::normalize_email(&request.email);
        if normalized.is_empty() || !normalized.contains('@') {
            return Err(Self::error(
                ErrorCode::InvalidRequest,
                "invalid email",
                StatusCode::BAD_REQUEST,
            ));
        }
        let password_hash = self.passwords.hash(&request.password)?;
        let user_id = Uuid::new_v4();
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        self.consume_invite(&mut tx, request.invite_token.as_deref(), &normalized)
            .await?;
        let inserted = sqlx::query(
            "INSERT INTO cloud_users (id,status,email,email_normalized,display_name,password_hash,password_version,password_changed_at,registration_source,auth_state) \
             VALUES ($1,'active',$2,$3,$4,$5,1,now(),$6,'active') ON CONFLICT (email_normalized) DO NOTHING"
        ).bind(user_id).bind(request.email.trim()).bind(&normalized).bind(&request.display_name).bind(password_hash)
            .bind(if self.config.auth_registration_mode == "invite" { "invite" } else { "open" })
            .execute(&mut *tx).await.map_err(Self::db)?;
        if inserted.rows_affected() != 1 {
            return Err(Self::error(
                ErrorCode::InvalidRequest,
                "account already exists",
                StatusCode::CONFLICT,
            ));
        }
        self.audit(
            &mut *tx,
            Some(user_id),
            None,
            None,
            Some(request.app_id.as_str()),
            "account.register",
            "success",
            context,
            json!({"registrationMode": self.config.auth_registration_mode}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        self.login(
            LoginRequestV1 {
                email: request.email,
                password: request.password,
                app_id: request.app_id,
                device_id: request.device_id,
                device_name: request.device_name,
                platform: request.platform,
                client_version: request.client_version,
                requested_scopes: request.requested_scopes,
                public_device: false,
            },
            context,
        )
        .await
    }

    pub async fn login(
        &self,
        request: LoginRequestV1,
        context: &RequestContext,
    ) -> Result<TokenResponseV1, ApiError> {
        let input = LoginInput {
            email: request.email,
            password: request.password,
            app_id: request.app_id.as_str().to_owned(),
            external_device_id: request.device_id,
            device_name: request.device_name,
            platform: request.platform,
            client_version: request.client_version,
            requested_scopes: request.requested_scopes,
            public_device: request.public_device,
        };
        let verified = self.verify_and_prepare_login(&input, context).await?;
        self.create_native_session(verified, input.public_device, context)
            .await
    }

    async fn verify_and_prepare_login(
        &self,
        input: &LoginInput,
        context: &RequestContext,
    ) -> Result<VerifiedLogin, ApiError> {
        if !scope::supported_app(&input.app_id) {
            return Err(Self::error(
                ErrorCode::AppIdUnsupported,
                "unsupported application",
                StatusCode::BAD_REQUEST,
            ));
        }
        let normalized = Self::normalize_email(&input.email);
        self.check_rate_limit(&normalized, context).await?;
        let row = sqlx::query(
            "SELECT id,email,display_name,password_hash,status,auth_state,email_verified_at,created_at,password_changed_at,locked_until \
             FROM cloud_users WHERE email_normalized=$1"
        ).bind(&normalized).fetch_optional(&self.pool).await.map_err(Self::db)?;
        let encoded = row
            .as_ref()
            .and_then(|row| {
                row.try_get::<Option<String>, _>("password_hash")
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(|| (*self.dummy_hash).clone());
        let valid = self.passwords.verify(&input.password, &encoded);
        let Some(row) = row else {
            let _ = self
                .record_login_attempt(&normalized, context, false, Some("user_not_found"))
                .await;
            return Err(Self::error(
                ErrorCode::AuthInvalid,
                "邮箱或密码错误",
                StatusCode::UNAUTHORIZED,
            ));
        };
        let user_id: Uuid = row.try_get("id").map_err(|_| {
            Self::error(
                ErrorCode::InternalError,
                "invalid user record",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        if !valid {
            self.record_login_attempt(&normalized, context, false, Some("password_invalid"))
                .await?;
            sqlx::query(
                "UPDATE cloud_users SET failed_login_count=failed_login_count+1, locked_until=CASE WHEN failed_login_count+1 >= $2 THEN now()+make_interval(secs => $3) ELSE locked_until END WHERE id=$1"
            ).bind(user_id).bind(self.config.auth_login_account_limit as i32).bind(self.config.auth_lockout_seconds as f64)
                .execute(&self.pool).await.map_err(Self::db)?;
            return Err(Self::error(
                ErrorCode::AuthInvalid,
                "邮箱或密码错误",
                StatusCode::UNAUTHORIZED,
            ));
        }
        if row
            .try_get::<Option<DateTime<Utc>>, _>("locked_until")
            .ok()
            .flatten()
            .is_some_and(|until| until > Utc::now())
        {
            return Err(Self::error(
                ErrorCode::AuthUserLocked,
                "account is temporarily locked",
                StatusCode::LOCKED,
            ));
        }
        if row.try_get::<String, _>("status").unwrap_or_default() != "active"
            || row.try_get::<String, _>("auth_state").unwrap_or_default() == "disabled"
        {
            return Err(Self::error(
                ErrorCode::AuthUserDisabled,
                "account is disabled",
                StatusCode::FORBIDDEN,
            ));
        }
        self.record_login_attempt(&normalized, context, true, None)
            .await?;
        sqlx::query("UPDATE cloud_users SET failed_login_count=0,locked_until=NULL,updated_at=now() WHERE id=$1").bind(user_id).execute(&self.pool).await.map_err(Self::db)?;
        if self.passwords.needs_rehash(&encoded) {
            if let Ok(new_hash) = self.passwords.hash(&input.password) {
                let _ = sqlx::query("UPDATE cloud_users SET password_hash=$2,password_version=password_version+1 WHERE id=$1").bind(user_id).bind(new_hash).execute(&self.pool).await;
            }
        }
        let user = Self::user_from_row(&row)?;
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        let allowed = scope::default_scopes(&input.app_id);
        let grant_row = sqlx::query(
            "INSERT INTO auth_app_grants (id,user_id,app_id,scopes,status) VALUES ($1,$2,$3,$4,'active') \
             ON CONFLICT (user_id,app_id) DO UPDATE SET updated_at=now() RETURNING id,scopes,status"
        ).bind(Uuid::new_v4()).bind(user_id).bind(&input.app_id).bind(&allowed)
            .fetch_one(&mut *tx).await.map_err(Self::db)?;
        if grant_row.try_get::<String, _>("status").unwrap_or_default() != "active" {
            return Err(Self::error(
                ErrorCode::AuthAppRevoked,
                "application grant is revoked",
                StatusCode::FORBIDDEN,
            ));
        }
        let granted: Vec<String> = grant_row.try_get("scopes").unwrap_or_default();
        let scopes = scope::issue_scopes(&input.app_id, &input.requested_scopes, &granted);
        if scopes.is_empty() {
            return Err(Self::error(
                ErrorCode::AuthScopeDenied,
                "no requested scopes are available",
                StatusCode::FORBIDDEN,
            ));
        }
        let device_id = Uuid::new_v4();
        let device = sqlx::query(
            "INSERT INTO cloud_devices (id,user_id,app_id,platform,client_version,status,external_device_id,device_group_id,device_name,first_seen_at,last_seen_at,last_login_at,last_login_ip,last_user_agent) \
             VALUES ($1,$2,$3,$4,$5,'active',$6,$6,$7,now(),now(),now(),CAST($8 AS inet),$9) \
             ON CONFLICT (user_id,app_id,external_device_id) DO UPDATE SET platform=EXCLUDED.platform,client_version=EXCLUDED.client_version,device_name=EXCLUDED.device_name,last_seen_at=now(),last_login_at=now(),last_login_ip=EXCLUDED.last_login_ip,last_user_agent=EXCLUDED.last_user_agent \
             RETURNING id,status,revoked_at"
        ).bind(device_id).bind(user_id).bind(&input.app_id).bind(&input.platform).bind(&input.client_version)
            .bind(&input.external_device_id).bind(&input.device_name).bind(Self::ip(context)).bind(&context.user_agent)
            .fetch_one(&mut *tx).await.map_err(Self::db)?;
        if device.try_get::<String, _>("status").unwrap_or_default() != "active"
            || device
                .try_get::<Option<DateTime<Utc>>, _>("revoked_at")
                .ok()
                .flatten()
                .is_some()
        {
            return Err(Self::error(
                ErrorCode::AuthDeviceRevoked,
                "device is revoked",
                StatusCode::FORBIDDEN,
            ));
        }
        let device_id: Uuid = device.try_get("id").map_err(|_| {
            Self::error(
                ErrorCode::InternalError,
                "invalid device record",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        tx.commit().await.map_err(Self::db)?;
        Ok(VerifiedLogin {
            user_id,
            user,
            device_id,
            app_id: input.app_id.clone(),
            scopes,
        })
    }

    async fn create_native_session(
        &self,
        verified: VerifiedLogin,
        public_device: bool,
        context: &RequestContext,
    ) -> Result<TokenResponseV1, ApiError> {
        let now = Utc::now();
        let absolute_seconds = if public_device {
            self.config.auth_public_device_ttl_seconds
        } else {
            self.config.auth_refresh_absolute_ttl_seconds
        };
        let idle_seconds = if public_device {
            absolute_seconds
        } else {
            self.config.auth_refresh_idle_ttl_seconds
        };
        let session_id = Uuid::new_v4();
        let access = self.tokens.generate(TokenKind::Access);
        let refresh = (!public_device).then(|| self.tokens.generate(TokenKind::Refresh));
        let family_id = Uuid::new_v4();
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        sqlx::query(
            "INSERT INTO auth_sessions (id,user_id,device_id,app_id,scopes,session_type,status,idle_expires_at,absolute_expires_at,login_ip,last_ip,user_agent,public_device) \
             VALUES ($1,$2,$3,$4,$5,'native','active',$6,$7,CAST($8 AS inet),CAST($8 AS inet),$9,$10)"
        ).bind(session_id).bind(verified.user_id).bind(verified.device_id).bind(&verified.app_id).bind(&verified.scopes)
            .bind(now + Duration::seconds(idle_seconds as i64)).bind(now + Duration::seconds(absolute_seconds as i64))
            .bind(Self::ip(context)).bind(&context.user_agent).bind(public_device)
            .execute(&mut *tx).await.map_err(Self::db)?;
        self.insert_access(&mut tx, session_id, &access, &verified.scopes, now)
            .await?;
        if let Some(refresh) = &refresh {
            self.insert_refresh(
                &mut tx,
                session_id,
                family_id,
                None,
                refresh,
                now,
                now + Duration::seconds(idle_seconds as i64),
                now + Duration::seconds(absolute_seconds as i64),
            )
            .await?;
        }
        self.audit(
            &mut *tx,
            Some(verified.user_id),
            Some(session_id),
            Some(verified.device_id),
            Some(&verified.app_id),
            "auth.login",
            "success",
            context,
            json!({"sessionType":"native","publicDevice":public_device}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(TokenResponseV1 {
            access_token: access.raw,
            refresh_token: refresh.as_ref().map(|value| value.raw.clone()),
            token_type: "Bearer".to_owned(),
            expires_in: self
                .config
                .auth_access_token_ttl_seconds
                .min(absolute_seconds),
            refresh_expires_in: refresh.as_ref().map(|_| absolute_seconds),
            user: verified.user,
            session: self.session_by_id(session_id, Some(session_id)).await?,
            scopes: verified.scopes.into_iter().map(Scope::new).collect(),
        })
    }

    async fn insert_access(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        session_id: Uuid,
        token: &GeneratedToken,
        scopes: &[String],
        now: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        sqlx::query("INSERT INTO auth_access_tokens (id,session_id,token_hash,scopes,expires_at) VALUES ($1,$2,$3,$4,$5)")
            .bind(token.id).bind(session_id).bind(&token.hash).bind(scopes)
            .bind(now + Duration::seconds(self.config.auth_access_token_ttl_seconds as i64))
            .execute(&mut **tx).await.map_err(Self::db)?;
        Ok(())
    }

    // Refresh-token lineage and both expiry boundaries must be persisted in
    // the same transaction. Explicit parameters mirror the immutable row and
    // make accidental omission during token rotation a compile-time error.
    #[allow(clippy::too_many_arguments)]
    async fn insert_refresh(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        session_id: Uuid,
        family_id: Uuid,
        parent: Option<Uuid>,
        token: &GeneratedToken,
        now: DateTime<Utc>,
        idle: DateTime<Utc>,
        absolute: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "INSERT INTO auth_refresh_tokens (id,session_id,family_id,parent_token_id,token_hash,created_at,idle_expires_at,absolute_expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"
        ).bind(token.id).bind(session_id).bind(family_id).bind(parent).bind(&token.hash).bind(now).bind(idle).bind(absolute)
            .execute(&mut **tx).await.map_err(Self::db)?;
        Ok(())
    }

    pub async fn refresh(
        &self,
        request: RefreshRequestV1,
        context: &RequestContext,
    ) -> Result<TokenResponseV1, ApiError> {
        let parsed = self
            .tokens
            .parse(TokenKind::Refresh, &request.refresh_token)
            .ok_or_else(|| {
                Self::error(
                    ErrorCode::AuthInvalid,
                    "invalid refresh token",
                    StatusCode::UNAUTHORIZED,
                )
            })?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        let row = sqlx::query(
            "SELECT rt.token_hash,rt.session_id,rt.family_id,rt.used_at,rt.replaced_by_token_id,rt.revoked_at,rt.idle_expires_at,rt.absolute_expires_at, \
                    s.user_id,s.device_id,s.app_id,s.scopes,s.status AS session_status,s.idle_expires_at AS session_idle,s.absolute_expires_at AS session_absolute,s.public_device, \
                    u.status AS user_status,u.auth_state,d.status AS device_status,d.external_device_id,g.status AS grant_status,g.scopes AS grant_scopes \
             FROM auth_refresh_tokens rt JOIN auth_sessions s ON s.id=rt.session_id JOIN cloud_users u ON u.id=s.user_id \
             JOIN cloud_devices d ON d.id=s.device_id JOIN auth_app_grants g ON g.user_id=s.user_id AND g.app_id=s.app_id \
             WHERE rt.id=$1 FOR UPDATE OF rt,s"
        ).bind(parsed.id).fetch_optional(&mut *tx).await.map_err(Self::db)?
            .ok_or_else(|| Self::error(ErrorCode::AuthInvalid, "invalid refresh token", StatusCode::UNAUTHORIZED))?;
        let expected: Vec<u8> = row.try_get("token_hash").map_err(|_| {
            Self::error(
                ErrorCode::AuthInvalid,
                "invalid refresh token",
                StatusCode::UNAUTHORIZED,
            )
        })?;
        if !self.tokens.verify(TokenKind::Refresh, &parsed, &expected) {
            return Err(Self::error(
                ErrorCode::AuthInvalid,
                "invalid refresh token",
                StatusCode::UNAUTHORIZED,
            ));
        }
        let session_id: Uuid = row.try_get("session_id").unwrap();
        let family_id: Uuid = row.try_get("family_id").unwrap();
        let reused = row
            .try_get::<Option<DateTime<Utc>>, _>("used_at")
            .ok()
            .flatten()
            .is_some()
            || row
                .try_get::<Option<Uuid>, _>("replaced_by_token_id")
                .ok()
                .flatten()
                .is_some();
        if reused {
            sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()),reuse_detected_at=CASE WHEN id=$2 THEN now() ELSE reuse_detected_at END,revoked_reason='refresh_reuse' WHERE family_id=$1")
                .bind(family_id).bind(parsed.id).execute(&mut *tx).await.map_err(Self::db)?;
            sqlx::query("UPDATE auth_access_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason='refresh_reuse' WHERE session_id=$1").bind(session_id).execute(&mut *tx).await.map_err(Self::db)?;
            sqlx::query("UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason='refresh_reuse' WHERE id=$1").bind(session_id).execute(&mut *tx).await.map_err(Self::db)?;
            self.audit(
                &mut *tx,
                row.try_get("user_id").ok(),
                Some(session_id),
                row.try_get("device_id").ok(),
                row.try_get::<String, _>("app_id").ok().as_deref(),
                "auth.refresh_reuse",
                "blocked",
                context,
                json!({"familyId":family_id}),
            )
            .await?;
            tx.commit().await.map_err(Self::db)?;
            return Err(Self::error(
                ErrorCode::AuthRefreshTokenReused,
                "refresh token reuse detected",
                StatusCode::UNAUTHORIZED,
            ));
        }
        let refresh_idle: DateTime<Utc> = row.try_get("idle_expires_at").unwrap_or(now);
        let refresh_absolute: DateTime<Utc> = row.try_get("absolute_expires_at").unwrap_or(now);
        if refresh_idle <= now || refresh_absolute <= now {
            return Err(Self::error(
                ErrorCode::AuthRefreshTokenExpired,
                "refresh token expired",
                StatusCode::UNAUTHORIZED,
            ));
        }
        if row
            .try_get::<Option<DateTime<Utc>>, _>("revoked_at")
            .ok()
            .flatten()
            .is_some()
            || row
                .try_get::<String, _>("session_status")
                .unwrap_or_default()
                != "active"
        {
            return Err(Self::error(
                ErrorCode::AuthSessionRevoked,
                "session revoked",
                StatusCode::UNAUTHORIZED,
            ));
        }
        if row.try_get::<String, _>("user_status").unwrap_or_default() != "active"
            || row.try_get::<String, _>("auth_state").unwrap_or_default() == "disabled"
        {
            return Err(Self::error(
                ErrorCode::AuthUserDisabled,
                "account disabled",
                StatusCode::FORBIDDEN,
            ));
        }
        if row
            .try_get::<String, _>("device_status")
            .unwrap_or_default()
            != "active"
        {
            return Err(Self::error(
                ErrorCode::AuthDeviceRevoked,
                "device revoked",
                StatusCode::FORBIDDEN,
            ));
        }
        if row.try_get::<String, _>("grant_status").unwrap_or_default() != "active" {
            return Err(Self::error(
                ErrorCode::AuthAppRevoked,
                "application grant revoked",
                StatusCode::FORBIDDEN,
            ));
        }
        let app_id: String = row.try_get("app_id").unwrap_or_default();
        if app_id != request.app_id.as_str()
            || row
                .try_get::<String, _>("external_device_id")
                .unwrap_or_default()
                != request.device_id
        {
            return Err(Self::error(
                ErrorCode::AuthInvalid,
                "refresh token binding mismatch",
                StatusCode::UNAUTHORIZED,
            ));
        }
        let grant_scopes: BTreeSet<String> = row
            .try_get::<Vec<String>, _>("grant_scopes")
            .unwrap_or_default()
            .into_iter()
            .collect();
        let scopes: Vec<String> = row
            .try_get::<Vec<String>, _>("scopes")
            .unwrap_or_default()
            .into_iter()
            .filter(|value| grant_scopes.contains(value))
            .collect();
        let next_refresh = self.tokens.generate(TokenKind::Refresh);
        let access = self.tokens.generate(TokenKind::Access);
        let new_idle = now + Duration::seconds(self.config.auth_refresh_idle_ttl_seconds as i64);
        let effective_idle = new_idle.min(refresh_absolute);
        self.insert_refresh(
            &mut tx,
            session_id,
            family_id,
            Some(parsed.id),
            &next_refresh,
            now,
            effective_idle,
            refresh_absolute,
        )
        .await?;
        self.insert_access(&mut tx, session_id, &access, &scopes, now)
            .await?;
        sqlx::query(
            "UPDATE auth_refresh_tokens SET used_at=now(),replaced_by_token_id=$2 WHERE id=$1",
        )
        .bind(parsed.id)
        .bind(next_refresh.id)
        .execute(&mut *tx)
        .await
        .map_err(Self::db)?;
        sqlx::query("UPDATE auth_sessions SET last_seen_at=now(),idle_expires_at=LEAST($2,absolute_expires_at),last_ip=CAST($3 AS inet) WHERE id=$1")
            .bind(session_id).bind(effective_idle).bind(Self::ip(context)).execute(&mut *tx).await.map_err(Self::db)?;
        let user_id: Uuid = row.try_get("user_id").unwrap();
        let device_id: Uuid = row.try_get("device_id").unwrap();
        self.audit(
            &mut *tx,
            Some(user_id),
            Some(session_id),
            Some(device_id),
            Some(&app_id),
            "auth.refresh",
            "success",
            context,
            json!({"familyId":family_id}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(TokenResponseV1 {
            access_token: access.raw,
            refresh_token: Some(next_refresh.raw),
            token_type: "Bearer".to_owned(),
            expires_in: self.config.auth_access_token_ttl_seconds,
            refresh_expires_in: Some((refresh_absolute - now).num_seconds().max(0) as u64),
            user: self.user_by_id(user_id).await?,
            session: self.session_by_id(session_id, Some(session_id)).await?,
            scopes: scopes.into_iter().map(Scope::new).collect(),
        })
    }

    pub async fn me(&self, principal: &AuthenticatedPrincipal) -> Result<AuthUserV1, ApiError> {
        self.user_by_id(Self::uuid(
            principal.user_id.as_str(),
            ErrorCode::AuthInvalid,
        )?)
        .await
    }

    pub async fn logout(
        &self,
        principal: &AuthenticatedPrincipal,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        let session_id = Self::uuid(principal.session_id.as_str(), ErrorCode::AuthInvalid)?;
        self.revoke_session_internal(session_id, "logout", context)
            .await?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    pub async fn logout_all(
        &self,
        principal: &AuthenticatedPrincipal,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason='logout_all' WHERE user_id=$1 AND status='active'").bind(user_id).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_access_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason='logout_all' WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1)").bind(user_id).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason='logout_all' WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1)").bind(user_id).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_web_sessions SET revoked_at=COALESCE(revoked_at,now()) WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1)").bind(user_id).execute(&mut *tx).await.map_err(Self::db)?;
        self.audit(
            &mut *tx,
            Some(user_id),
            None,
            None,
            None,
            "auth.logout_all",
            "success",
            context,
            json!({}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    async fn revoke_session_internal(
        &self,
        session_id: Uuid,
        reason: &str,
        context: &RequestContext,
    ) -> Result<(), ApiError> {
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        let row = sqlx::query("SELECT user_id,device_id,app_id FROM auth_sessions WHERE id=$1")
            .bind(session_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(Self::db)?;
        sqlx::query("UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE id=$1").bind(session_id).bind(reason).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_access_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE session_id=$1").bind(session_id).bind(reason).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE session_id=$1").bind(session_id).bind(reason).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_web_sessions SET revoked_at=COALESCE(revoked_at,now()) WHERE session_id=$1").bind(session_id).execute(&mut *tx).await.map_err(Self::db)?;
        if let Some(row) = row {
            self.audit(
                &mut *tx,
                row.try_get("user_id").ok(),
                Some(session_id),
                row.try_get("device_id").ok(),
                row.try_get::<String, _>("app_id").ok().as_deref(),
                "session.revoke",
                "success",
                context,
                json!({"reason":reason}),
            )
            .await?;
        }
        tx.commit().await.map_err(Self::db)?;
        Ok(())
    }

    pub async fn list_sessions(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<SessionListV1, ApiError> {
        principal.require_scope("sessions:read")?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let current = Self::uuid(principal.session_id.as_str(), ErrorCode::AuthInvalid)?;
        let rows =
            sqlx::query("SELECT id FROM auth_sessions WHERE user_id=$1 ORDER BY created_at DESC")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await
                .map_err(Self::db)?;
        let mut sessions = Vec::with_capacity(rows.len());
        for row in rows {
            sessions.push(
                self.session_by_id(row.try_get("id").unwrap(), Some(current))
                    .await?,
            );
        }
        Ok(SessionListV1 { sessions })
    }

    pub async fn revoke_session(
        &self,
        principal: &AuthenticatedPrincipal,
        session: &str,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        principal.require_scope("sessions:write")?;
        let session_id = Self::uuid(session, ErrorCode::InvalidRequest)?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let owner: Option<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM auth_sessions WHERE id=$1")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(Self::db)?;
        if owner != Some(user_id) {
            return Err(Self::error(
                ErrorCode::AuthScopeDenied,
                "session does not belong to the current user",
                StatusCode::FORBIDDEN,
            ));
        }
        self.revoke_session_internal(session_id, "user_revoke", context)
            .await?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    pub async fn list_devices(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<DeviceListV1, ApiError> {
        principal.require_scope("devices:read")?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let current = Self::uuid(principal.device_id.as_str(), ErrorCode::AuthInvalid)?;
        let rows = sqlx::query(
            "SELECT id,external_device_id,device_group_id,device_name,app_id,platform,status,client_version,first_seen_at,last_seen_at,last_login_at,last_sync_at,revoked_at FROM cloud_devices WHERE user_id=$1 ORDER BY last_seen_at DESC"
        ).bind(user_id).fetch_all(&self.pool).await.map_err(Self::db)?;
        let devices = rows
            .into_iter()
            .map(|row| DeviceInstallationV1 {
                id: AppInstallationId::new(row.try_get::<Uuid, _>("id").unwrap().to_string()),
                external_device_id: row.try_get("external_device_id").unwrap_or_default(),
                device_group_id: row.try_get("device_group_id").ok(),
                device_name: row
                    .try_get::<Option<String>, _>("device_name")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "Unknown device".to_owned()),
                app_id: AppId::new(row.try_get::<String, _>("app_id").unwrap_or_default()),
                platform: row.try_get("platform").unwrap_or_default(),
                status: row.try_get("status").unwrap_or_default(),
                client_version: row.try_get("client_version").ok(),
                first_seen_at: row.try_get("first_seen_at").unwrap(),
                last_seen_at: row.try_get("last_seen_at").unwrap(),
                last_login_at: row.try_get("last_login_at").ok(),
                last_sync_at: row.try_get("last_sync_at").ok(),
                revoked_at: row.try_get("revoked_at").ok(),
                current: row.try_get::<Uuid, _>("id").unwrap() == current,
            })
            .collect();
        Ok(DeviceListV1 { devices })
    }

    pub async fn update_device(
        &self,
        principal: &AuthenticatedPrincipal,
        device: &str,
        request: UpdateDeviceRequestV1,
    ) -> Result<DeviceInstallationV1, ApiError> {
        principal.require_scope("devices:write")?;
        if request.device_name.trim().is_empty() || request.device_name.chars().count() > 100 {
            return Err(Self::error(
                ErrorCode::InvalidRequest,
                "invalid device name",
                StatusCode::BAD_REQUEST,
            ));
        }
        let device_id = Self::uuid(device, ErrorCode::InvalidRequest)?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let updated = sqlx::query(
            "UPDATE cloud_devices SET device_name=$3 WHERE id=$1 AND user_id=$2 RETURNING id",
        )
        .bind(device_id)
        .bind(user_id)
        .bind(request.device_name.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db)?;
        if updated.is_none() {
            return Err(Self::error(
                ErrorCode::InvalidRequest,
                "device not found",
                StatusCode::NOT_FOUND,
            ));
        }
        let list = self.list_devices(principal).await?;
        list.devices
            .into_iter()
            .find(|value| value.id.as_str() == device)
            .ok_or_else(|| {
                Self::error(
                    ErrorCode::InternalError,
                    "device update failed",
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })
    }

    pub async fn revoke_device(
        &self,
        principal: &AuthenticatedPrincipal,
        device: &str,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        principal.require_scope("devices:write")?;
        let device_id = Self::uuid(device, ErrorCode::InvalidRequest)?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        let changed = sqlx::query("UPDATE cloud_devices SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason='user_revoke' WHERE id=$1 AND user_id=$2").bind(device_id).bind(user_id).execute(&mut *tx).await.map_err(Self::db)?;
        if changed.rows_affected() == 0 {
            return Err(Self::error(
                ErrorCode::InvalidRequest,
                "device not found",
                StatusCode::NOT_FOUND,
            ));
        }
        self.revoke_device_tokens(&mut tx, device_id, "device_revoke")
            .await?;
        self.audit(
            &mut *tx,
            Some(user_id),
            None,
            Some(device_id),
            None,
            "device.revoke",
            "success",
            context,
            json!({}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    async fn revoke_device_tokens(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        device_id: Uuid,
        reason: &str,
    ) -> Result<(), ApiError> {
        sqlx::query("UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE device_id=$1").bind(device_id).bind(reason).execute(&mut **tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_access_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE session_id IN (SELECT id FROM auth_sessions WHERE device_id=$1)").bind(device_id).bind(reason).execute(&mut **tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE session_id IN (SELECT id FROM auth_sessions WHERE device_id=$1)").bind(device_id).bind(reason).execute(&mut **tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_web_sessions SET revoked_at=COALESCE(revoked_at,now()) WHERE session_id IN (SELECT id FROM auth_sessions WHERE device_id=$1)").bind(device_id).execute(&mut **tx).await.map_err(Self::db)?;
        Ok(())
    }

    pub async fn revoke_device_group(
        &self,
        principal: &AuthenticatedPrincipal,
        group: &str,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        principal.require_scope("devices:write")?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        let rows = sqlx::query("UPDATE cloud_devices SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason='group_revoke' WHERE user_id=$1 AND device_group_id=$2 RETURNING id").bind(user_id).bind(group).fetch_all(&mut *tx).await.map_err(Self::db)?;
        for row in rows {
            self.revoke_device_tokens(&mut tx, row.try_get("id").unwrap(), "group_revoke")
                .await?;
        }
        self.audit(
            &mut *tx,
            Some(user_id),
            None,
            None,
            None,
            "device_group.revoke",
            "success",
            context,
            json!({"deviceGroupId":group}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    pub async fn list_grants(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<AppGrantListV1, ApiError> {
        principal.require_scope("account:read")?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let rows = sqlx::query("SELECT id,app_id,scopes,status,granted_at,updated_at,revoked_at FROM auth_app_grants WHERE user_id=$1 ORDER BY app_id").bind(user_id).fetch_all(&self.pool).await.map_err(Self::db)?;
        Ok(AppGrantListV1 {
            grants: rows
                .into_iter()
                .map(Self::grant_from_row)
                .collect::<Result<_, _>>()?,
        })
    }

    pub async fn update_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        app_id: &str,
        request: UpdateAppGrantRequestV1,
        context: &RequestContext,
    ) -> Result<AppGrantV1, ApiError> {
        principal.require_scope("account:write")?;
        if !scope::supported_app(app_id) {
            return Err(Self::error(
                ErrorCode::AppIdUnsupported,
                "unsupported application",
                StatusCode::BAD_REQUEST,
            ));
        }
        let allowed = scope::allowed_scopes(app_id);
        let values: Vec<String> = request
            .scopes
            .into_iter()
            .map(|value| value.0)
            .filter(|value| allowed.contains(value))
            .collect();
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let row = sqlx::query("UPDATE auth_app_grants SET scopes=$3,status='active',revoked_at=NULL,updated_at=now() WHERE user_id=$1 AND app_id=$2 RETURNING id,app_id,scopes,status,granted_at,updated_at,revoked_at")
            .bind(user_id).bind(app_id).bind(&values).fetch_optional(&self.pool).await.map_err(Self::db)?
            .ok_or_else(|| Self::error(ErrorCode::InvalidRequest, "application grant not found", StatusCode::NOT_FOUND))?;
        sqlx::query("UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason='scope_change' WHERE user_id=$1 AND app_id=$2 AND status='active'").bind(user_id).bind(app_id).execute(&self.pool).await.map_err(Self::db)?;
        let _ = self
            .audit(
                &self.pool,
                Some(user_id),
                None,
                None,
                Some(app_id),
                "app_grant.update",
                "success",
                context,
                json!({"scopes":values}),
            )
            .await;
        Self::grant_from_row(row)
    }

    pub async fn revoke_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        app_id: &str,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        principal.require_scope("account:write")?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_app_grants SET status='revoked',revoked_at=COALESCE(revoked_at,now()),updated_at=now() WHERE user_id=$1 AND app_id=$2").bind(user_id).bind(app_id).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason='app_grant_revoke' WHERE user_id=$1 AND app_id=$2").bind(user_id).bind(app_id).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_access_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason='app_grant_revoke' WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1 AND app_id=$2)").bind(user_id).bind(app_id).execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason='app_grant_revoke' WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1 AND app_id=$2)").bind(user_id).bind(app_id).execute(&mut *tx).await.map_err(Self::db)?;
        self.audit(
            &mut *tx,
            Some(user_id),
            None,
            None,
            Some(app_id),
            "app_grant.revoke",
            "success",
            context,
            json!({}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    pub async fn change_password(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ChangePasswordRequestV1,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        principal.require_scope("account:write")?;
        let user_id = Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid)?;
        let encoded: String =
            sqlx::query_scalar("SELECT password_hash FROM cloud_users WHERE id=$1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(Self::db)?;
        if !self.passwords.verify(&request.current_password, &encoded) {
            return Err(Self::error(
                ErrorCode::AuthPasswordInvalid,
                "current password is invalid",
                StatusCode::UNAUTHORIZED,
            ));
        }
        let hash = self.passwords.hash(&request.new_password)?;
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        sqlx::query("UPDATE cloud_users SET password_hash=$2,password_version=password_version+1,password_changed_at=now(),updated_at=now() WHERE id=$1").bind(user_id).bind(hash).execute(&mut *tx).await.map_err(Self::db)?;
        self.revoke_all_user_sessions(&mut tx, user_id, "password_change")
            .await?;
        self.audit(
            &mut *tx,
            Some(user_id),
            None,
            None,
            None,
            "password.change",
            "success",
            context,
            json!({}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    pub async fn forgot_password(
        &self,
        request: ForgotPasswordRequestV1,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        let normalized = Self::normalize_email(&request.email);
        self.check_rate_limit(&normalized, context).await?;
        if let Some(user_id) = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM cloud_users WHERE email_normalized=$1 AND status='active'",
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db)?
        {
            let token = self.tokens.generate(TokenKind::PasswordReset);
            let mut tx = self.pool.begin().await.map_err(Self::db)?;
            sqlx::query("UPDATE auth_password_reset_tokens SET revoked_at=COALESCE(revoked_at,now()) WHERE user_id=$1 AND used_at IS NULL AND revoked_at IS NULL").bind(user_id).execute(&mut *tx).await.map_err(Self::db)?;
            sqlx::query("INSERT INTO auth_password_reset_tokens (id,user_id,token_hash,expires_at,requested_ip) VALUES ($1,$2,$3,$4,CAST($5 AS inet))")
                .bind(token.id).bind(user_id).bind(&token.hash).bind(Utc::now()+Duration::seconds(self.config.auth_reset_token_ttl_seconds as i64)).bind(Self::ip(context))
                .execute(&mut *tx).await.map_err(Self::db)?;
            self.audit(
                &mut *tx,
                Some(user_id),
                None,
                None,
                None,
                "password.forgot",
                "accepted",
                context,
                json!({}),
            )
            .await?;
            tx.commit().await.map_err(Self::db)?;
            if !self.config.is_production() && self.config.auth_reset_notifier == "console" {
                eprintln!(
                    "[lifetrace-cloud] development password reset token: {}",
                    token.raw
                );
                if let Ok(mut captured) = self.development_reset_token.lock() {
                    *captured = Some(token.raw);
                }
            }
        }
        Ok(AcceptedResponseV1 { accepted: true })
    }

    pub fn development_reset_token(&self) -> Option<String> {
        self.development_reset_token
            .lock()
            .ok()
            .and_then(|value| value.clone())
    }

    pub async fn reset_password(
        &self,
        request: ResetPasswordRequestV1,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        let parsed = self
            .tokens
            .parse(TokenKind::PasswordReset, &request.token)
            .ok_or_else(|| {
                Self::error(
                    ErrorCode::AuthPasswordResetInvalid,
                    "invalid password reset token",
                    StatusCode::BAD_REQUEST,
                )
            })?;
        let hash = self.passwords.hash(&request.new_password)?;
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        let row = sqlx::query("SELECT user_id,token_hash,expires_at,used_at,revoked_at FROM auth_password_reset_tokens WHERE id=$1 FOR UPDATE")
            .bind(parsed.id).fetch_optional(&mut *tx).await.map_err(Self::db)?
            .ok_or_else(|| Self::error(ErrorCode::AuthPasswordResetInvalid, "invalid password reset token", StatusCode::BAD_REQUEST))?;
        let expected: Vec<u8> = row.try_get("token_hash").unwrap_or_default();
        if !self
            .tokens
            .verify(TokenKind::PasswordReset, &parsed, &expected)
            || row
                .try_get::<Option<DateTime<Utc>>, _>("used_at")
                .ok()
                .flatten()
                .is_some()
            || row
                .try_get::<Option<DateTime<Utc>>, _>("revoked_at")
                .ok()
                .flatten()
                .is_some()
        {
            return Err(Self::error(
                ErrorCode::AuthPasswordResetInvalid,
                "invalid password reset token",
                StatusCode::BAD_REQUEST,
            ));
        }
        if row
            .try_get::<DateTime<Utc>, _>("expires_at")
            .unwrap_or_else(|_| Utc::now())
            <= Utc::now()
        {
            return Err(Self::error(
                ErrorCode::AuthPasswordResetExpired,
                "password reset token expired",
                StatusCode::BAD_REQUEST,
            ));
        }
        let user_id: Uuid = row.try_get("user_id").unwrap();
        sqlx::query("UPDATE auth_password_reset_tokens SET used_at=now() WHERE id=$1")
            .bind(parsed.id)
            .execute(&mut *tx)
            .await
            .map_err(Self::db)?;
        sqlx::query("UPDATE cloud_users SET password_hash=$2,password_version=password_version+1,password_changed_at=now(),auth_state='active',updated_at=now() WHERE id=$1").bind(user_id).bind(hash).execute(&mut *tx).await.map_err(Self::db)?;
        self.revoke_all_user_sessions(&mut tx, user_id, "password_reset")
            .await?;
        self.audit(
            &mut *tx,
            Some(user_id),
            None,
            None,
            None,
            "password.reset",
            "success",
            context,
            json!({}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok(AcceptedResponseV1 { accepted: true })
    }

    async fn revoke_all_user_sessions(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        reason: &str,
    ) -> Result<(), ApiError> {
        sqlx::query("UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE user_id=$1").bind(user_id).bind(reason).execute(&mut **tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_access_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1)").bind(user_id).bind(reason).execute(&mut **tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()),revoked_reason=$2 WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1)").bind(user_id).bind(reason).execute(&mut **tx).await.map_err(Self::db)?;
        sqlx::query("UPDATE auth_web_sessions SET revoked_at=COALESCE(revoked_at,now()) WHERE session_id IN (SELECT id FROM auth_sessions WHERE user_id=$1)").bind(user_id).execute(&mut **tx).await.map_err(Self::db)?;
        Ok(())
    }

    pub async fn web_login(
        &self,
        request: WebLoginRequestV1,
        context: &RequestContext,
    ) -> Result<(WebSessionResponseV1, String), ApiError> {
        let user_agent = context
            .user_agent
            .clone()
            .unwrap_or_else(|| "browser".to_owned());
        let external_device_id = format!("web-{:x}", Sha256::digest(user_agent.as_bytes()));
        let input = LoginInput {
            email: request.email,
            password: request.password,
            app_id: AppId::WEB.to_owned(),
            external_device_id,
            device_name: "LifeTrace Web Browser".to_owned(),
            platform: "web".to_owned(),
            client_version: None,
            requested_scopes: request.requested_scopes,
            public_device: request.public_device,
        };
        let verified = self.verify_and_prepare_login(&input, context).await?;
        let now = Utc::now();
        let absolute_seconds = if input.public_device {
            self.config.auth_public_device_ttl_seconds
        } else {
            self.config.auth_web_absolute_ttl_seconds
        };
        let idle_seconds = if input.public_device {
            absolute_seconds
        } else {
            self.config.auth_web_idle_ttl_seconds
        };
        let session_id = Uuid::new_v4();
        let web = self.tokens.generate(TokenKind::WebSession);
        let csrf = self.tokens.generate(TokenKind::Csrf);
        let mut tx = self.pool.begin().await.map_err(Self::db)?;
        sqlx::query("INSERT INTO auth_sessions (id,user_id,device_id,app_id,scopes,session_type,status,idle_expires_at,absolute_expires_at,login_ip,last_ip,user_agent,public_device) VALUES ($1,$2,$3,$4,$5,'web','active',$6,$7,CAST($8 AS inet),CAST($8 AS inet),$9,$10)")
            .bind(session_id).bind(verified.user_id).bind(verified.device_id).bind(&verified.app_id).bind(&verified.scopes)
            .bind(now+Duration::seconds(idle_seconds as i64)).bind(now+Duration::seconds(absolute_seconds as i64)).bind(Self::ip(context)).bind(&context.user_agent).bind(input.public_device)
            .execute(&mut *tx).await.map_err(Self::db)?;
        sqlx::query("INSERT INTO auth_web_sessions (id,session_id,token_hash,csrf_hash,expires_at) VALUES ($1,$2,$3,$4,$5)")
            .bind(web.id).bind(session_id).bind(&web.hash).bind(&csrf.hash).bind(now+Duration::seconds(absolute_seconds as i64))
            .execute(&mut *tx).await.map_err(Self::db)?;
        self.audit(
            &mut *tx,
            Some(verified.user_id),
            Some(session_id),
            Some(verified.device_id),
            Some(AppId::WEB),
            "auth.web_login",
            "success",
            context,
            json!({"publicDevice":input.public_device}),
        )
        .await?;
        tx.commit().await.map_err(Self::db)?;
        Ok((
            WebSessionResponseV1 {
                user: verified.user,
                session: self.session_by_id(session_id, Some(session_id)).await?,
                csrf_token: csrf.raw,
            },
            web.raw,
        ))
    }

    pub async fn web_session(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<WebSessionResponseV1, ApiError> {
        let session_id = Self::uuid(principal.session_id.as_str(), ErrorCode::AuthInvalid)?;
        let csrf = self.tokens.generate(TokenKind::Csrf);
        sqlx::query(
            "UPDATE auth_web_sessions SET csrf_hash=$2 WHERE session_id=$1 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .bind(&csrf.hash)
        .execute(&self.pool)
        .await
        .map_err(Self::db)?;
        Ok(WebSessionResponseV1 {
            user: self.me(principal).await?,
            session: self.session_by_id(session_id, Some(session_id)).await?,
            csrf_token: csrf.raw,
        })
    }

    pub async fn verify_web_csrf(
        &self,
        raw_session: &str,
        raw_csrf: &str,
        origin: Option<&str>,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        if !origin_allowed(origin, &self.config) {
            return Err(Self::error(
                ErrorCode::AuthCsrfInvalid,
                "invalid request origin",
                StatusCode::FORBIDDEN,
            ));
        }
        let parsed_session = self
            .tokens
            .parse(TokenKind::WebSession, raw_session)
            .ok_or_else(|| {
                Self::error(
                    ErrorCode::AuthRequired,
                    "web session required",
                    StatusCode::UNAUTHORIZED,
                )
            })?;
        let parsed_csrf = self
            .tokens
            .parse(TokenKind::Csrf, raw_csrf)
            .ok_or_else(|| {
                Self::error(
                    ErrorCode::AuthCsrfInvalid,
                    "invalid CSRF token",
                    StatusCode::FORBIDDEN,
                )
            })?;
        let row = sqlx::query(
            "SELECT token_hash,csrf_hash FROM auth_web_sessions WHERE id=$1 AND revoked_at IS NULL",
        )
        .bind(parsed_session.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db)?
        .ok_or_else(|| {
            Self::error(
                ErrorCode::AuthRequired,
                "web session required",
                StatusCode::UNAUTHORIZED,
            )
        })?;
        let token_hash: Vec<u8> = row.try_get("token_hash").unwrap_or_default();
        let csrf_hash: Vec<u8> = row.try_get("csrf_hash").unwrap_or_default();
        let actual_csrf = self
            .tokens
            .hash(TokenKind::Csrf, parsed_csrf.id, parsed_csrf.secret);
        if !self
            .tokens
            .verify(TokenKind::WebSession, &parsed_session, &token_hash)
            || !csrf_matches(&csrf_hash, &actual_csrf)
        {
            return Err(Self::error(
                ErrorCode::AuthCsrfInvalid,
                "invalid CSRF token",
                StatusCode::FORBIDDEN,
            ));
        }
        use crate::auth::{AuthCredential, AuthProvider};
        let provider =
            crate::auth::DatabaseAuthProvider::new(self.pool.clone(), self.tokens.clone());
        provider
            .authenticate(AuthCredential::WebSession(Some(raw_session)))
            .await
    }

    pub async fn rotate_web_session(
        &self,
        principal: &AuthenticatedPrincipal,
        old_raw: &str,
        context: &RequestContext,
    ) -> Result<(WebSessionResponseV1, String), ApiError> {
        let old = self
            .tokens
            .parse(TokenKind::WebSession, old_raw)
            .ok_or_else(|| {
                Self::error(
                    ErrorCode::AuthInvalid,
                    "invalid web session",
                    StatusCode::UNAUTHORIZED,
                )
            })?;
        let session_id = Self::uuid(principal.session_id.as_str(), ErrorCode::AuthInvalid)?;
        let web = self.tokens.generate(TokenKind::WebSession);
        let csrf = self.tokens.generate(TokenKind::Csrf);
        let expires: DateTime<Utc> =
            sqlx::query_scalar("SELECT absolute_expires_at FROM auth_sessions WHERE id=$1")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await
                .map_err(Self::db)?;
        sqlx::query("UPDATE auth_web_sessions SET id=$2,token_hash=$3,csrf_hash=$4,rotated_at=now(),expires_at=$5 WHERE id=$1 AND session_id=$6")
            .bind(old.id).bind(web.id).bind(&web.hash).bind(&csrf.hash).bind(expires).bind(session_id).execute(&self.pool).await.map_err(Self::db)?;
        let _ = self
            .audit(
                &self.pool,
                Self::uuid(principal.user_id.as_str(), ErrorCode::AuthInvalid).ok(),
                Some(session_id),
                Self::uuid(principal.device_id.as_str(), ErrorCode::AuthInvalid).ok(),
                Some(principal.app_id.as_str()),
                "auth.web_rotate",
                "success",
                context,
                json!({}),
            )
            .await;
        Ok((
            WebSessionResponseV1 {
                user: self.me(principal).await?,
                session: self.session_by_id(session_id, Some(session_id)).await?,
                csrf_token: csrf.raw,
            },
            web.raw,
        ))
    }

    pub async fn web_logout(
        &self,
        principal: &AuthenticatedPrincipal,
        context: &RequestContext,
    ) -> Result<AcceptedResponseV1, ApiError> {
        self.logout(principal, context).await
    }

    pub async fn bootstrap_user(
        &self,
        email: &str,
        display_name: Option<&str>,
        password: &str,
        allow_additional: bool,
    ) -> Result<UserId, ApiError> {
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM cloud_users WHERE email_normalized IS NOT NULL AND status='active'").fetch_one(&self.pool).await.map_err(Self::db)?;
        if active > 0 && !allow_additional {
            return Err(Self::error(
                ErrorCode::InvalidRequest,
                "an active account already exists; pass --allow-additional explicitly",
                StatusCode::CONFLICT,
            ));
        }
        let normalized = Self::normalize_email(email);
        let hash = self.passwords.hash(password)?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO cloud_users (id,status,email,email_normalized,display_name,password_hash,password_version,password_changed_at,registration_source,auth_state,email_verified_at) VALUES ($1,'active',$2,$3,$4,$5,1,now(),'bootstrap','active',now())")
            .bind(id).bind(email.trim()).bind(normalized).bind(display_name).bind(hash).execute(&self.pool).await.map_err(Self::db)?;
        Ok(UserId::new(id.to_string()))
    }

    pub async fn create_invite(
        &self,
        email: Option<&str>,
        expires_in_seconds: u64,
        created_by: Option<Uuid>,
    ) -> Result<String, ApiError> {
        let token = self.tokens.generate(TokenKind::Invite);
        let normalized = email.map(Self::normalize_email);
        sqlx::query("INSERT INTO auth_registration_invites (id,token_hash,email_normalized,expires_at,created_by) VALUES ($1,$2,$3,$4,$5)")
            .bind(token.id).bind(&token.hash).bind(normalized).bind(Utc::now()+Duration::seconds(expires_in_seconds as i64)).bind(created_by)
            .execute(&self.pool).await.map_err(Self::db)?;
        Ok(token.raw)
    }

    async fn user_by_id(&self, id: Uuid) -> Result<AuthUserV1, ApiError> {
        let row = sqlx::query("SELECT id,email,display_name,status,auth_state,email_verified_at,created_at,password_changed_at FROM cloud_users WHERE id=$1")
            .bind(id).fetch_optional(&self.pool).await.map_err(Self::db)?
            .ok_or_else(|| Self::error(ErrorCode::AuthInvalid, "account not found", StatusCode::UNAUTHORIZED))?;
        Self::user_from_row(&row)
    }

    fn user_from_row(row: &sqlx::postgres::PgRow) -> Result<AuthUserV1, ApiError> {
        let id: Uuid = row.try_get("id").map_err(|_| {
            Self::error(
                ErrorCode::InternalError,
                "invalid user record",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        Ok(AuthUserV1 {
            id: UserId::new(id.to_string()),
            email: row
                .try_get::<Option<String>, _>("email")
                .ok()
                .flatten()
                .unwrap_or_default(),
            display_name: row.try_get("display_name").ok(),
            state: row
                .try_get::<String, _>("auth_state")
                .unwrap_or_else(|_| row.try_get::<String, _>("status").unwrap_or_default()),
            email_verified_at: row.try_get("email_verified_at").ok(),
            created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            password_changed_at: row.try_get("password_changed_at").ok(),
        })
    }

    async fn session_by_id(
        &self,
        id: Uuid,
        current: Option<Uuid>,
    ) -> Result<AuthSessionV1, ApiError> {
        let row = sqlx::query("SELECT id,app_id,device_id,session_type,status,scopes,public_device,created_at,last_seen_at,idle_expires_at,absolute_expires_at,revoked_at FROM auth_sessions WHERE id=$1")
            .bind(id).fetch_optional(&self.pool).await.map_err(Self::db)?
            .ok_or_else(|| Self::error(ErrorCode::AuthSessionRevoked, "session not found", StatusCode::UNAUTHORIZED))?;
        Ok(AuthSessionV1 {
            id: AuthSessionId::new(id.to_string()),
            app_id: AppId::new(row.try_get::<String, _>("app_id").unwrap_or_default()),
            device_id: AppInstallationId::new(
                row.try_get::<Uuid, _>("device_id").unwrap().to_string(),
            ),
            session_type: row.try_get("session_type").unwrap_or_default(),
            status: row.try_get("status").unwrap_or_default(),
            scopes: row
                .try_get::<Vec<String>, _>("scopes")
                .unwrap_or_default()
                .into_iter()
                .map(Scope::new)
                .collect(),
            public_device: row.try_get("public_device").unwrap_or(false),
            created_at: row.try_get("created_at").unwrap(),
            last_seen_at: row.try_get("last_seen_at").unwrap(),
            idle_expires_at: row.try_get("idle_expires_at").unwrap(),
            absolute_expires_at: row.try_get("absolute_expires_at").unwrap(),
            revoked_at: row.try_get("revoked_at").ok(),
            current: current == Some(id),
        })
    }

    fn grant_from_row(row: sqlx::postgres::PgRow) -> Result<AppGrantV1, ApiError> {
        Ok(AppGrantV1 {
            id: AppGrantId::new(
                row.try_get::<Uuid, _>("id")
                    .map_err(|_| {
                        Self::error(
                            ErrorCode::InternalError,
                            "invalid grant",
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })?
                    .to_string(),
            ),
            app_id: AppId::new(row.try_get::<String, _>("app_id").unwrap_or_default()),
            scopes: row
                .try_get::<Vec<String>, _>("scopes")
                .unwrap_or_default()
                .into_iter()
                .map(Scope::new)
                .collect(),
            status: row.try_get("status").unwrap_or_default(),
            granted_at: row.try_get("granted_at").unwrap(),
            updated_at: row.try_get("updated_at").unwrap(),
            revoked_at: row.try_get("revoked_at").ok(),
        })
    }
}
