//! Public PWA photo scoring endpoint for the private LifeTrace photography challenge.
//!
//! The browser never receives the Zhipu API key. A separate challenge passphrase
//! authorizes uploads, while scored records are written into the configured
//! owner's normal LifeTrace sync stream as `photo.challenge_entry` entities.

use std::process::Stdio;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use lifetrace_contracts::ids::{ChangeId, DeviceId, EntityId, RequestId};
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::sync::v1::{
    AppId, ChangeOperation, ClientPlatform, PushChangeResultV1, PushRequestV1, SyncChangeV1,
    SyncClientInfo,
};
use lifetrace_contracts::{ErrorCode, UserId, PROTOCOL_VERSION, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::types::Uuid;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::ApiError;
use crate::state::AppState;

const ENTITY_TYPE: &str = "photo.challenge_entry";
const DEVICE_ID: &str = "photo-challenge-pwa";
const MAX_IMAGE_BYTES: usize = 2 * 1024 * 1024;
const MAX_THUMBNAIL_BYTES: usize = 180 * 1024;
const HIGH_SCORE_THRESHOLD: i64 = 90;
const REQUIRED_HIGH_SCORE_COUNT: usize = 501;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScoreRequest {
    file_name: Option<String>,
    image_data_url: String,
    thumbnail_data_url: String,
    captured_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScoreBreakdown {
    composition: i64,
    light_color: i64,
    subject_story: i64,
    technical: i64,
    originality: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelScore {
    #[serde(default)]
    score: i64,
    composition: i64,
    light_color: i64,
    subject_story: i64,
    technical: i64,
    originality: i64,
    feedback: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeStats {
    total: usize,
    high_score_count: usize,
    remaining: usize,
    target: usize,
    achieved: bool,
    average_score: f64,
}

#[derive(Debug, Serialize)]
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

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/photo-challenge/summary", get(summary))
        .route("/api/v1/photo-challenge/score", post(score_photo))
}

async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ChallengeStats>, ApiError> {
    verify_challenge_key(&headers)?;
    let owner = challenge_owner(&state).await?;
    Ok(Json(load_stats(&state, &owner).await?))
}

async fn score_photo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ScoreRequest>,
) -> Result<Json<ScoreResponse>, ApiError> {
    verify_challenge_key(&headers)?;
    let owner = challenge_owner(&state).await?;
    let image = decode_image(&request.image_data_url, MAX_IMAGE_BYTES, "照片")?;
    let thumbnail = decode_image(&request.thumbnail_data_url, MAX_THUMBNAIL_BYTES, "缩略图")?;
    let image_hash = hex::encode(Sha256::digest(&image.bytes));

    let current = state.store.list_entities(&owner, ENTITY_TYPE).await?;
    if let Some(existing) = current.iter().find_map(|snapshot| {
        let payload = &snapshot.payload.0;
        (payload.get("imageHash").and_then(Value::as_str) == Some(image_hash.as_str()))
            .then(|| response_from_payload(payload, true))
            .flatten()
    }) {
        let stats = stats_from_snapshots(&current);
        return Ok(Json(ScoreResponse { stats, ..existing }));
    }

    let model_score = call_glm_score(&image.base64).await?;
    let breakdown = ScoreBreakdown {
        composition: model_score.composition.clamp(0, 25),
        light_color: model_score.light_color.clamp(0, 20),
        subject_story: model_score.subject_story.clamp(0, 20),
        technical: model_score.technical.clamp(0, 20),
        originality: model_score.originality.clamp(0, 15),
    };
    // Recompute the final score from the rubric so a malformed model total can
    // never accidentally count as a qualifying photo.
    let score = breakdown.composition
        + breakdown.light_color
        + breakdown.subject_story
        + breakdown.technical
        + breakdown.originality;
    let qualified = score > HIGH_SCORE_THRESHOLD;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let thumbnail_data_url = format!("data:{};base64,{}", thumbnail.mime, thumbnail.base64);
    let payload = json!({
        "meta": {
            "id": id,
            "userId": owner.as_str(),
            "createdAt": now,
            "updatedAt": now,
            "localVersion": 1,
            "serverVersion": Value::Null,
            "modifiedByDevice": DEVICE_ID,
            "deletedAt": Value::Null
        },
        "score": score,
        "qualified": qualified,
        "threshold": HIGH_SCORE_THRESHOLD,
        "imageHash": image_hash,
        "fileName": clean_file_name(request.file_name.as_deref()),
        "capturedAt": request.captured_at,
        "scoredAt": now,
        "model": env_non_empty("PHOTO_CHALLENGE_MODEL").unwrap_or_else(|| "glm-4v-flash".to_owned()),
        "breakdown": breakdown,
        "feedback": model_score.feedback.trim().chars().take(500).collect::<String>(),
        "thumbnailDataUrl": thumbnail_data_url,
    });

    persist_entry(&state, &owner, &id, payload.clone()).await?;
    let stats = load_stats(&state, &owner).await?;
    let response = response_from_payload(&payload, false).ok_or_else(|| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "评分记录生成失败",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    Ok(Json(ScoreResponse { stats, ..response }))
}

struct DecodedImage {
    mime: String,
    base64: String,
    bytes: Vec<u8>,
}

fn decode_image(data_url: &str, max_bytes: usize, label: &str) -> Result<DecodedImage, ApiError> {
    let (prefix, encoded) = data_url.split_once(',').ok_or_else(|| {
        bad_request(format!("{label}格式无效"))
    })?;
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
        bytes,
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
            ErrorCode::InvalidRequest,
            "摄影挑战口令无效",
            StatusCode::UNAUTHORIZED,
        ));
    }
    Ok(())
}

