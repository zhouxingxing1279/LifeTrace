//! BeeCount-compatible profile, device and shared-ledger account surface.

use std::collections::BTreeMap;

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::ErrorCode;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::beecount_collaboration::{
    beecount_user_id, conflict, db_error, ensure_owner_registry, internal, internal_user_id,
    invalid, member_user_ids, parse_user_id, resolve_ledger_access, MEMBER_LIMIT, ROLE_EDITOR,
    ROLE_OWNER,
};
use crate::error::ApiError;
use crate::state::AppState;

const PREFIX: &str = "/api/v1/integrations/beecount/compat";
const AVATAR_MAX_BYTES: usize = 1024 * 1024;
const AVATAR_MULTIPART_OVERHEAD: usize = 256 * 1024;
const INVITE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const MAX_ACTIVE_INVITES: i64 = 10;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route(
            &format!("{PREFIX}/profile/me"),
            get(get_profile).patch(patch_profile),
        )
        .route(&format!("{PREFIX}/profile/avatar"), post(upload_avatar))
        .route(
            &format!("{PREFIX}/profile/avatar/{{user_id}}"),
            get(download_avatar),
        )
        .route(&format!("{PREFIX}/devices"), get(list_devices))
        .route(
            &format!("{PREFIX}/devices/{{device_id}}/revoke"),
            post(revoke_device),
        )
        .route(
            &format!("{PREFIX}/ledgers/{{ledger_id}}/invites"),
            post(create_invite).get(list_invites),
        )
        .route(
            &format!("{PREFIX}/ledgers/{{ledger_id}}/invites/{{code}}"),
            delete(revoke_invite),
        )
        .route(
            &format!("{PREFIX}/invites/{{code}}/preview"),
            post(preview_invite),
        )
        .route(
            &format!("{PREFIX}/invites/{{code}}/accept"),
            post(accept_invite),
        )
        .route(
            &format!("{PREFIX}/ledgers/{{ledger_id}}/members"),
            get(list_members),
        )
        .route(
            &format!("{PREFIX}/ledgers/{{ledger_id}}/members/{{user_id}}"),
            patch(update_member_role).delete(remove_member),
        )
        .route(
            &format!("{PREFIX}/ledgers/{{ledger_id}}/transfer"),
            post(transfer_ownership),
        )
        .route(
            &format!("{PREFIX}/ledgers/{{ledger_id}}/shared-resources"),
            get(shared_resources),
        )
        .layer(DefaultBodyLimit::max(
            AVATAR_MAX_BYTES + AVATAR_MULTIPART_OVERHEAD,
        ))
}

