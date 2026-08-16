//! Authenticated AI assistant for browser and native clients.
//!
//! The provider key is read only by the cloud process. Browser clients use a
//! CSRF-protected HttpOnly session while native clients use their Bearer session;
//! both entry points share the exact same assistant execution logic.

use std::process::Stdio;
use std::time::Duration;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::auth::security::cookie_value;
use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

const MAX_PROMPT_CHARS: usize = 4_000;
const MAX_CONTEXT_BYTES: usize = 250_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantRequest {
    prompt: String,
    #[serde(default)]
    context: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssistantResponse {
    reply: String,
    provider: &'static str,
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
        .route("/api/v1/web/assistant", post(web_assistant))
        .route("/api/v1/assistant", post(native_assistant))
}

async fn web_assistant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AssistantRequest>,
) -> Result<Json<AssistantResponse>, ApiError> {
    let raw_session = cookie_value(&headers, &state.config.auth_cookie_name).unwrap_or_default();
    let csrf = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let origin = headers.get("origin").and_then(|value| value.to_str().ok());
    state
        .auth_service
        .verify_web_csrf(&raw_session, csrf, origin)
        .await?;
    run_assistant(request).await
}

async fn native_assistant(
    _principal: AuthenticatedPrincipal,
    Json(request): Json<AssistantRequest>,
) -> Result<Json<AssistantResponse>, ApiError> {
    run_assistant(request).await
}

async fn run_assistant(
    request: AssistantRequest,
) -> Result<Json<AssistantResponse>, ApiError> {
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Ok(Json(AssistantResponse {
            reply: "请先输入一个具体问题。".to_owned(),
            provider: "local",
        }));
    }
    let prompt: String = prompt.chars().take(MAX_PROMPT_CHARS).collect();
    let context = compact_context(request.context);
    let fallback = local_reply(&prompt, &context);

    let Some(api_key) = env_non_empty("DEEPSEEK_API_KEY") else {
        return Ok(Json(AssistantResponse {
            reply: fallback,
            provider: "local",
        }));
    };
    if contains_newline(&api_key) {
        return Ok(Json(AssistantResponse {
            reply: fallback,
            provider: "local",
        }));
    }

    let base_url =
        env_non_empty("DEEPSEEK_BASE_URL").unwrap_or_else(|| "https://api.deepseek.com".to_owned());
    let model = env_non_empty("DEEPSEEK_MODEL").unwrap_or_else(|| "deepseek-chat".to_owned());
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let context_text = serde_json::to_string(&context).unwrap_or_else(|_| "{}".to_owned());
    let provider_request = json!({
        "model": model,
        "temperature": 0.35,
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": "你是 LifeTrace 个人管理平台中的 AI 管家。只根据用户提供的记录进行分析，不编造不存在的数据。回答使用中文，先给结论，再给最多三条具体建议。涉及健康、金融或安全时明确说明局限，不替代专业意见。"
            },
            {
                "role": "user",
                "content": format!("用户问题：\n{}\n\n近期 LifeTrace 记录（JSON）：\n{}", prompt, context_text)
            }
        ]
    });

    let provider = tokio::time::timeout(
        Duration::from_secs(50),
        call_provider(&endpoint, &api_key, &provider_request),
    )
    .await;
    let parsed = match provider {
        Ok(Ok(value)) => Some(value),
        Ok(Err(_)) | Err(_) => None,
    };
    let reply = parsed
        .and_then(|value| value.choices.into_iter().next())
        .map(|choice| choice.message.content.trim().to_owned())
        .filter(|value| !value.is_empty());

    Ok(Json(match reply {
        Some(reply) => AssistantResponse {
            reply,
            provider: "deepseek",
        },
        None => AssistantResponse {
            reply: format!("AI 服务暂时不可用。\n\n{fallback}"),
            provider: "local",
        },
    }))
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
        .arg("45")
        .arg("--request")
        .arg("POST")
        .arg("--url")
        .arg(endpoint)
        .arg("--header")
        .arg(format!("Authorization: Bearer {api_key}"))
        .arg("--header")
        .arg("Content-Type: application/json")
        .arg("--header")
        .arg("Accept: application/json")
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
        .ok_or_else(|| "无法打开 AI 请求输入流".to_owned())?;
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
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if message.is_empty() {
            format!("AI 服务返回状态 {}", output.status)
        } else {
            message
        });
    }
    serde_json::from_slice::<ProviderResponse>(&output.stdout).map_err(|error| error.to_string())
}

