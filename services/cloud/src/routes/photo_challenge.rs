//! Disposable photography challenge built on top of the reusable photo staging relay.
//!
//! Original files are staged for the owner's desktop photo library. GLM only
//! receives a client-generated preview, while the challenge keeps lightweight
//! scoring metadata and a small thumbnail until the challenge is retired.

use std::process::Stdio;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Utc};
use lifetrace_contracts::{ErrorCode, UserId};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use uuid::Uuid;

use crate::auth::security::cookie_value;
use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::error::ApiError;
use crate::state::AppState;

use super::photo_staging::{self, StageInput, MAX_STAGED_PHOTO_BYTES};

const MAX_MODEL_PREVIEW_BYTES: usize = 2 * 1024 * 1024;
const MAX_THUMBNAIL_BYTES: usize = 180 * 1024;
const HIGH_SCORE_THRESHOLD: i64 = 90;
// “超过 500 张” means the first winning state is 501 qualifying photos.
const REQUIRED_HIGH_SCORE_COUNT: usize = 501;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScoreBreakdown {
    composition: i64,
    light_color: i64,
    subject_story: i64,
    technical: i64,
    originality: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelScore {
    #[serde(
        default,
        alias = "total",
        alias = "totalScore",
        alias = "total_score",
        deserialize_with = "deserialize_score_number"
    )]
    score: i64,
    #[serde(
        alias = "composition_score",
        alias = "compositionScore",
        alias = "构图",
        deserialize_with = "deserialize_score_number"
    )]
    composition: i64,
    #[serde(
        alias = "light_color",
        alias = "lightAndColor",
        alias = "lightingColor",
        alias = "lighting_color",
        alias = "光线与色彩",
        alias = "光线色彩",
        deserialize_with = "deserialize_score_number"
    )]
    light_color: i64,
    #[serde(
        alias = "subject_story",
        alias = "subjectAndStory",
        alias = "subjectNarrative",
        alias = "subject_narrative",
        alias = "主体与叙事",
        alias = "主体叙事",
        deserialize_with = "deserialize_score_number"
    )]
    subject_story: i64,
    #[serde(
        alias = "technicalQuality",
        alias = "technical_quality",
        alias = "技术质量",
        deserialize_with = "deserialize_score_number"
    )]
    technical: i64,
    #[serde(
        alias = "originalityMoment",
        alias = "originality_moment",
        alias = "原创性与瞬间感",
        alias = "原创性",
        deserialize_with = "deserialize_score_number"
    )]
    originality: i64,
    #[serde(
        default,
        alias = "comment",
        alias = "comments",
        alias = "review",
        alias = "评价",
        alias = "反馈"
    )]
    feedback: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeStats {
    total: usize,
    high_score_count: usize,
    remaining: usize,
    target: usize,
    achieved: bool,
    average_score: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScoreResponse {
    id: String,
    score: i64,
    qualified: bool,
    breakdown: ScoreBreakdown,
    feedback: String,
    duplicate: bool,
    stats: ChallengeStats,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeEntry {
    id: String,
    file_name: Option<String>,
    captured_at: Option<DateTime<Utc>>,
    score: i64,
    qualified: bool,
    breakdown: ScoreBreakdown,
    feedback: String,
    model: String,
    thumbnail_data_url: Option<String>,
    scored_at: DateTime<Utc>,
    staging_pending: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminResponse {
    stats: ChallengeStats,
    entries: Vec<ChallengeEntry>,
}

#[derive(Debug, Deserialize)]
struct ProviderResponse {
    #[serde(default)]
    choices: Vec<ProviderChoice>,
}

#[derive(Debug, Deserialize)]
struct ProviderChoice {
    message: ProviderMessage,
}

#[derive(Debug, Deserialize)]
struct ProviderMessage {
    content: String,
}

struct ChallengeUpload {
    original_name: String,
    mime_type: String,
    original: Vec<u8>,
    preview_data_url: String,
    thumbnail_data_url: String,
    captured_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/photo-challenge/summary", get(public_summary))
        .route("/api/v1/photo-challenge/score", post(score_photo))
        .route("/api/v1/photo-challenge/admin", get(admin))
        .layer(DefaultBodyLimit::max(
            MAX_STAGED_PHOTO_BYTES + 4 * 1024 * 1024,
        ))
}

async fn public_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ChallengeStats>, ApiError> {
    verify_challenge_key(&headers)?;
    let owner = challenge_owner(&state).await?;
    Ok(Json(load_stats(&state, &owner).await?))
}

async fn admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminResponse>, ApiError> {
    let principal = web_principal(&state, &headers).await?;
    let owner = challenge_owner(&state).await?;
    if principal.user_id != owner {
        return Err(ApiError::new(
            ErrorCode::AuthScopeDenied,
            "只有摄影挑战所属账号可以查看详情",
            StatusCode::FORBIDDEN,
        ));
    }
    let owner_uuid = user_uuid(&owner)?;
    let rows = sqlx::query(
        "SELECT id,file_name,captured_at,score,qualified,breakdown,feedback,model,thumbnail_data_url,scored_at,staging_id \
         FROM photo_challenge_scores WHERE user_id=$1 ORDER BY scored_at DESC LIMIT 1000",
    )
    .bind(owner_uuid)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    let entries = rows
        .iter()
        .map(row_to_entry)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(AdminResponse {
        stats: load_stats(&state, &owner).await?,
        entries,
    }))
}

async fn score_photo(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Json<ScoreResponse>, ApiError> {
    verify_challenge_key(&headers)?;
    let owner = challenge_owner(&state).await?;
    let owner_uuid = user_uuid(&owner)?;
    let upload = read_challenge_upload(multipart).await?;
    let image_hash = hex::encode(Sha256::digest(&upload.original));

    if let Some(row) = sqlx::query(
        "SELECT id,score,qualified,breakdown,feedback FROM photo_challenge_scores WHERE user_id=$1 AND image_hash=$2",
    )
    .bind(owner_uuid)
    .bind(&image_hash)
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    {
        let mut response = response_from_row(&row, true)?;
        response.stats = load_stats(&state, &owner).await?;
        return Ok(Json(response));
    }

    // The original is queued first so even if the AI provider is temporarily
    // unavailable, the photo still reaches the owner's desktop library.
    let staged = photo_staging::stage_for_user(
        &state,
        &owner,
        StageInput {
            source: "photo-challenge".to_owned(),
            client_asset_id: Some(image_hash.clone()),
            original_name: upload.original_name.clone(),
            media_type: "image".to_owned(),
            mime_type: upload.mime_type.clone(),
            captured_at: upload.captured_at,
            content: upload.original,
        },
    )
    .await?;

    let model_score = call_glm_score(&upload.preview_data_url).await?;
    let breakdown = ScoreBreakdown {
        composition: model_score.composition.clamp(0, 25),
        light_color: model_score.light_color.clamp(0, 20),
        subject_story: model_score.subject_story.clamp(0, 20),
        technical: model_score.technical.clamp(0, 20),
        originality: model_score.originality.clamp(0, 15),
    };
    let score = breakdown.composition
        + breakdown.light_color
        + breakdown.subject_story
        + breakdown.technical
        + breakdown.originality;
    let qualified = score > HIGH_SCORE_THRESHOLD;
    let id = Uuid::new_v4();
    let model = env_non_empty("PHOTO_CHALLENGE_MODEL").unwrap_or_else(|| "glm-4v-flash".to_owned());
    let feedback: String = model_score.feedback.trim().chars().take(500).collect();
    let staging_id = Uuid::parse_str(&staged.id).map_err(|_| bad_request("暂存照片编号无效"))?;

    let row = sqlx::query(
        "INSERT INTO photo_challenge_scores \
         (id,user_id,staging_id,image_hash,file_name,captured_at,score,qualified,breakdown,feedback,model,thumbnail_data_url) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
         RETURNING id,score,qualified,breakdown,feedback",
    )
    .bind(id)
    .bind(owner_uuid)
    .bind(staging_id)
    .bind(&image_hash)
    .bind(&upload.original_name)
    .bind(upload.captured_at)
    .bind(score)
    .bind(qualified)
    .bind(serde_json::to_value(&breakdown).map_err(|_| bad_request("评分明细无效"))?)
    .bind(&feedback)
    .bind(&model)
    .bind(&upload.thumbnail_data_url)
    .fetch_one(&state.pool)
    .await
    .map_err(database_error)?;

    let mut response = response_from_row(&row, false)?;
    response.stats = load_stats(&state, &owner).await?;
    Ok(Json(response))
}

async fn read_challenge_upload(mut multipart: Multipart) -> Result<ChallengeUpload, ApiError> {
    let mut original_name = None;
    let mut mime_type = None;
    let mut original = None;
    let mut preview_data_url = None;
    let mut thumbnail_data_url = None;
    let mut captured_at = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| bad_request("上传表单无效"))?
    {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "file" => {
                let file_name = field.file_name().unwrap_or("photo.jpg").to_owned();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_owned();
                if !matches!(
                    content_type.as_str(),
                    "image/jpeg" | "image/png" | "image/webp"
                ) {
                    return Err(bad_request("摄影挑战仅支持 JPEG、PNG 或 WebP"));
                }
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| bad_request("读取原始照片失败"))?;
                if bytes.is_empty() || bytes.len() > MAX_STAGED_PHOTO_BYTES {
                    return Err(bad_request("原始照片为空或超过 64 MiB"));
                }
                original_name = Some(clean_file_name(&file_name));
                mime_type = Some(content_type);
                original = Some(bytes.to_vec());
            }
            "previewDataUrl" => {
                let value = field
                    .text()
                    .await
                    .map_err(|_| bad_request("评分预览无效"))?;
                let decoded = decode_data_url(&value, MAX_MODEL_PREVIEW_BYTES, "评分预览")?;
                preview_data_url = Some(format!("data:{};base64,{}", decoded.mime, decoded.base64));
            }
            "thumbnailDataUrl" => {
                let value = field.text().await.map_err(|_| bad_request("缩略图无效"))?;
                let decoded = decode_data_url(&value, MAX_THUMBNAIL_BYTES, "缩略图")?;
                thumbnail_data_url =
                    Some(format!("data:{};base64,{}", decoded.mime, decoded.base64));
            }
            "capturedAt" => {
                let value = field
                    .text()
                    .await
                    .map_err(|_| bad_request("拍摄时间无效"))?;
                if !value.trim().is_empty() {
                    captured_at = Some(
                        DateTime::parse_from_rfc3339(value.trim())
                            .map_err(|_| bad_request("拍摄时间格式无效"))?
                            .with_timezone(&Utc),
                    );
                }
            }
            _ => {}
        }
    }

    Ok(ChallengeUpload {
        original_name: original_name.ok_or_else(|| bad_request("缺少照片文件"))?,
        mime_type: mime_type.ok_or_else(|| bad_request("缺少照片 MIME 类型"))?,
        original: original.ok_or_else(|| bad_request("缺少照片文件"))?,
        preview_data_url: preview_data_url.ok_or_else(|| bad_request("缺少评分预览"))?,
        thumbnail_data_url: thumbnail_data_url.ok_or_else(|| bad_request("缺少缩略图"))?,
        captured_at,
    })
}