#[derive(Debug, Clone, Serialize)]
struct ProfileOut {
    user_id: String,
    email: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    avatar_version: i64,
    income_is_red: Option<bool>,
    theme_primary_color: Option<String>,
    appearance: Option<Value>,
    ai_config: Option<Value>,
    primary_currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfilePatch {
    display_name: Option<String>,
    income_is_red: Option<bool>,
    theme_primary_color: Option<String>,
    appearance: Option<Value>,
    ai_config: Option<Value>,
    primary_currency: Option<String>,
}

#[derive(Debug, Serialize)]
struct AvatarUploadOut {
    avatar_url: String,
    avatar_version: i64,
}

async fn get_profile(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<ProfileOut>, ApiError> {
    authorize(&principal, "account:read")?;
    profile_out(&state, parse_user_id(principal.user_id.as_str())?)
        .await
        .map(Json)
}

async fn patch_profile(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<ProfilePatch>,
) -> Result<Json<ProfileOut>, ApiError> {
    authorize(&principal, "account:read")?;
    validate_profile_patch(&request)?;
    let user_id = parse_user_id(principal.user_id.as_str())?;
    let mut tx = state.pool.begin().await.map_err(db_error)?;
    let existing = sqlx::query(
        "SELECT income_is_red,theme_primary_color,appearance,ai_config,primary_currency \
         FROM beecount_user_profiles WHERE user_id=$1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error)?;
    let old_bool = existing
        .as_ref()
        .and_then(|row| row.try_get::<Option<bool>, _>("income_is_red").ok())
        .flatten();
    let old_theme = existing
        .as_ref()
        .and_then(|row| row.try_get::<Option<String>, _>("theme_primary_color").ok())
        .flatten();
    let old_appearance = existing
        .as_ref()
        .and_then(|row| row.try_get::<Option<Value>, _>("appearance").ok())
        .flatten();
    let old_ai = existing
        .as_ref()
        .and_then(|row| row.try_get::<Option<Value>, _>("ai_config").ok())
        .flatten();
    let old_currency = existing
        .as_ref()
        .and_then(|row| row.try_get::<Option<String>, _>("primary_currency").ok())
        .flatten();
    let income_is_red = request.income_is_red.or(old_bool);
    let theme = request
        .theme_primary_color
        .as_deref()
        .map(|value| value.trim().to_ascii_uppercase())
        .or(old_theme);
    let appearance = merge_json_setting(request.appearance, old_appearance);
    let ai_config = merge_json_setting(request.ai_config, old_ai);
    let currency = request
        .primary_currency
        .as_deref()
        .map(|value| value.trim().to_ascii_uppercase())
        .or(old_currency);
    sqlx::query(
        "INSERT INTO beecount_user_profiles \
         (user_id,income_is_red,theme_primary_color,appearance,ai_config,primary_currency,updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,now()) \
         ON CONFLICT (user_id) DO UPDATE SET \
           income_is_red=EXCLUDED.income_is_red,theme_primary_color=EXCLUDED.theme_primary_color, \
           appearance=EXCLUDED.appearance,ai_config=EXCLUDED.ai_config, \
           primary_currency=EXCLUDED.primary_currency,updated_at=now()",
    )
    .bind(user_id)
    .bind(income_is_red)
    .bind(theme.as_deref())
    .bind(appearance.as_ref())
    .bind(ai_config.as_ref())
    .bind(currency.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    if let Some(display_name) = request.display_name.as_deref() {
        sqlx::query("UPDATE cloud_users SET display_name=$2,updated_at=now() WHERE id=$1")
            .bind(user_id)
            .bind(display_name.trim())
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
    }
    tx.commit().await.map_err(db_error)?;
    let profile = profile_out(&state, user_id).await?;
    state.beecount_realtime.publish(
        principal.user_id.as_str(),
        json!({
            "type": "profile_change",
            "avatar_version": profile.avatar_version,
            "display_name": profile.display_name,
            "income_is_red": profile.income_is_red,
            "theme_primary_color": profile.theme_primary_color,
            "appearance": profile.appearance,
            "primary_currency": profile.primary_currency,
        }),
    );
    Ok(Json(profile))
}

async fn upload_avatar(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    mut multipart: Multipart,
) -> Result<Json<AvatarUploadOut>, ApiError> {
    authorize(&principal, "account:read")?;
    let mut upload = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| invalid(&error.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let original_name = field.file_name().unwrap_or("avatar.bin").to_owned();
        let content_type = field.content_type().map(str::to_owned);
        let content = field
            .bytes()
            .await
            .map_err(|error| invalid(&error.to_string()))?
            .to_vec();
        upload = Some((original_name, content_type, content));
    }
    let (original_name, content_type, content) =
        upload.ok_or_else(|| invalid("file is required"))?;
    if content.is_empty() {
        return Err(invalid("Profile avatar file is empty"));
    }
    if content.len() > AVATAR_MAX_BYTES {
        return Err(ApiError::new(
            ErrorCode::PayloadTooLarge,
            "Profile avatar upload too large",
            StatusCode::PAYLOAD_TOO_LARGE,
        ));
    }
    let (mime_type, extension) = avatar_type(content_type.as_deref(), &original_name)?;
    let user_id = parse_user_id(principal.user_id.as_str())?;
    let file_name = format!("avatar_{}.{}", Uuid::new_v4().simple(), extension);
    let version: i64 = sqlx::query_scalar(
        "INSERT INTO beecount_user_profiles \
         (user_id,avatar_version,avatar_mime_type,avatar_file_name,avatar_content,updated_at) \
         VALUES ($1,1,$2,$3,$4,now()) \
         ON CONFLICT (user_id) DO UPDATE SET \
           avatar_version=beecount_user_profiles.avatar_version+1, \
           avatar_mime_type=EXCLUDED.avatar_mime_type,avatar_file_name=EXCLUDED.avatar_file_name, \
           avatar_content=EXCLUDED.avatar_content,updated_at=now() \
         RETURNING avatar_version",
    )
    .bind(user_id)
    .bind(mime_type)
    .bind(file_name)
    .bind(content)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;
    let wire_id = beecount_user_id(&state.pool, user_id).await?;
    let avatar_url = avatar_url(&wire_id, version);
    state.beecount_realtime.publish(
        principal.user_id.as_str(),
        json!({"type":"profile_change","avatar_version":version}),
    );
    Ok(Json(AvatarUploadOut {
        avatar_url,
        avatar_version: version,
    }))
}

async fn download_avatar(
    State(state): State<AppState>,
    Path(wire_user_id): Path<String>,
    Query(query): Query<BTreeMap<String, String>>,
) -> Result<Response, ApiError> {
    let user_id = internal_user_id(&state.pool, &wire_user_id).await?;
    let row = sqlx::query(
        "SELECT avatar_content,avatar_mime_type,avatar_file_name \
         FROM beecount_user_profiles WHERE user_id=$1 AND avatar_content IS NOT NULL",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| crate::beecount_collaboration::not_found("Profile avatar not found"))?;
    let content: Vec<u8> = row.try_get("avatar_content").map_err(internal)?;
    let mime_type: String = row.try_get("avatar_mime_type").map_err(internal)?;
    let file_name: String = row.try_get("avatar_file_name").map_err(internal)?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(
            header::CACHE_CONTROL,
            if query.contains_key("v") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            },
        )
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("inline; filename=\"{file_name}\""))
                .unwrap_or_else(|_| HeaderValue::from_static("inline")),
        )
        .header(header::CONTENT_LENGTH, content.len().to_string())
        .body(Body::from(content))
        .map_err(internal)
}

async fn profile_out(state: &AppState, user_id: Uuid) -> Result<ProfileOut, ApiError> {
    let row = sqlx::query(
        "SELECT u.email,u.display_name,COALESCE(l.beecount_user_id,u.id::text) AS wire_user_id, \
                p.avatar_version,p.avatar_content IS NOT NULL AS has_avatar,p.income_is_red, \
                p.theme_primary_color,p.appearance,p.ai_config,p.primary_currency \
         FROM cloud_users u LEFT JOIN beecount_identity_links l ON l.user_id=u.id \
         LEFT JOIN beecount_user_profiles p ON p.user_id=u.id WHERE u.id=$1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| crate::beecount_collaboration::not_found("User not found"))?;
    let wire_id: String = row.try_get("wire_user_id").map_err(internal)?;
    let avatar_version = row
        .try_get::<Option<i64>, _>("avatar_version")
        .map_err(internal)?
        .unwrap_or(0);
    let has_avatar = row.try_get::<bool, _>("has_avatar").unwrap_or(false);
    Ok(ProfileOut {
        user_id: wire_id.clone(),
        email: row.try_get("email").map_err(internal)?,
        display_name: row.try_get("display_name").map_err(internal)?,
        avatar_url: has_avatar.then(|| avatar_url(&wire_id, avatar_version)),
        avatar_version,
        income_is_red: row.try_get("income_is_red").map_err(internal)?,
        theme_primary_color: row.try_get("theme_primary_color").map_err(internal)?,
        appearance: row.try_get("appearance").map_err(internal)?,
        ai_config: row.try_get("ai_config").map_err(internal)?,
        primary_currency: row.try_get("primary_currency").map_err(internal)?,
    })
}

fn validate_profile_patch(request: &ProfilePatch) -> Result<(), ApiError> {
    if let Some(value) = request.display_name.as_deref() {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 32 {
            return Err(invalid("invalid display_name"));
        }
    }
    if let Some(value) = request.theme_primary_color.as_deref() {
        let bytes = value.trim().as_bytes();
        if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
            return Err(invalid("theme_primary_color must be #RRGGBB hex"));
        }
    }
    for value in [&request.appearance, &request.ai_config]
        .into_iter()
        .flatten()
    {
        if !value.is_object() {
            return Err(invalid("profile JSON settings must be objects"));
        }
    }
    if let Some(value) = request.primary_currency.as_deref() {
        let trimmed = value.trim();
        if !(3..=8).contains(&trimmed.len())
            || !trimmed.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            return Err(invalid("invalid primary_currency"));
        }
    }
    Ok(())
}

