use lifetrace_contracts::UserId;
use mail_parser::MessageParser;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::credential::CredentialCipher;
use super::domain::MailAccountSecret;
use super::protocol;

const MAX_ATTACHMENT_BYTES: i64 = 25 * 1024 * 1024;
const MAX_RAW_MESSAGE_BYTES: i64 = 64 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum AttachmentReadError {
    #[error("mail storage requires PostgreSQL")]
    DatabaseRequired,
    #[error("invalid authenticated user id")]
    InvalidUser,
    #[error("mail attachment not found")]
    NotFound,
    #[error("mail attachment is too large")]
    TooLarge,
    #[error("mail attachment metadata is invalid")]
    InvalidPart,
    #[error("mail credential is unavailable")]
    Credential,
    #[error("mail provider operation failed")]
    Protocol,
    #[error("mail message parse failed")]
    Parse,
    #[error("mail database operation failed")]
    Database,
}

impl From<sqlx::Error> for AttachmentReadError {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}

#[derive(Debug)]
pub struct AttachmentContent {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub filename: String,
}

#[derive(Debug, sqlx::FromRow)]
struct AttachmentSource {
    part_id: String,
    filename: Option<String>,
    mime_type: Option<String>,
    size_bytes: i64,
    message_size_bytes: Option<i64>,
    remote_uid: i64,
    folder_name: String,
    account_id: Uuid,
}

#[derive(Clone)]
pub struct AttachmentReader {
    pool: PgPool,
    database_enabled: bool,
}

impl AttachmentReader {
    pub fn new(pool: PgPool, database_enabled: bool) -> Self {
        Self {
            pool,
            database_enabled,
        }
    }

    pub async fn read(
        &self,
        user_id: &UserId,
        attachment_id: Uuid,
    ) -> Result<AttachmentContent, AttachmentReadError> {
        if !self.database_enabled {
            return Err(AttachmentReadError::DatabaseRequired);
        }
        let user_id =
            Uuid::parse_str(user_id.as_str()).map_err(|_| AttachmentReadError::InvalidUser)?;
        let source = sqlx::query_as::<_, AttachmentSource>(
            r#"
            SELECT a.part_id,a.filename,a.mime_type,a.size_bytes,
                   m.size_bytes AS message_size_bytes,m.remote_uid,
                   f.remote_name AS folder_name,m.account_id
            FROM mail_attachments a
            JOIN mail_messages m ON m.id=a.message_id
            JOIN mail_folders f ON f.id=m.folder_id
            WHERE a.id=$1 AND m.user_id=$2
            "#,
        )
        .bind(attachment_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AttachmentReadError::NotFound)?;

        if source.size_bytes > MAX_ATTACHMENT_BYTES
            || source
                .message_size_bytes
                .is_some_and(|size| size > MAX_RAW_MESSAGE_BYTES)
        {
            return Err(AttachmentReadError::TooLarge);
        }
        let part_index: usize = source
            .part_id
            .parse()
            .map_err(|_| AttachmentReadError::InvalidPart)?;
        let account = self.account_secret(user_id, source.account_id).await?;
        let cipher = CredentialCipher::from_env().map_err(|_| AttachmentReadError::Credential)?;
        let secret = cipher
            .decrypt(&account.credential_ciphertext, &account.credential_nonce)
            .map_err(|_| AttachmentReadError::Credential)?;
        let raw = protocol::fetch_raw_message(
            account,
            secret,
            source.folder_name,
            u32::try_from(source.remote_uid).map_err(|_| AttachmentReadError::InvalidPart)?,
        )
        .await
        .map_err(|_| AttachmentReadError::Protocol)?;
        if raw.len() as i64 > MAX_RAW_MESSAGE_BYTES {
            return Err(AttachmentReadError::TooLarge);
        }
        let message = MessageParser::default()
            .parse(&raw)
            .ok_or(AttachmentReadError::Parse)?;
        let attachment = message
            .attachments()
            .nth(part_index)
            .ok_or(AttachmentReadError::NotFound)?;
        let bytes = attachment.contents().to_vec();
        if bytes.len() as i64 > MAX_ATTACHMENT_BYTES {
            return Err(AttachmentReadError::TooLarge);
        }
        Ok(AttachmentContent {
            bytes,
            mime_type: source
                .mime_type
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
            filename: safe_filename(source.filename.as_deref().unwrap_or("attachment")),
        })
    }

    async fn account_secret(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<MailAccountSecret, AttachmentReadError> {
        sqlx::query_as::<_, MailAccountSecret>(
            r#"
            SELECT id,user_id,provider,email_address,display_name,
                   imap_host,imap_port,imap_security,smtp_host,smtp_port,smtp_security,
                   username,credential_ciphertext,credential_nonce,status
            FROM mail_accounts
            WHERE user_id=$1 AND id=$2 AND deleted_at IS NULL AND status <> 'disabled'
            "#,
        )
        .bind(user_id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AttachmentReadError::NotFound)
    }
}

fn safe_filename(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | '\0' | '\r' | '\n' => '_',
            _ => ch,
        })
        .take(240)
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "attachment".to_owned()
    } else {
        cleaned.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_cannot_escape_download_directory_or_inject_headers() {
        assert_eq!(safe_filename("../../report\r\n.txt"), "_.._report__.txt");
        assert_eq!(safe_filename(".."), "attachment");
    }
}