struct DecodedImage {
    mime: String,
    base64: String,
}

fn decode_data_url(
    data_url: &str,
    max_bytes: usize,
    label: &str,
) -> Result<DecodedImage, ApiError> {
    let (prefix, encoded) = data_url
        .split_once(',')
        .ok_or_else(|| bad_request(format!("{label}格式无效")))?;
    let mime = prefix
        .strip_prefix("data:")
        .and_then(|value| value.strip_suffix(";base64"))
        .filter(|value| matches!(*value, "image/jpeg" | "image/png" | "image/webp"))
        .ok_or_else(|| bad_request(format!("{label}仅支持 JPEG、PNG 或 WebP")))?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| bad_request(format!("{label} Base64 数据无效")))?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(bad_request(format!("{label}大小超出限制")));
    }
    Ok(DecodedImage {
        mime: mime.to_owned(),
        base64: encoded.to_owned(),
    })
}

fn verify_challenge_key(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = env_non_empty("PHOTO_CHALLENGE_ACCESS_KEY").ok_or_else(|| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "摄影挑战尚未配置访问口令",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    let provided = headers
        .get("x-photo-challenge-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if provided.is_empty() || provided != expected {
        return Err(ApiError::new(
            ErrorCode::AuthInvalid,
            "摄影挑战口令无效",
            StatusCode::UNAUTHORIZED,
        ));
    }
    Ok(())
}

async fn challenge_owner(state: &AppState) -> Result<UserId, ApiError> {
    if !state.database_enabled {
        return Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "摄影挑战云端模式需要 PostgreSQL",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }
    let email = env_non_empty("PHOTO_CHALLENGE_OWNER_EMAIL")
        .map(|value| value.to_lowercase())
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::TemporarilyUnavailable,
                "摄影挑战尚未配置所属账号",
                StatusCode::SERVICE_UNAVAILABLE,
            )
        })?;
    let id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM cloud_users WHERE email_normalized=$1 AND status='active' LIMIT 1",
    )
    .bind(email.trim())
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    .ok_or_else(|| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "摄影挑战所属账号不存在",
            StatusCode::NOT_FOUND,
        )
    })?;
    Ok(UserId::new(id.to_string()))
}