fn merge_json_setting(incoming: Option<Value>, existing: Option<Value>) -> Option<Value> {
    match incoming {
        Some(Value::Object(map)) if map.is_empty() => None,
        Some(value) => Some(value),
        None => existing,
    }
}

fn avatar_type(
    content_type: Option<&str>,
    file_name: &str,
) -> Result<(&'static str, &'static str), ApiError> {
    let mime = content_type.unwrap_or_default().trim().to_ascii_lowercase();
    let name = file_name.trim().to_ascii_lowercase();
    match mime.as_str() {
        "image/jpeg" => Ok(("image/jpeg", "jpg")),
        "image/png" => Ok(("image/png", "png")),
        "image/webp" => Ok(("image/webp", "webp")),
        _ if name.ends_with(".jpg") || name.ends_with(".jpeg") => Ok(("image/jpeg", "jpg")),
        _ if name.ends_with(".png") => Ok(("image/png", "png")),
        _ if name.ends_with(".webp") => Ok(("image/webp", "webp")),
        _ => Err(invalid("Profile avatar format invalid")),
    }
}

fn avatar_url(wire_user_id: &str, version: i64) -> String {
    format!("/api/v1/profile/avatar/{wire_user_id}?v={version}")
}

#[derive(Debug, Deserialize)]
struct DeviceQuery {
    #[serde(default = "default_device_view")]
    view: String,
    #[serde(default = "default_active_days")]
    active_within_days: i64,
}

fn default_device_view() -> String {
    "deduped".to_owned()
}

fn default_active_days() -> i64 {
    30
}

#[derive(Debug, Clone, Serialize)]
struct DeviceOut {
    id: String,
    name: String,
    platform: String,
    app_version: Option<String>,
    os_version: Option<String>,
    device_model: Option<String>,
    last_ip: Option<String>,
    last_seen_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    session_count: i64,
}