async fn challenge_owner(state: &AppState) -> Result<UserId, ApiError> {
    if !state.database_enabled {
        return Ok(UserId::new(state.config.dev_auth_user_id.clone()));
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
    .map_err(|error| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            format!("查询摄影挑战所属账号失败: {error}"),
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?
    .ok_or_else(|| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "摄影挑战所属账号不存在",
            StatusCode::NOT_FOUND,
        )
    })?;
    Ok(UserId::new(id.to_string()))
}

async fn persist_entry(
    state: &AppState,
    owner: &UserId,
    id: &str,
    payload: Value,
) -> Result<(), ApiError> {
    let change = SyncChangeV1 {
        change_id: ChangeId::new(Uuid::new_v4().to_string()),
        entity_type: lifetrace_contracts::EntityType::new(ENTITY_TYPE),
        entity_id: EntityId::new(id),
        operation: ChangeOperation::new(ChangeOperation::UPSERT),
        base_server_version: lifetrace_contracts::ServerVersion::zero(),
        entity_schema_version: 1,
        client_modified_at: Utc::now(),
        payload: Some(JsonValue(payload)),
        atomic_group_id: None,
        dependencies: vec![],
    };
    let request = PushRequestV1 {
        request_id: RequestId::new(Uuid::new_v4().to_string()),
        client: SyncClientInfo {
            app_id: AppId::new("photo-challenge-pwa"),
            client_version: "1.0.0".to_owned(),
            platform: ClientPlatform::new(ClientPlatform::WEB),
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
            device_id: DeviceId::new(DEVICE_ID),
        },
        changes: vec![change],
    };
    let result = state.store.push(owner, &request).await?;
    match result.results.first() {
        Some(PushChangeResultV1::Accepted { .. }) => Ok(()),
        Some(PushChangeResultV1::Duplicate { .. }) => Ok(()),
        Some(PushChangeResultV1::Conflict { reason, .. }) => Err(ApiError::new(
            ErrorCode::BaseVersionMismatch,
            format!("保存评分记录冲突: {reason}"),
            StatusCode::CONFLICT,
        )),
        Some(PushChangeResultV1::Rejected { code, message, .. }) => Err(ApiError::new(
            code.clone(),
            message,
            StatusCode::BAD_REQUEST,
        )),
        None => Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "云端未返回评分记录保存结果",
            StatusCode::SERVICE_UNAVAILABLE,
        )),
    }
}

async fn load_stats(state: &AppState, owner: &UserId) -> Result<ChallengeStats, ApiError> {
    let snapshots = state.store.list_entities(owner, ENTITY_TYPE).await?;
    Ok(stats_from_snapshots(&snapshots))
}