async fn web_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedPrincipal, ApiError> {
    let raw = cookie_value(headers, &state.config.auth_cookie_name);
    state
        .auth
        .authenticate(AuthCredential::WebSession(raw.as_deref()))
        .await
}

async fn load_stats(state: &AppState, owner: &UserId) -> Result<ChallengeStats, ApiError> {
    let owner_uuid = user_uuid(owner)?;
    let row = sqlx::query(
        "SELECT COUNT(*) AS total,COUNT(*) FILTER (WHERE qualified) AS high_count,COALESCE(AVG(score),0)::float8 AS average_score \
         FROM photo_challenge_scores WHERE user_id=$1",
    )
    .bind(owner_uuid)
    .fetch_one(&state.pool)
    .await
    .map_err(database_error)?;
    let total = row
        .try_get::<i64, _>("total")
        .map_err(database_error)?
        .max(0) as usize;
    let high_score_count = row
        .try_get::<i64, _>("high_count")
        .map_err(database_error)?
        .max(0) as usize;
    let average_score = row
        .try_get::<f64, _>("average_score")
        .map_err(database_error)?;
    Ok(ChallengeStats {
        total,
        high_score_count,
        remaining: REQUIRED_HIGH_SCORE_COUNT.saturating_sub(high_score_count),
        target: REQUIRED_HIGH_SCORE_COUNT,
        achieved: high_score_count >= REQUIRED_HIGH_SCORE_COUNT,
        average_score: (average_score * 10.0).round() / 10.0,
    })
}