async fn list_devices(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<DeviceQuery>,
) -> Result<Json<Vec<DeviceOut>>, ApiError> {
    authorize(&principal, "devices:read")?;
    if !matches!(query.view.as_str(), "deduped" | "sessions") || query.active_within_days < 0 {
        return Err(invalid("invalid device list query"));
    }
    let user_id = parse_user_id(principal.user_id.as_str())?;
    let cutoff = (query.active_within_days > 0)
        .then(|| Utc::now() - Duration::days(query.active_within_days));
    let rows = sqlx::query(
        "SELECT external_device_id,device_name,platform,client_version,os_version,device_model, \
                host(last_login_ip) AS last_ip,last_seen_at,first_seen_at \
         FROM cloud_devices WHERE user_id=$1 AND app_id='beecount-mobile' \
           AND status='active' AND revoked_at IS NULL \
           AND ($2::timestamptz IS NULL OR last_seen_at >= $2) ORDER BY last_seen_at DESC",
    )
    .bind(user_id)
    .bind(cutoff)
    .fetch_all(&state.pool)
    .await
    .map_err(db_error)?;
    let devices = rows
        .into_iter()
        .map(|row| {
            let id: String = row.try_get("external_device_id").map_err(internal)?;
            Ok(DeviceOut {
                name: row
                    .try_get::<Option<String>, _>("device_name")
                    .map_err(internal)?
                    .unwrap_or_else(|| id.clone()),
                id,
                platform: row.try_get("platform").map_err(internal)?,
                app_version: row.try_get("client_version").map_err(internal)?,
                os_version: row.try_get("os_version").map_err(internal)?,
                device_model: row.try_get("device_model").map_err(internal)?,
                last_ip: row.try_get("last_ip").map_err(internal)?,
                last_seen_at: row.try_get("last_seen_at").map_err(internal)?,
                created_at: row.try_get("first_seen_at").map_err(internal)?,
                session_count: 1,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(Json(devices))
}

async fn revoke_device(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(external_device_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    authorize(&principal, "devices:write")?;
    let user_id = parse_user_id(principal.user_id.as_str())?;
    let mut tx = state.pool.begin().await.map_err(db_error)?;
    let device_id: Uuid = sqlx::query_scalar(
        "UPDATE cloud_devices SET status='revoked',revoked_at=COALESCE(revoked_at,now()), \
         revoked_reason='user_revoke' WHERE user_id=$1 AND app_id='beecount-mobile' \
         AND external_device_id=$2 RETURNING id",
    )
    .bind(user_id)
    .bind(&external_device_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error)?
    .ok_or_else(|| crate::beecount_collaboration::not_found("Device not found"))?;
    revoke_device_tokens(&mut tx, device_id).await?;
    tx.commit().await.map_err(db_error)?;
    Ok(Json(json!({"ok":true,"device_id":external_device_id})))
}

async fn revoke_device_tokens(
    tx: &mut Transaction<'_, Postgres>,
    device_id: Uuid,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE auth_sessions SET status='revoked',revoked_at=COALESCE(revoked_at,now()), \
         revoked_reason='device_revoke' WHERE device_id=$1",
    )
    .bind(device_id)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    for table in ["auth_access_tokens", "auth_refresh_tokens"] {
        let query = format!(
            "UPDATE {table} SET revoked_at=COALESCE(revoked_at,now()),revoked_reason='device_revoke' \
             WHERE session_id IN (SELECT id FROM auth_sessions WHERE device_id=$1)"
        );
        sqlx::query(&query)
            .bind(device_id)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
    }
    sqlx::query(
        "UPDATE auth_web_sessions SET revoked_at=COALESCE(revoked_at,now()) \
         WHERE session_id IN (SELECT id FROM auth_sessions WHERE device_id=$1)",
    )
    .bind(device_id)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

fn authorize(principal: &AuthenticatedPrincipal, scope: &str) -> Result<(), ApiError> {
    if principal.app_id.as_str() != AppId::BEECOUNT {
        return Err(ApiError::new(
            ErrorCode::AuthInvalid,
            "BeeCount session required",
            StatusCode::UNAUTHORIZED,
        ));
    }
    principal.require_scope(scope)
}

#[derive(Debug, Deserialize)]
struct InviteCreateRequest {
    #[serde(default = "default_editor_role")]
    role: String,
    #[serde(default = "default_invite_hours")]
    expires_in_hours: i64,
}

fn default_editor_role() -> String {
    ROLE_EDITOR.to_owned()
}

fn default_invite_hours() -> i64 {
    24
}

#[derive(Debug, Clone, Serialize)]
struct InviteCreateResponse {
    code: String,
    formatted_code: String,
    target_role: String,
    expires_at: DateTime<Utc>,
    share_url: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct InviteListItem {
    code: String,
    formatted_code: String,
    target_role: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    invited_by_user_id: String,
    share_url: String,
}

#[derive(Debug, Serialize)]
struct InvitePreviewResponse {
    code: String,
    ledger_external_id: String,
    ledger_name: Option<String>,
    ledger_currency: String,
    invited_by_display: String,
    target_role: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InviteAcceptResponse {
    ledger_external_id: String,
    ledger_name: Option<String>,
    ledger_currency: String,
    role: String,
    member_count: i64,
}

#[derive(Debug, Clone, Serialize)]
struct MemberOut {
    user_id: String,
    email: String,
    display_name: Option<String>,
    role: String,
    joined_at: DateTime<Utc>,
    invited_by_user_id: Option<String>,
    is_self: bool,
    avatar_url: Option<String>,
    avatar_version: i64,
}

#[derive(Debug, Deserialize)]
struct MemberRoleUpdateRequest {
    role: String,
}

#[derive(Debug, Deserialize)]
struct TransferOwnershipRequest {
    new_owner_user_id: String,
}

#[derive(Debug)]
struct LedgerInfo {
    name: Option<String>,
    currency: String,
}

async fn create_invite(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(ledger_id): Path<String>,
    Json(request): Json<InviteCreateRequest>,
) -> Result<(StatusCode, Json<InviteCreateResponse>), ApiError> {
    authorize(&principal, "finance:write")?;
    if request.role != ROLE_EDITOR || !(1..=168).contains(&request.expires_in_hours) {
        return Err(invalid("invalid invite role or expiry"));
    }
    let actor = parse_user_id(principal.user_id.as_str())?;
    let access = resolve_ledger_access(&state.pool, actor, &ledger_id, true).await?;
    if access.role != ROLE_OWNER {
        return Err(crate::beecount_collaboration::not_found("Ledger not found"));
    }
    ensure_owner_registry(&state.pool, access.storage_user_id, &ledger_id).await?;
    let mut tx = state.pool.begin().await.map_err(db_error)?;
    sqlx::query("SELECT ledger_id FROM beecount_shared_ledgers WHERE ledger_id=$1 FOR UPDATE")
        .bind(&ledger_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM beecount_ledger_invites \
         WHERE ledger_id=$1 AND used_at IS NULL AND expires_at>now()",
    )
    .bind(&ledger_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;
    if active_count >= MAX_ACTIVE_INVITES {
        return Err(conflict("Too many active invites for this ledger"));
    }
    let code = allocate_invite_code(&mut tx).await?;
    let created_at = Utc::now();
    let expires_at = created_at + Duration::hours(request.expires_in_hours);
    sqlx::query(
        "INSERT INTO beecount_ledger_invites \
         (code,ledger_id,invited_by,target_role,expires_at,created_at) \
         VALUES ($1,$2,$3,'editor',$4,$5)",
    )
    .bind(&code)
    .bind(&ledger_id)
    .bind(actor)
    .bind(expires_at)
    .bind(created_at)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    Ok((
        StatusCode::CREATED,
        Json(InviteCreateResponse {
            formatted_code: format_code(&code),
            share_url: invite_share_url(&state, &code),
            code,
            target_role: ROLE_EDITOR.to_owned(),
            expires_at,
            created_at,
        }),
    ))
}

async fn list_invites(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(ledger_id): Path<String>,
) -> Result<Json<Vec<InviteListItem>>, ApiError> {
    authorize(&principal, "finance:read")?;
    let actor = parse_user_id(principal.user_id.as_str())?;
    let access = resolve_ledger_access(&state.pool, actor, &ledger_id, false).await?;
    if access.role != ROLE_OWNER {
        return Err(crate::beecount_collaboration::not_found("Ledger not found"));
    }
    let rows = sqlx::query(
        "SELECT code,target_role,expires_at,created_at,invited_by \
         FROM beecount_ledger_invites WHERE ledger_id=$1 AND used_at IS NULL \
           AND expires_at>now() ORDER BY created_at DESC",
    )
    .bind(&ledger_id)
    .fetch_all(&state.pool)
    .await
    .map_err(db_error)?;
    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let code: String = row.try_get("code").map_err(internal)?;
        let invited_by: Uuid = row.try_get("invited_by").map_err(internal)?;
        output.push(InviteListItem {
            formatted_code: format_code(&code),
            share_url: invite_share_url(&state, &code),
            code,
            target_role: row.try_get("target_role").map_err(internal)?,
            expires_at: row.try_get("expires_at").map_err(internal)?,
            created_at: row.try_get("created_at").map_err(internal)?,
            invited_by_user_id: beecount_user_id(&state.pool, invited_by).await?,
        });
    }
    Ok(Json(output))
}

async fn revoke_invite(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path((ledger_id, code)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    authorize(&principal, "finance:write")?;
    let actor = parse_user_id(principal.user_id.as_str())?;
    let access = resolve_ledger_access(&state.pool, actor, &ledger_id, true).await?;
    if access.role != ROLE_OWNER {
        return Err(crate::beecount_collaboration::not_found("Ledger not found"));
    }
    let code = normalize_code(&code);
    let row = sqlx::query(
        "UPDATE beecount_ledger_invites SET expires_at=now() \
         WHERE ledger_id=$1 AND code=$2 AND used_at IS NULL RETURNING code",
    )
    .bind(&ledger_id)
    .bind(&code)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?;
    if row.is_none() {
        return Err(crate::beecount_collaboration::not_found("Invite not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn preview_invite(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(code): Path<String>,
) -> Result<Json<InvitePreviewResponse>, ApiError> {
    authorize(&principal, "finance:read")?;
    let code = normalize_code(&code);
    let row = active_invite_row(&state, &code).await?;
    let ledger_id: String = row.try_get("ledger_id").map_err(internal)?;
    let storage_user_id: Uuid = row.try_get("storage_user_id").map_err(internal)?;
    let invited_by: Uuid = row.try_get("invited_by").map_err(internal)?;
    let info = ledger_info(&state, storage_user_id, &ledger_id).await?;
    let display = display_name(&state, invited_by).await?;
    Ok(Json(InvitePreviewResponse {
        code,
        ledger_external_id: ledger_id,
        ledger_name: info.name,
        ledger_currency: info.currency,
        invited_by_display: display,
        target_role: row.try_get("target_role").map_err(internal)?,
        expires_at: row.try_get("expires_at").map_err(internal)?,
    }))
}

async fn accept_invite(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(code): Path<String>,
) -> Result<Json<InviteAcceptResponse>, ApiError> {
    authorize(&principal, "finance:write")?;
    let actor = parse_user_id(principal.user_id.as_str())?;
    let code = normalize_code(&code);
    let now = Utc::now();
    let mut tx = state.pool.begin().await.map_err(db_error)?;
    let invite = sqlx::query(
        "SELECT i.ledger_id,i.invited_by,i.target_role,s.storage_user_id \
         FROM beecount_ledger_invites i JOIN beecount_shared_ledgers s USING (ledger_id) \
         WHERE i.code=$1 AND i.used_at IS NULL AND i.expires_at>$2 FOR UPDATE OF i",
    )
    .bind(&code)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error)?
    .ok_or_else(|| crate::beecount_collaboration::not_found("Invalid or expired invite"))?;
    let ledger_id: String = invite.try_get("ledger_id").map_err(internal)?;
    let invited_by: Uuid = invite.try_get("invited_by").map_err(internal)?;
    let storage_user_id: Uuid = invite.try_get("storage_user_id").map_err(internal)?;
    sqlx::query("SELECT ledger_id FROM beecount_shared_ledgers WHERE ledger_id=$1 FOR UPDATE")
        .bind(&ledger_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM beecount_ledger_members WHERE ledger_id=$1 AND user_id=$2)",
    )
    .bind(&ledger_id)
    .bind(actor)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;
    if exists {
        return Err(conflict("Already a member of this ledger"));
    }
    if actor == invited_by {
        return Err(conflict("Cannot accept your own invite"));
    }
    let member_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM beecount_ledger_members WHERE ledger_id=$1",
    )
    .bind(&ledger_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;
    if member_count >= MEMBER_LIMIT {
        return Err(conflict("Ledger has reached the 5-member limit"));
    }
    sqlx::query(
        "INSERT INTO beecount_ledger_members (ledger_id,user_id,role,invited_by,joined_at) \
         VALUES ($1,$2,'editor',$3,$4)",
    )
    .bind(&ledger_id)
    .bind(actor)
    .bind(invited_by)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    sqlx::query("UPDATE beecount_ledger_invites SET used_at=$2,used_by=$3 WHERE code=$1")
        .bind(&code)
        .bind(now)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    let info = ledger_info(&state, storage_user_id, &ledger_id).await?;
    let wire_actor = beecount_user_id(&state.pool, actor).await?;
    let actor_display = display_name(&state, actor).await?;
    publish_member_event(
        &state,
        &ledger_id,
        json!({
            "type":"member_change","ledgerId":ledger_id,"changeType":"joined",
            "userId":wire_actor,"displayName":actor_display,"role":ROLE_EDITOR,
        }),
        &[],
    )
    .await?;
    Ok(Json(InviteAcceptResponse {
        ledger_external_id: ledger_id,
        ledger_name: info.name,
        ledger_currency: info.currency,
        role: ROLE_EDITOR.to_owned(),
        member_count: member_count + 1,
    }))
}

async fn list_members(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(ledger_id): Path<String>,
) -> Result<Json<Vec<MemberOut>>, ApiError> {
    authorize(&principal, "finance:read")?;
    let actor = parse_user_id(principal.user_id.as_str())?;
    resolve_ledger_access(&state.pool, actor, &ledger_id, false).await?;
    list_members_out(&state, &ledger_id, actor).await.map(Json)
}

async fn update_member_role(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path((ledger_id, wire_user_id)): Path<(String, String)>,
    Json(request): Json<MemberRoleUpdateRequest>,
) -> Result<Json<MemberOut>, ApiError> {
    authorize(&principal, "finance:write")?;
    if request.role != ROLE_EDITOR {
        return Err(invalid("Unsupported role"));
    }
    let actor = parse_user_id(principal.user_id.as_str())?;
    let access = resolve_ledger_access(&state.pool, actor, &ledger_id, true).await?;
    if access.role != ROLE_OWNER {
        return Err(crate::beecount_collaboration::not_found("Ledger not found"));
    }
    let target = internal_user_id(&state.pool, &wire_user_id).await?;
    if target == actor {
        return Err(conflict(
            "Owner cannot change own role; use transfer endpoint",
        ));
    }
    let changed = sqlx::query(
        "UPDATE beecount_ledger_members SET role='editor' \
         WHERE ledger_id=$1 AND user_id=$2 AND role='editor' RETURNING user_id",
    )
    .bind(&ledger_id)
    .bind(target)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?;
    if changed.is_none() {
        return Err(crate::beecount_collaboration::not_found("Member not found"));
    }
    publish_member_event(
        &state,
        &ledger_id,
        json!({"type":"member_change","ledgerId":ledger_id,"changeType":"role_changed","userId":wire_user_id,"newRole":ROLE_EDITOR}),
        &[],
    )
    .await?;
    let members = list_members_out(&state, &ledger_id, actor).await?;
    members
        .into_iter()
        .find(|member| member.user_id == wire_user_id)
        .map(Json)
        .ok_or_else(|| internal("updated member missing"))
}

async fn remove_member(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path((ledger_id, wire_user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    authorize(&principal, "finance:write")?;
    let actor = parse_user_id(principal.user_id.as_str())?;
    let access = resolve_ledger_access(&state.pool, actor, &ledger_id, true).await?;
    let target = internal_user_id(&state.pool, &wire_user_id).await?;
    let target_row =
        sqlx::query("SELECT role FROM beecount_ledger_members WHERE ledger_id=$1 AND user_id=$2")
            .bind(&ledger_id)
            .bind(target)
            .fetch_optional(&state.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| crate::beecount_collaboration::not_found("Member not found"))?;
    let target_role: String = target_row.try_get("role").map_err(internal)?;
    let is_self = target == actor;
    if !is_self && access.role != ROLE_OWNER {
        return Err(crate::beecount_collaboration::not_found("Member not found"));
    }
    if target_role == ROLE_OWNER {
        return Err(conflict("Cannot remove owner; transfer ownership first"));
    }
    sqlx::query("DELETE FROM beecount_ledger_members WHERE ledger_id=$1 AND user_id=$2")
        .bind(&ledger_id)
        .bind(target)
        .execute(&state.pool)
        .await
        .map_err(db_error)?;
    publish_member_event(
        &state,
        &ledger_id,
        json!({"type":"member_change","ledgerId":ledger_id,"changeType":"removed","userId":wire_user_id,"isSelf":is_self}),
        &[target],
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn transfer_ownership(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(ledger_id): Path<String>,
    Json(request): Json<TransferOwnershipRequest>,
) -> Result<Json<Vec<MemberOut>>, ApiError> {
    authorize(&principal, "finance:write")?;
    let actor = parse_user_id(principal.user_id.as_str())?;
    let access = resolve_ledger_access(&state.pool, actor, &ledger_id, true).await?;
    if access.role != ROLE_OWNER {
        return Err(crate::beecount_collaboration::not_found("Ledger not found"));
    }
    let target = internal_user_id(&state.pool, request.new_owner_user_id.trim()).await?;
    if target == actor {
        return Err(conflict("Target is already the owner"));
    }
    let mut tx = state.pool.begin().await.map_err(db_error)?;
    sqlx::query("SELECT ledger_id FROM beecount_shared_ledgers WHERE ledger_id=$1 FOR UPDATE")
        .bind(&ledger_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    let target_role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM beecount_ledger_members WHERE ledger_id=$1 AND user_id=$2",
    )
    .bind(&ledger_id)
    .bind(target)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error)?;
    if target_role.as_deref() != Some(ROLE_EDITOR) {
        return Err(crate::beecount_collaboration::not_found(
            "Target user is not a member of this ledger",
        ));
    }
    sqlx::query("UPDATE beecount_ledger_members SET role='editor' WHERE ledger_id=$1 AND user_id=$2 AND role='owner'")
        .bind(&ledger_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    sqlx::query(
        "UPDATE beecount_ledger_members SET role='owner' WHERE ledger_id=$1 AND user_id=$2",
    )
    .bind(&ledger_id)
    .bind(target)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    let wire_target = beecount_user_id(&state.pool, target).await?;
    let wire_actor = beecount_user_id(&state.pool, actor).await?;
    for (wire_id, role) in [(&wire_target, ROLE_OWNER), (&wire_actor, ROLE_EDITOR)] {
        publish_member_event(
            &state,
            &ledger_id,
            json!({"type":"member_change","ledgerId":ledger_id,"changeType":"role_changed","userId":wire_id,"newRole":role}),
            &[],
        )
        .await?;
    }
    list_members_out(&state, &ledger_id, actor).await.map(Json)
}

#[derive(Debug, Serialize)]
struct SharedResourcesResponse {
    owner_user_id: String,
    categories: Vec<Value>,
    accounts: Vec<Value>,
    tags: Vec<Value>,
}

async fn shared_resources(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(ledger_id): Path<String>,
) -> Result<Json<SharedResourcesResponse>, ApiError> {
    authorize(&principal, "finance:read")?;
    let actor = parse_user_id(principal.user_id.as_str())?;
    resolve_ledger_access(&state.pool, actor, &ledger_id, false).await?;
    let owner_user_id: Uuid = sqlx::query_scalar(
        "SELECT user_id FROM beecount_ledger_members WHERE ledger_id=$1 AND role='owner'",
    )
    .bind(&ledger_id)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;
    let rows = sqlx::query(
        "SELECT entity_type,entity_id,payload FROM sync_entities \
         WHERE user_id=$1 AND is_deleted=FALSE \
           AND entity_type=ANY($2) ORDER BY entity_type,entity_id",
    )
    .bind(owner_user_id)
    .bind(vec![
        "finance.category".to_owned(),
        "finance.account".to_owned(),
        "finance.tag".to_owned(),
    ])
    .fetch_all(&state.pool)
    .await
    .map_err(db_error)?;
    let mut categories = Vec::new();
    let mut accounts = Vec::new();
    let mut tags = Vec::new();
    for row in rows {
        let entity_type: String = row.try_get("entity_type").map_err(internal)?;
        let entity_id: String = row.try_get("entity_id").map_err(internal)?;
        let payload: Value = row.try_get("payload").map_err(internal)?;
        let Some(kind) = crate::beecount_compat::BeeCountEntityKind::from_lifetrace(&entity_type)
        else {
            continue;
        };
        let sync_id = crate::beecount_compat::beecount_wire_id(&entity_id);
        let raw =
            crate::beecount_compat::beecount_payload(kind, &sync_id, &payload).map_err(internal)?;
        match kind {
            crate::beecount_compat::BeeCountEntityKind::Category => categories.push(json!({
                "sync_id": sync_id,
                "name": raw.get("name"),
                "kind": raw.get("kind").or_else(|| raw.get("type")),
                "icon": raw.get("icon"),
                "icon_type": raw.get("iconType"),
                "icon_cloud_file_id": raw.get("iconCloudFileId"),
                "icon_cloud_sha256": raw.get("iconCloudSha256"),
                "sort_order": raw.get("sortOrder"),
                "level": raw.get("level"),
                "parent_name": raw.get("parentName"),
                "parent_sync_id": raw.get("parentSyncId"),
            })),
            crate::beecount_compat::BeeCountEntityKind::Account => accounts.push(json!({
                "sync_id": sync_id,
                "name": raw.get("name"),
                "account_type": raw.get("type").or_else(|| raw.get("accountType")),
                "currency": raw.get("currency"),
                "initial_balance": raw.get("initialBalance"),
                "note": raw.get("note"),
                "credit_limit": raw.get("creditLimit"),
                "billing_day": raw.get("billingDay"),
                "payment_due_day": raw.get("paymentDueDay"),
                "bank_name": raw.get("bankName"),
                "card_last_four": raw.get("cardLastFour"),
            })),
            crate::beecount_compat::BeeCountEntityKind::Tag => tags.push(json!({
                "sync_id": sync_id,"name":raw.get("name"),"color":raw.get("color"),
            })),
            _ => {}
        }
    }
    Ok(Json(SharedResourcesResponse {
        owner_user_id: beecount_user_id(&state.pool, owner_user_id).await?,
        categories,
        accounts,
        tags,
    }))
}

async fn allocate_invite_code(tx: &mut Transaction<'_, Postgres>) -> Result<String, ApiError> {
    for _ in 0..8 {
        let mut rng = OsRng;
        let code = (0..6)
            .map(|_| *INVITE_ALPHABET.choose(&mut rng).expect("invite alphabet") as char)
            .collect::<String>();
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM beecount_ledger_invites WHERE code=$1)",
        )
        .bind(&code)
        .fetch_one(&mut **tx)
        .await
        .map_err(db_error)?;
        if !exists {
            return Ok(code);
        }
    }
    Err(ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "Failed to allocate unique invite code; please retry",
        StatusCode::SERVICE_UNAVAILABLE,
    ))
}

async fn active_invite_row(
    state: &AppState,
    code: &str,
) -> Result<sqlx::postgres::PgRow, ApiError> {
    sqlx::query(
        "SELECT i.ledger_id,i.invited_by,i.target_role,i.expires_at,s.storage_user_id \
         FROM beecount_ledger_invites i JOIN beecount_shared_ledgers s USING (ledger_id) \
         WHERE i.code=$1 AND i.used_at IS NULL AND i.expires_at>now()",
    )
    .bind(code)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| crate::beecount_collaboration::not_found("Invalid or expired invite"))
}

async fn ledger_info(
    state: &AppState,
    storage_user_id: Uuid,
    ledger_id: &str,
) -> Result<LedgerInfo, ApiError> {
    let row = sqlx::query(
        "SELECT payload FROM sync_entities WHERE user_id=$1 AND entity_type='finance.ledger' \
         AND is_deleted=FALSE AND (entity_id=$2 OR payload->>'beecountLedgerId'=$3)",
    )
    .bind(storage_user_id)
    .bind(crate::beecount_compat::lifetrace_entity_id(ledger_id))
    .bind(ledger_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| crate::beecount_collaboration::not_found("Ledger not found"))?;
    let payload: Value = row.try_get("payload").map_err(internal)?;
    Ok(LedgerInfo {
        name: payload
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned),
        currency: payload
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("CNY")
            .to_owned(),
    })
}

async fn display_name(state: &AppState, user_id: Uuid) -> Result<String, ApiError> {
    let row = sqlx::query("SELECT email,display_name FROM cloud_users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)?;
    let display: Option<String> = row.try_get("display_name").map_err(internal)?;
    let email: Option<String> = row.try_get("email").map_err(internal)?;
    Ok(display
        .filter(|value| !value.trim().is_empty())
        .or_else(|| email.and_then(|value| value.split('@').next().map(str::to_owned)))
        .unwrap_or_else(|| "Unknown".to_owned()))
}

async fn list_members_out(
    state: &AppState,
    ledger_id: &str,
    caller_user_id: Uuid,
) -> Result<Vec<MemberOut>, ApiError> {
    let rows = sqlx::query(
        "SELECT m.user_id,m.role,m.joined_at,m.invited_by,u.email,u.display_name, \
                COALESCE(l.beecount_user_id,u.id::text) AS wire_user_id, \
                p.avatar_version,p.avatar_content IS NOT NULL AS has_avatar, \
                COALESCE(il.beecount_user_id,m.invited_by::text) AS invited_by_wire \
         FROM beecount_ledger_members m JOIN cloud_users u ON u.id=m.user_id \
         LEFT JOIN beecount_identity_links l ON l.user_id=m.user_id \
         LEFT JOIN beecount_user_profiles p ON p.user_id=m.user_id \
         LEFT JOIN beecount_identity_links il ON il.user_id=m.invited_by \
         WHERE m.ledger_id=$1 ORDER BY m.joined_at",
    )
    .bind(ledger_id)
    .fetch_all(&state.pool)
    .await
    .map_err(db_error)?;
    rows.into_iter()
        .map(|row| {
            let user_id: Uuid = row.try_get("user_id").map_err(internal)?;
            let wire_id: String = row.try_get("wire_user_id").map_err(internal)?;
            let version = row
                .try_get::<Option<i64>, _>("avatar_version")
                .map_err(internal)?
                .unwrap_or(0);
            let has_avatar = row.try_get::<bool, _>("has_avatar").unwrap_or(false);
            Ok(MemberOut {
                avatar_url: has_avatar.then(|| avatar_url(&wire_id, version)),
                avatar_version: version,
                user_id: wire_id,
                email: row
                    .try_get::<Option<String>, _>("email")
                    .map_err(internal)?
                    .unwrap_or_default(),
                display_name: row.try_get("display_name").map_err(internal)?,
                role: row.try_get("role").map_err(internal)?,
                joined_at: row.try_get("joined_at").map_err(internal)?,
                invited_by_user_id: row.try_get("invited_by_wire").map_err(internal)?,
                is_self: user_id == caller_user_id,
            })
        })
        .collect()
}

async fn publish_member_event(
    state: &AppState,
    ledger_id: &str,
    payload: Value,
    extra_users: &[Uuid],
) -> Result<(), ApiError> {
    let mut users = member_user_ids(&state.pool, ledger_id).await?;
    users.extend_from_slice(extra_users);
    users.sort_unstable();
    users.dedup();
    for user_id in users {
        state
            .beecount_realtime
            .publish(&user_id.to_string(), payload.clone());
    }
    Ok(())
}

fn normalize_code(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

fn format_code(code: &str) -> String {
    if code.len() == 6 {
        format!("{} {}", &code[..3], &code[3..])
    } else {
        code.to_owned()
    }
}

fn invite_share_url(state: &AppState, code: &str) -> String {
    let origin = state
        .config
        .public_web_base_url
        .as_deref()
        .unwrap_or("https://count.beejz.com")
        .trim_end_matches('/');
    format!("{origin}/invite/{code}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_code_normalization_matches_stock_client() {
        assert_eq!(normalize_code("abc-234"), "ABC234");
        assert_eq!(format_code("ABC234"), "ABC 234");
    }

    #[test]
    fn profile_validation_rejects_non_object_settings() {
        let request = ProfilePatch {
            display_name: None,
            income_is_red: None,
            theme_primary_color: Some("#12ABEF".to_owned()),
            appearance: Some(json!([])),
            ai_config: None,
            primary_currency: Some("cny".to_owned()),
        };
        assert!(validate_profile_patch(&request).is_err());
    }
}
