//! Command-style finance screenshot capture endpoint.
//!
//! Images are accepted only for the duration of the request. The raw image and
//! provider response are never persisted by this module.

use std::env;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use lifetrace_contracts::domain::enums::TransactionType;
use lifetrace_contracts::domain::finance::{FinanceCaptureBill, FinanceCaptureResponse};
use lifetrace_contracts::money::CurrencyCode;
use lifetrace_contracts::ErrorCode;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

const DEFAULT_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";
const DEFAULT_MODEL: &str = "glm-4v-flash";
const DEFAULT_MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const MULTIPART_BODY_LIMIT: usize = 11 * 1024 * 1024;
const PROVIDER_TIMEOUT_SECONDS: u64 = 45;

#[derive(Debug, Clone)]
struct VisionSettings {
    base_url: String,
    api_key: Option<String>,
    model: String,
    max_image_bytes: usize,
}

impl VisionSettings {
    fn from_env() -> Self {
        Self {
            base_url: env::var("LIFETRACE_VISION_BASE_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned()),
            api_key: env::var("LIFETRACE_VISION_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            model: env::var("LIFETRACE_VISION_MODEL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_MODEL.to_owned()),
            max_image_bytes: env::var("LIFETRACE_VISION_MAX_IMAGE_BYTES")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(DEFAULT_MAX_IMAGE_BYTES),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[derive(Debug)]
struct CaptureInput {
    bytes: Vec<u8>,
    mime: String,
    current_time: Option<String>,
    timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCaptureBill {
    amount_cents: i64,
    #[serde(default = "default_currency")]
    currency: String,
    #[serde(rename = "type", alias = "transactionType")]
    transaction_type: String,
    merchant: Option<String>,
    item: Option<String>,
    occurred_at: Option<String>,
    account_hint: Option<String>,
    category_hint: Option<String>,
    external_transaction_id: Option<String>,
    confidence: Option<f64>,
}

fn default_currency() -> String {
    "CNY".to_owned()
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/finance/capture/image", post(capture_image))
        .layer(DefaultBodyLimit::max(MULTIPART_BODY_LIMIT))
}

async fn capture_image(
    State(_state): State<AppState>,
    principal: AuthenticatedPrincipal,
    multipart: Multipart,
) -> Result<Json<FinanceCaptureResponse>, ApiError> {
    principal.require_scope("finance:write")?;
    let settings = VisionSettings::from_env();
    let api_key = settings.api_key.as_deref().ok_or_else(|| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "finance image capture is not configured",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;

    let input = read_capture_input(multipart, settings.max_image_bytes).await?;
    let provider_text = call_zhipu(&settings, api_key, &input).await?;
    let bills = parse_capture_bills(&provider_text)?;

    Ok(Json(FinanceCaptureResponse {
        provider: "zhipu".to_owned(),
        model: settings.model,
        bills,
    }))
}

async fn read_capture_input(
    mut multipart: Multipart,
    max_image_bytes: usize,
) -> Result<CaptureInput, ApiError> {
    let mut image: Option<(Vec<u8>, Option<String>)> = None;
    let mut current_time = None;
    let mut timezone = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "invalid multipart request",
            StatusCode::BAD_REQUEST,
        )
    })? {
        let name = field.name().unwrap_or_default().to_owned();
        if name == "image" {
            if image.is_some() {
                return Err(ApiError::new(
                    ErrorCode::InvalidRequest,
                    "exactly one image is supported",
                    StatusCode::BAD_REQUEST,
                ));
            }
            let declared_mime = field.content_type().map(str::to_owned);
            let bytes = field.bytes().await.map_err(|_| {
                ApiError::new(
                    ErrorCode::InvalidRequest,
                    "unable to read image field",
                    StatusCode::BAD_REQUEST,
                )
            })?;
            if bytes.is_empty() {
                return Err(ApiError::new(
                    ErrorCode::InvalidRequest,
                    "image is empty",
                    StatusCode::BAD_REQUEST,
                ));
            }
            if bytes.len() > max_image_bytes {
                return Err(ApiError::new(
                    ErrorCode::PayloadTooLarge,
                    "image exceeds the configured size limit",
                    StatusCode::PAYLOAD_TOO_LARGE,
                ));
            }
            image = Some((bytes.to_vec(), declared_mime));
        } else if name == "currentTime" {
            current_time = field.text().await.ok().filter(|value| !value.trim().is_empty());
        } else if name == "timezone" {
            timezone = field.text().await.ok().filter(|value| !value.trim().is_empty());
        }
    }

    let (bytes, declared_mime) = image.ok_or_else(|| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "multipart field 'image' is required",
            StatusCode::BAD_REQUEST,
        )
    })?;
    let detected_mime = detect_image_mime(&bytes).ok_or_else(|| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "only PNG, JPEG and WebP images are supported",
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        )
    })?;

    if let Some(declared) = declared_mime {
        let normalized = match declared.as_str() {
            "image/jpg" => "image/jpeg",
            other => other,
        };
        if !matches!(normalized, "image/png" | "image/jpeg" | "image/webp")
            || normalized != detected_mime
        {
            return Err(ApiError::new(
                ErrorCode::InvalidRequest,
                "declared image type does not match file contents",
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ));
        }
    }

    Ok(CaptureInput {
        bytes,
        mime: detected_mime.to_owned(),
        current_time,
        timezone,
    })
}

fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

async fn call_zhipu(
    settings: &VisionSettings,
    api_key: &str,
    input: &CaptureInput,
) -> Result<String, ApiError> {
    let image = BASE64.encode(&input.bytes);
    let prompt = capture_prompt(input.current_time.as_deref(), input.timezone.as_deref());
    let body = json!({
        "model": settings.model,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": { "url": format!("data:{};base64,{}", input.mime, image) }
                },
                { "type": "text", "text": prompt }
            ]
        }],
        "temperature": 0.1,
        "stream": false
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(PROVIDER_TIMEOUT_SECONDS))
        .build()
        .map_err(|_| provider_unavailable("unable to initialize vision provider"))?;
    let response = client
        .post(settings.endpoint())
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|_| provider_unavailable("vision provider request failed"))?;

    if !response.status().is_success() {
        return Err(provider_unavailable(format!(
            "vision provider returned HTTP {}",
            response.status().as_u16()
        )));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|_| provider_unavailable("vision provider returned invalid JSON"))?;
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| provider_unavailable("vision provider response did not contain content"))
}

fn provider_unavailable(message: impl Into<String>) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        message,
        StatusCode::BAD_GATEWAY,
    )
}

fn capture_prompt(current_time: Option<&str>, timezone: Option<&str>) -> String {
    let time_context = current_time.unwrap_or("unknown");
    let zone_context = timezone.unwrap_or("unknown");
    format!(
        r#"You are a payment screenshot parser. Analyze the single image and return data, not an explanation.

First decide whether the image contains real financial transaction evidence such as a payment result, transaction detail, receipt, bank transaction, or bill list. Desktop screenshots, chats, social posts, articles, normal app pages, photos, settings pages, product pages without completed payment evidence, and unrelated screenshots are NOT bills. If it is not a bill, return [].

A screenshot may contain multiple independent completed transactions. Return one object per transaction and do not double-count original price, discount, coupon, fee breakdown, or subtotal as separate transactions.

Current client time: {time_context}
Client timezone: {zone_context}

Return ONLY a JSON array. No markdown, prose, or code fence. Each item must use exactly these camelCase fields:
- amountCents: positive integer in the smallest currency unit. For CNY 28.50 yuan => 2850. Use the final paid/received amount, not original price or discount.
- currency: ISO 4217 uppercase code, normally CNY.
- type: one of expense, income, transfer, refund, fee.
- merchant: merchant/payee/payer name or null.
- item: short product/service description or null.
- occurredAt: RFC3339 timestamp with timezone if visible or reliably inferable, otherwise null.
- accountHint: payment account/bank/wallet text visible in the image or null.
- categoryHint: short semantic category such as 餐饮/交通/购物/医疗/工资/转账, or null.
- externalTransactionId: transaction/order identifier only when clearly visible, otherwise null.
- confidence: number from 0 to 1 for the extracted transaction.

Examples:
[]
[{"amountCents":2850,"currency":"CNY","type":"expense","merchant":"瑞幸咖啡","item":null,"occurredAt":"2026-08-12T09:30:00+08:00","accountHint":"招商银行储蓄卡","categoryHint":"餐饮","externalTransactionId":null,"confidence":0.95}]"#
    )
}