fn response_from_row(
    row: &sqlx::postgres::PgRow,
    duplicate: bool,
) -> Result<ScoreResponse, ApiError> {
    let breakdown_value: Value = row.try_get("breakdown").map_err(database_error)?;
    let breakdown: ScoreBreakdown =
        serde_json::from_value(breakdown_value).map_err(|_| bad_request("云端评分明细损坏"))?;
    Ok(ScoreResponse {
        id: row
            .try_get::<Uuid, _>("id")
            .map_err(database_error)?
            .to_string(),
        score: row.try_get("score").map_err(database_error)?,
        qualified: row.try_get("qualified").map_err(database_error)?,
        breakdown,
        feedback: row.try_get("feedback").map_err(database_error)?,
        duplicate,
        stats: ChallengeStats {
            total: 0,
            high_score_count: 0,
            remaining: REQUIRED_HIGH_SCORE_COUNT,
            target: REQUIRED_HIGH_SCORE_COUNT,
            achieved: false,
            average_score: 0.0,
        },
    })
}

fn row_to_entry(row: &sqlx::postgres::PgRow) -> Result<ChallengeEntry, ApiError> {
    let breakdown_value: Value = row.try_get("breakdown").map_err(database_error)?;
    Ok(ChallengeEntry {
        id: row
            .try_get::<Uuid, _>("id")
            .map_err(database_error)?
            .to_string(),
        file_name: row.try_get("file_name").map_err(database_error)?,
        captured_at: row.try_get("captured_at").map_err(database_error)?,
        score: row.try_get("score").map_err(database_error)?,
        qualified: row.try_get("qualified").map_err(database_error)?,
        breakdown: serde_json::from_value(breakdown_value)
            .map_err(|_| bad_request("云端评分明细损坏"))?,
        feedback: row.try_get("feedback").map_err(database_error)?,
        model: row.try_get("model").map_err(database_error)?,
        thumbnail_data_url: row.try_get("thumbnail_data_url").map_err(database_error)?,
        scored_at: row.try_get("scored_at").map_err(database_error)?,
        staging_pending: row
            .try_get::<Option<Uuid>, _>("staging_id")
            .map_err(database_error)?
            .is_some(),
    })
}

