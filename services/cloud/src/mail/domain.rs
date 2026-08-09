use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MailProvider {
    Qq,
    #[serde(rename = "163")]
    NetEase163,
    #[serde(rename = "126")]
    NetEase126,
    Yeah,
    Generic,
}

impl MailProvider {
    pub fn as_db(&self) -> &'static str {
        match self {
            Self::Qq => "qq",
            Self::NetEase163 => "163",
            Self::NetEase126 => "126",
            Self::Yeah => "yeah",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MailSecurity {
    Tls,
    Starttls,
}

impl MailSecurity {
    pub fn as_db(&self) -> &'static str {
        match self {
            Self::Tls => "tls",
            Self::Starttls => "starttls",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccountInput {
    pub provider: MailProvider,
    pub email_address: String,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub authorization_code: String,
    pub imap_host: Option<String>,
    pub imap_port: Option<u16>,
    pub imap_security: Option<MailSecurity>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_security: Option<MailSecurity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MailAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub email_address: String,
    pub display_name: Option<String>,
    pub imap_host: String,
    pub imap_port: i32,
    pub imap_security: String,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_security: String,
    pub username: String,
    pub status: String,
    pub idle_supported: bool,
    pub last_validated_at: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MailAccountSecret {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub email_address: String,
    pub display_name: Option<String>,
    pub imap_host: String,
    pub imap_port: i32,
    pub imap_security: String,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_security: String,
    pub username: String,
    pub credential_ciphertext: Vec<u8>,
    pub credential_nonce: Vec<u8>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MailFolder {
    pub id: Uuid,
    pub account_id: Uuid,
    pub remote_name: String,
    pub normalized_role: String,
    pub uidvalidity: Option<i64>,
    pub uidnext: Option<i64>,
    pub last_seen_uid: i64,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MailThread {
    pub id: Uuid,
    pub account_id: Uuid,
    pub normalized_subject: String,
    pub latest_message_at: Option<DateTime<Utc>>,
    pub message_count: i32,
    pub unread_count: i32,
    pub participant_summary: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MailMessage {
    pub id: Uuid,
    pub account_id: Uuid,
    pub folder_id: Uuid,
    pub thread_id: Uuid,
    pub remote_uid: i64,
    pub uidvalidity: i64,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub subject: String,
    pub from_json: Value,
    pub to_json: Value,
    pub cc_json: Value,
    pub reply_to_json: Value,
    pub sent_at: Option<DateTime<Utc>>,
    pub received_at: DateTime<Utc>,
    pub flags_json: Value,
    pub is_read: bool,
    pub is_archived: bool,
    pub size_bytes: Option<i64>,
    pub snippet: Option<String>,
    pub body_text: Option<String>,
    pub body_html_sanitized: Option<String>,
    pub has_attachments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MailAttachment {
    pub id: Uuid,
    pub message_id: Uuid,
    pub part_id: String,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub content_id: Option<String>,
    pub disposition: Option<String>,
    pub checksum: Option<String>,
    pub storage_ref: Option<String>,
    pub download_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub imap_ok: bool,
    pub smtp_ok: bool,
    pub idle_supported: bool,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailListQuery {
    pub account_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub q: Option<String>,
    pub unread_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMailInput {
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    #[serde(default)]
    pub bcc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub in_reply_to_message_id: Option<Uuid>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone)]
pub struct ProviderPreset {
    pub imap_host: &'static str,
    pub imap_port: u16,
    pub imap_security: MailSecurity,
    pub smtp_host: &'static str,
    pub smtp_port: u16,
    pub smtp_security: MailSecurity,
}

pub fn provider_preset(provider: &MailProvider) -> Option<ProviderPreset> {
    match provider {
        MailProvider::Qq => Some(ProviderPreset {
            imap_host: "imap.qq.com",
            imap_port: 993,
            imap_security: MailSecurity::Tls,
            smtp_host: "smtp.qq.com",
            smtp_port: 465,
            smtp_security: MailSecurity::Tls,
        }),
        MailProvider::NetEase163 => Some(ProviderPreset {
            imap_host: "imap.163.com",
            imap_port: 993,
            imap_security: MailSecurity::Tls,
            smtp_host: "smtp.163.com",
            smtp_port: 465,
            smtp_security: MailSecurity::Tls,
        }),
        MailProvider::NetEase126 => Some(ProviderPreset {
            imap_host: "imap.126.com",
            imap_port: 993,
            imap_security: MailSecurity::Tls,
            smtp_host: "smtp.126.com",
            smtp_port: 465,
            smtp_security: MailSecurity::Tls,
        }),
        MailProvider::Yeah => Some(ProviderPreset {
            imap_host: "imap.yeah.net",
            imap_port: 993,
            imap_security: MailSecurity::Tls,
            smtp_host: "smtp.yeah.net",
            smtp_port: 465,
            smtp_security: MailSecurity::Tls,
        }),
        MailProvider::Generic => None,
    }
}

pub fn normalize_subject(value: &str) -> String {
    let mut current = value.trim();
    loop {
        let lower = current.to_ascii_lowercase();
        let next = ["re:", "fw:", "fwd:"].iter().find_map(|prefix| {
            lower.strip_prefix(prefix).map(|_| current[prefix.len()..].trim())
        });
        match next {
            Some(value) => current = value,
            None => break,
        }
    }
    current.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_presets_use_tls_ports() {
        let qq = provider_preset(&MailProvider::Qq).expect("qq preset");
        assert_eq!(qq.imap_host, "imap.qq.com");
        assert_eq!(qq.imap_port, 993);
        assert_eq!(qq.smtp_port, 465);
    }

    #[test]
    fn subject_normalization_strips_reply_prefixes() {
        assert_eq!(normalize_subject(" Re: FWD:  Project Update "), "project update");
    }
}
