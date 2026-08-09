use ammonia::Builder;
use chrono::{DateTime, Utc};
use mail_parser::{MessageParser, MimeHeaders};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::domain::normalize_subject;

#[derive(Debug, Clone)]
pub struct ParsedAttachment {
    pub part_id: String,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub content_id: Option<String>,
    pub disposition: Option<String>,
    pub checksum: String,
}

#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub subject: String,
    pub normalized_subject: String,
    pub from_json: Value,
    pub to_json: Value,
    pub cc_json: Value,
    pub bcc_json: Value,
    pub reply_to_json: Value,
    pub sent_at: Option<DateTime<Utc>>,
    pub snippet: String,
    pub body_text: String,
    pub body_html_sanitized: Option<String>,
    pub content_hash: String,
    pub attachments: Vec<ParsedAttachment>,
}

fn address_json<T: serde::Serialize>(value: Option<&T>) -> Value {
    value
        .and_then(|address| serde_json::to_value(address).ok())
        .unwrap_or_else(|| json!([]))
}

pub fn sanitize_html(input: &str) -> String {
    // Images are removed server-side so opening an email cannot fire remote tracking pixels.
    // Links remain usable and Ammonia adds noopener/noreferrer to them.
    Builder::default().rm_tags(&["img"]).clean(input).to_string()
}

pub fn parse_message(raw: &[u8]) -> Result<ParsedMessage, &'static str> {
    let message = MessageParser::default().parse(raw).ok_or("invalid MIME message")?;
    let subject = message.subject().unwrap_or("").trim().to_owned();
    let body_text = message.body_text(0).map(|value| value.into_owned()).unwrap_or_default();
    let body_html_sanitized = message
        .body_html(0)
        .map(|value| sanitize_html(value.as_ref()))
        .filter(|value| !value.trim().is_empty());
    let snippet = message
        .body_preview(240)
        .unwrap_or_else(|| body_text.chars().take(240).collect::<String>().into())
        .into_owned();

    let sent_at = message.date().and_then(|date| {
        DateTime::parse_from_rfc3339(&date.to_rfc3339())
            .ok()
            .map(|value| value.with_timezone(&Utc))
    });

    let attachments = message
        .attachments()
        .enumerate()
        .map(|(index, part)| {
            let mime_type = part.content_type().map(|content_type| {
                match content_type.subtype() {
                    Some(subtype) => format!("{}/{}", content_type.ctype(), subtype),
                    None => content_type.ctype().to_owned(),
                }
            });
            let disposition = part.content_disposition().map(|value| value.ctype().to_owned());
            let checksum = hex::encode(Sha256::digest(part.contents()));
            ParsedAttachment {
                part_id: index.to_string(),
                filename: part.attachment_name().map(str::to_owned),
                mime_type,
                size_bytes: part.len() as i64,
                content_id: part.content_id().map(str::to_owned),
                disposition,
                checksum,
            }
        })
        .collect();

    Ok(ParsedMessage {
        message_id: message.message_id().map(str::to_owned),
        in_reply_to: message.in_reply_to().map(str::to_owned),
        normalized_subject: normalize_subject(message.thread_name().unwrap_or(&subject)),
        subject,
        from_json: address_json(message.from()),
        to_json: address_json(message.to()),
        cc_json: address_json(message.cc()),
        bcc_json: address_json(message.bcc()),
        reply_to_json: address_json(message.reply_to()),
        sent_at,
        snippet,
        body_text,
        body_html_sanitized,
        content_hash: hex::encode(Sha256::digest(raw)),
        attachments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_removes_script_handlers_and_remote_images() {
        let cleaned = sanitize_html(
            r#"<p onclick="alert(1)">hello</p><script>alert(2)</script><img src="https://tracker.invalid/pixel"><a href="https://example.com">open</a>"#,
        );
        assert!(!cleaned.contains("script"));
        assert!(!cleaned.contains("onclick"));
        assert!(!cleaned.contains("tracker.invalid"));
        assert!(cleaned.contains("example.com"));
    }

    #[test]
    fn parses_realistic_multipart_message() {
        let raw = b"From: Alice <alice@example.com>\r\nTo: Bob <bob@example.com>\r\nMessage-ID: <m1@example.com>\r\nSubject: Re: Project Update\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nStatus is green.";
        let parsed = parse_message(raw).expect("parse");
        assert_eq!(parsed.message_id.as_deref(), Some("m1@example.com"));
        assert_eq!(parsed.normalized_subject, "project update");
        assert!(parsed.body_text.contains("Status is green"));
    }
}