fn compact_context(value: Value) -> Value {
    let encoded = serde_json::to_vec(&value).unwrap_or_default();
    if encoded.len() <= MAX_CONTEXT_BYTES {
        value
    } else {
        json!({
            "truncated": true,
            "summary": context_counts(&value),
            "message": "The detailed context exceeded the server safety limit."
        })
    }
}

fn context_counts(context: &Value) -> Value {
    let mut counts = serde_json::Map::new();
    if let Some(object) = context.as_object() {
        for (key, value) in object {
            if let Some(items) = value.as_array() {
                counts.insert(key.clone(), json!(items.len()));
            }
        }
    }
    Value::Object(counts)
}

fn count(context: &Value, key: &str) -> usize {
    context
        .get(key)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn local_reply(prompt: &str, context: &Value) -> String {
    let habits = count(context, "habit.log");
    let activities = count(context, "habit.activity");
    let workouts = count(context, "workout.workout");
    let transactions = count(context, "finance.transaction");
    let notes = count(context, "note.note");
    let readings = count(context, "english.learning_record");
    let reviews = count(context, "review.daily");

    let focus = if prompt.contains("消费") || prompt.contains("财务") || prompt.contains("账单")
    {
        format!("当前上下文包含 {transactions} 笔近期账单。建议先按支出类别和月份核对大额、重复及候选流水，再结合预算判断是否需要调整。")
    } else if prompt.contains("训练") || prompt.contains("健身") {
        format!("当前上下文包含 {workouts} 次近期训练。建议对照训练频率、时长和容量变化，并结合复盘中的精力状态安排下一周负荷。")
    } else if prompt.contains("坚持") || prompt.contains("习惯") {
        format!("当前有 {activities} 个坚持项目和 {habits} 条近期记录。建议优先保证最重要项目的最低完成量，并检查连续两次缺失的项目。")
    } else if prompt.contains("英语") || prompt.contains("阅读") {
        format!("当前上下文包含 {readings} 条英语学习记录。建议继续保持阅读总结，并把反复遇到的词汇放入复习计划。")
    } else {
        format!("当前上下文包含：{activities} 个坚持项目、{habits} 条坚持记录、{workouts} 次训练、{transactions} 笔账单、{notes} 篇笔记、{readings} 条英语记录和 {reviews} 条复盘。")
    };
    format!("{focus}\n\n1. 先处理今天最重要且可在 20 分钟内开始的一项。\n2. 用每日复盘记录精力和阻碍，避免只看完成数量。\n3. 连续观察一周后再调整目标，不根据单日波动下结论。")
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn contains_newline(value: &str) -> bool {
    value.contains('\r') || value.contains('\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_reply_uses_available_domain_counts() {
        let context = json!({
            "habit.activity": [{"name":"练琴"}],
            "habit.log": [{}, {}],
            "workout.workout": [{}],
            "finance.transaction": [{}, {}, {}]
        });
        let reply = local_reply("总结我的状态", &context);
        assert!(reply.contains("1 个坚持项目"));
        assert!(reply.contains("2 条坚持记录"));
        assert!(reply.contains("3 笔账单"));
    }

    #[test]
    fn oversized_context_is_replaced_with_counts() {
        let context = json!({"note.note": ["x".repeat(MAX_CONTEXT_BYTES + 1)]});
        let compact = compact_context(context);
        assert_eq!(compact["truncated"], true);
        assert_eq!(compact["summary"]["note.note"], 1);
    }

    #[test]
    fn provider_key_rejects_header_injection() {
        assert!(!contains_newline("safe-key"));
        assert!(contains_newline("unsafe\nkey"));
    }
}