fn parse_capture_bills(content: &str) -> Result<Vec<FinanceCaptureBill>, ApiError> {
    let block = first_balanced_array(content).ok_or_else(|| {
        provider_unavailable("vision provider did not return a JSON array")
    })?;
    let cleaned = cleanup_trailing_commas(block);
    let raw: Vec<RawCaptureBill> = serde_json::from_str(&cleaned)
        .map_err(|_| provider_unavailable("vision provider returned malformed bill JSON"))?;

    let mut bills = Vec::with_capacity(raw.len());
    for item in raw {
        if item.amount_cents <= 0 {
            continue;
        }
        let transaction_type = match item.transaction_type.as_str() {
            TransactionType::EXPENSE | TransactionType::INCOME | TransactionType::TRANSFER
            | TransactionType::REFUND | TransactionType::FEE => {
                TransactionType::new(item.transaction_type)
            }
            _ => continue,
        };
        let currency = CurrencyCode::new(item.currency).unwrap_or_else(|_| CurrencyCode::cny());
        let occurred_at = item
            .occurred_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc));
        bills.push(FinanceCaptureBill {
            amount_cents: item.amount_cents,
            currency,
            transaction_type,
            merchant: clean_optional(item.merchant, 120),
            item: clean_optional(item.item, 160),
            occurred_at,
            account_hint: clean_optional(item.account_hint, 120),
            category_hint: clean_optional(item.category_hint, 80),
            external_transaction_id: clean_optional(item.external_transaction_id, 160),
            confidence: item.confidence.filter(|value| value.is_finite()).map(|value| value.clamp(0.0, 1.0)),
        });
    }
    Ok(bills)
}

fn clean_optional(value: Option<String>, max_chars: usize) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(max_chars).collect())
        }
    })
}

fn first_balanced_array(input: &str) -> Option<&str> {
    let start = input.find('[')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in input[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '[' {
            depth += 1;
        } else if ch == ']' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(&input[start..start + offset + ch.len_utf8()]);
            }
        }
    }
    None
}

fn cleanup_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            in_string = true;
            output.push(ch);
            i += 1;
            continue;
        }
        if ch == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && matches!(chars[j], '}' | ']') {
                i += 1;
                continue;
            }
        }
        output.push(ch);
        i += 1;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_image_magic() {
        assert_eq!(detect_image_mime(&[0xFF, 0xD8, 0xFF, 0x00]), Some("image/jpeg"));
        assert_eq!(
            detect_image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Some("image/png")
        );
        assert_eq!(detect_image_mime(b"RIFF1234WEBPxxxx"), Some("image/webp"));
        assert_eq!(detect_image_mime(b"not-image"), None);
    }

    #[test]
    fn parses_markdown_wrapped_multi_bill_response_and_sanitizes() {
        let content = r#"result:\n```json\n[
          {"amountCents":2850,"currency":"CNY","type":"expense","merchant":" 瑞幸咖啡 ","occurredAt":"2026-08-12T09:30:00+08:00","confidence":1.2},
          {"amountCents":5000,"currency":"CNY","type":"income","merchant":"退款","occurredAt":null,"confidence":0.8},
        ]\n```"#;
        let bills = parse_capture_bills(content).unwrap();
        assert_eq!(bills.len(), 2);
        assert_eq!(bills[0].amount_cents, 2850);
        assert_eq!(bills[0].merchant.as_deref(), Some("瑞幸咖啡"));
        assert_eq!(bills[0].confidence, Some(1.0));
        assert!(bills[0].occurred_at.is_some());
        assert_eq!(bills[1].transaction_type.as_str(), TransactionType::INCOME);
    }

    #[test]
    fn empty_array_is_a_valid_non_bill_result() {
        let bills = parse_capture_bills("[]").unwrap();
        assert!(bills.is_empty());
    }

    #[test]
    fn invalid_amount_and_unknown_type_are_discarded() {
        let bills = parse_capture_bills(
            r#"[{"amountCents":0,"type":"expense"},{"amountCents":100,"type":"mystery"}]"#,
        )
        .unwrap();
        assert!(bills.is_empty());
    }
}