async fn call_glm_score(image_data_url: &str) -> Result<ModelScore, ApiError> {
    let api_key = env_non_empty("ZHIPU_API_KEY").ok_or_else(|| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "缺少 ZHIPU_API_KEY，无法进行照片评分",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    if api_key.contains(['\r', '\n']) {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "ZHIPU_API_KEY 配置无效",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }
    let base_url = env_non_empty("ZHIPU_BASE_URL")
        .unwrap_or_else(|| "https://open.bigmodel.cn/api/paas/v4".to_owned());
    let model = env_non_empty("PHOTO_CHALLENGE_MODEL").unwrap_or_else(|| "glm-4v-flash".to_owned());
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let rubric = r#"你是一名严格、稳定的摄影比赛评委。请只评价照片本身，不因鼓励用户而抬高分数。按以下固定量表打分，总分100：构图 composition 0-25；光线与色彩 lightColor 0-20；主体与叙事 subjectStory 0-20；对焦/曝光/噪点等技术质量 technical 0-20；原创性与瞬间感 originality 0-15。90分代表已经达到非常优秀、少见的作品水平，普通好看的照片应明显低于90。只输出一个JSON对象，不要Markdown，不要额外文字，字段必须为：score, composition, lightColor, subjectStory, technical, originality, feedback。所有分数字段必须是JSON数字，不要带“分”、斜杠或其他单位；feedback用中文，最多80字，指出最关键优点和一个最值得改进的点。"#;
    let request = json!({
        "model": model,
        "temperature": 0.1,
        "stream": false,
        "max_tokens": 600,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "image_url", "image_url": {"url": image_data_url}},
                {"type": "text", "text": rubric}
            ]
        }]
    });

    let provider = tokio::time::timeout(
        Duration::from_secs(55),
        call_provider(&endpoint, &api_key, &request),
    )
    .await
    .map_err(|_| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "GLM-4V-Flash 评分超时",
            StatusCode::GATEWAY_TIMEOUT,
        )
    })?
    .map_err(|message| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            format!("GLM-4V-Flash 评分失败: {message}"),
            StatusCode::BAD_GATEWAY,
        )
    })?;
    let content = provider
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::TemporarilyUnavailable,
                "GLM-4V-Flash 未返回评分内容",
                StatusCode::BAD_GATEWAY,
            )
        })?;
    if let Some(score) = parse_model_score(&content) {
        return Ok(score);
    }

    let raw_sample: String = content.chars().take(2_000).collect();
    eprintln!("[photo-challenge] failed to parse GLM score response: {raw_sample}");
    Err(ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "GLM-4V-Flash 返回的评分格式无效",
        StatusCode::BAD_GATEWAY,
    ))
}

async fn call_provider(
    endpoint: &str,
    api_key: &str,
    request: &Value,
) -> Result<ProviderResponse, String> {
    let payload = serde_json::to_vec(request).map_err(|error| error.to_string())?;
    let mut child = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--fail-with-body")
        .arg("--max-time")
        .arg("50")
        .arg("--request")
        .arg("POST")
        .arg("--url")
        .arg(endpoint)
        .arg("--header")
        .arg(format!("Authorization: Bearer {api_key}"))
        .arg("--header")
        .arg("Content-Type: application/json")
        .arg("--data-binary")
        .arg("@-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "无法打开评分请求输入流".to_owned())?;
    stdin
        .write_all(&payload)
        .await
        .map_err(|error| error.to_string())?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .await
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let body = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if !body.is_empty() {
            body
        } else if !error.is_empty() {
            error
        } else {
            format!("HTTP 调用失败: {}", output.status)
        });
    }
    serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())
}