fn stats_from_snapshots(snapshots: &[crate::repository::EntitySnapshot]) -> ChallengeStats {
    let scores: Vec<i64> = snapshots
        .iter()
        .filter_map(|snapshot| snapshot.payload.0.get("score").and_then(Value::as_i64))
        .collect();
    let high_score_count = scores
        .iter()
        .filter(|score| **score > HIGH_SCORE_THRESHOLD)
        .count();
    let average_score = if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<i64>() as f64 / scores.len() as f64
    };
    ChallengeStats {
        total: scores.len(),
        high_score_count,
        remaining: REQUIRED_HIGH_SCORE_COUNT.saturating_sub(high_score_count),
        target: REQUIRED_HIGH_SCORE_COUNT,
        achieved: high_score_count >= REQUIRED_HIGH_SCORE_COUNT,
        average_score: (average_score * 10.0).round() / 10.0,
    }
}

fn response_from_payload(payload: &Value, duplicate: bool) -> Option<ScoreResponse> {
    let breakdown = serde_json::from_value(payload.get("breakdown")?.clone()).ok()?;
    let score = payload.get("score")?.as_i64()?;
    Some(ScoreResponse {
        id: payload.get("meta")?.get("id")?.as_str()?.to_owned(),
        score,
        qualified: score > HIGH_SCORE_THRESHOLD,
        breakdown,
        feedback: payload
            .get("feedback")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
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

async fn call_glm_score(image_base64: &str) -> Result<ModelScore, ApiError> {
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
    let model = env_non_empty("PHOTO_CHALLENGE_MODEL")
        .unwrap_or_else(|| "glm-4v-flash".to_owned());
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let rubric = r#"你是一名严格、稳定的摄影比赛评委。请只评价照片本身，不因鼓励用户而抬高分数。按以下固定量表打分，总分100：构图 composition 0-25；光线与色彩 lightColor 0-20；主体与叙事 subjectStory 0-20；对焦/曝光/噪点等技术质量 technical 0-20；原创性与瞬间感 originality 0-15。90分代表已经达到非常优秀、少见的作品水平，普通好看的照片应明显低于90。只输出一个JSON对象，不要Markdown，不要额外文字，字段必须为：score, composition, lightColor, subjectStory, technical, originality, feedback。feedback用中文，最多80字，指出最关键优点和一个最值得改进的点。"#;
    let request = json!({
        "model": model,
        "temperature": 0.1,
        "stream": false,
        "max_tokens": 600,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "image_url", "image_url": {"url": image_base64}},
                {"type": "text", "text": rubric}
            ]
        }]
    });

    let provider = tokio::time::timeout(
        Duration::from_secs(55),
        call_provider(&endpoint, &api_key, &request),
    )
    .await
    .map_err(|_| ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "GLM-4V-Flash 评分超时",
        StatusCode::GATEWAY_TIMEOUT,
    ))?
    .map_err(|message| ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        format!("GLM-4V-Flash 评分失败: {message}"),
        StatusCode::BAD_GATEWAY,
    ))?;
    let content = provider
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "GLM-4V-Flash 未返回评分内容",
            StatusCode::BAD_GATEWAY,
        ))?;
    parse_model_score(&content).ok_or_else(|| ApiError::new(
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
    let mut stdin = child.stdin.take().ok_or_else(|| "无法打开评分请求输入流".to_owned())?;
    stdin.write_all(&payload).await.map_err(|error| error.to_string())?;
    drop(stdin);
    let output = child.wait_with_output().await.map_err(|error| error.to_string())?;
    if !output.status.success() {
        let body = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if !body.is_empty() { body } else if !error.is_empty() { error } else { format!("HTTP 调用失败: {}", output.status) });
    }
    serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())
}

fn parse_model_score(content: &str) -> Option<ModelScore> {
    if let Ok(parsed) = serde_json::from_str::<ModelScore>(content.trim()) {
        return Some(parsed);
    }
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    serde_json::from_str::<ModelScore>(&content[start..=end]).ok()
}

fn clean_file_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(120).collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_total_is_derived_from_rubric_parts() {
        let content = r#"{"score":99,"composition":23,"lightColor":18,"subjectStory":17,"technical":18,"originality":13,"feedback":"不错"}"#;
        let parsed = parse_model_score(content).expect("score");
        assert_eq!(parsed.composition + parsed.light_color + parsed.subject_story + parsed.technical + parsed.originality, 89);
    }

    #[test]
    fn model_score_parser_accepts_fenced_noise() {
        let content = "```json\n{\"score\":91,\"composition\":23,\"lightColor\":18,\"subjectStory\":18,\"technical\":18,\"originality\":14,\"feedback\":\"ok\"}\n```";
        assert_eq!(parse_model_score(content).expect("score").score, 91);
    }
}