fn deserialize_score_number<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| {
                number
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .map(|v| v.round() as i64)
            })
            .ok_or_else(|| D::Error::custom("score must be numeric")),
        Value::String(text) => parse_score_text(&text)
            .ok_or_else(|| D::Error::custom("score string must start with a number")),
        _ => Err(D::Error::custom("score must be a number or numeric string")),
    }
}

fn parse_score_text(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value);
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        if value.is_finite() {
            return Some(value.round() as i64);
        }
    }

    let numeric_prefix: String = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit() || matches!(*character, '+' | '-' | '.'))
        .collect();
    if numeric_prefix.is_empty() || matches!(numeric_prefix.as_str(), "+" | "-" | ".") {
        return None;
    }
    numeric_prefix
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value.round() as i64)
}

fn parse_model_score(content: &str) -> Option<ModelScore> {
    if let Ok(parsed) = serde_json::from_str::<ModelScore>(content.trim()) {
        return Some(parsed);
    }

    // Model providers sometimes wrap JSON in Markdown or add a sentence before/after it.
    // Try each object start and deserialize one JSON object without requiring the rest of
    // the response to be empty.
    for (start, character) in content.char_indices() {
        if character != '{' {
            continue;
        }
        let mut deserializer = serde_json::Deserializer::from_str(&content[start..]);
        if let Ok(parsed) = ModelScore::deserialize(&mut deserializer) {
            return Some(parsed);
        }
    }
    None
}

fn clean_file_name(value: &str) -> String {
    std::path::Path::new(value)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("photo.jpg")
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect()
}

fn user_uuid(user_id: &UserId) -> Result<Uuid, ApiError> {
    Uuid::parse_str(user_id.as_str()).map_err(|_| bad_request("摄影挑战所属账号无效"))
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

fn database_error(error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        format!("摄影挑战数据库操作失败: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_total_is_derived_from_rubric_parts() {
        let content = r#"{"score":99,"composition":23,"lightColor":18,"subjectStory":17,"technical":18,"originality":13,"feedback":"不错"}"#;
        let parsed = parse_model_score(content).expect("score");
        assert_eq!(
            parsed.composition
                + parsed.light_color
                + parsed.subject_story
                + parsed.technical
                + parsed.originality,
            89
        );
    }

    #[test]
    fn model_score_parser_accepts_fenced_noise() {
        let content = "```json\n{\"score\":91,\"composition\":23,\"lightColor\":18,\"subjectStory\":18,\"technical\":18,\"originality\":14,\"feedback\":\"ok\"}\n```";
        assert_eq!(parse_model_score(content).expect("score").score, 91);
    }

    #[test]
    fn model_score_parser_accepts_aliases_and_numeric_strings() {
        let content = r#"评分如下：
        {"total_score":"91","composition":"23/25","light_color":"18分","subject_story":18.0,"technical_quality":"18","originality_moment":14,"review":"构图稳定"}
        请参考。"#;
        let parsed = parse_model_score(content).expect("score");
        assert_eq!(parsed.score, 91);
        assert_eq!(parsed.composition, 23);
        assert_eq!(parsed.light_color, 18);
        assert_eq!(parsed.subject_story, 18);
        assert_eq!(parsed.technical, 18);
        assert_eq!(parsed.originality, 14);
        assert_eq!(parsed.feedback, "构图稳定");
    }

    #[test]
    fn model_score_parser_accepts_chinese_field_names() {
        let content = r#"{"构图":22,"光线与色彩":17,"主体与叙事":18,"技术质量":17,"原创性与瞬间感":13,"反馈":"不错"}"#;
        let parsed = parse_model_score(content).expect("score");
        assert_eq!(parsed.score, 0);
        assert_eq!(parsed.composition, 22);
        assert_eq!(parsed.light_color, 17);
        assert_eq!(parsed.subject_story, 18);
        assert_eq!(parsed.technical, 17);
        assert_eq!(parsed.originality, 13);
        assert_eq!(parsed.feedback, "不错");
    }
}
