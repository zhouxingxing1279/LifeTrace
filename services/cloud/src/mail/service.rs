use std::collections::BTreeSet;

use chrono::{Duration, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use lifetrace_contracts::UserId;

use super::credential::{CredentialCipher, CredentialError};
use super::domain::{
    provider_preset, ConnectionTestResult, MailAccount, MailAccountInput, MailAccountSecret,
    MailAttachment, MailFolder, MailListQuery, MailMessage, MailProvider, MailSecurity, MailThread,
    SendMailInput,
};
use super::parser::{parse_message, ParsedMessage};
use super::protocol::{self, MailProtocolError, RemoteFolderSnapshot};

#[derive(Debug, Error)]
pub enum MailServiceError {
    #[error("mail storage requires PostgreSQL")]
    DatabaseRequired,
    #[error("invalid authenticated user id")]
    InvalidUser,
    #[error("invalid mail account configuration")]
    InvalidAccount,
    #[error("mail account not found")]
    AccountNotFound,
    #[error("mail message not found")]
    MessageNotFound,
    #[error("mail thread not found")]
    ThreadNotFound,
    #[error("archive folder is unavailable")]
    ArchiveUnavailable,
    #[error("mail credential is unavailable")]
    Credential,
    #[error("mail protocol operation failed")]
    Protocol,
    #[error("mail database operation failed")]
    Database,
    #[error("mail message parse failed")]
    Parse,
    #[error("mail send is already in progress")]
    SendInProgress,
}

impl From<sqlx::Error> for MailServiceError {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}

impl From<CredentialError> for MailServiceError {
    fn from(_: CredentialError) -> Self {
        Self::Credential
    }
}

impl From<MailProtocolError> for MailServiceError {
    fn from(_: MailProtocolError) -> Self {
        Self::Protocol
    }
}

#[derive(Clone)]
pub struct MailService {
    pool: PgPool,
    database_enabled: bool,
}

impl MailService {
    pub fn new(pool: PgPool, database_enabled: bool) -> Self {
        Self {
            pool,
            database_enabled,
        }
    }

    fn require_database(&self) -> Result<(), MailServiceError> {
        if self.database_enabled {
            Ok(())
        } else {
            Err(MailServiceError::DatabaseRequired)
        }
    }

    fn user_uuid(user_id: &UserId) -> Result<Uuid, MailServiceError> {
        Uuid::parse_str(user_id.as_str()).map_err(|_| MailServiceError::InvalidUser)
    }

    fn resolve_input(input: &MailAccountInput) -> Result<ResolvedAccount, MailServiceError> {
        let email = input.email_address.trim().to_ascii_lowercase();
        if email.is_empty() || !email.contains('@') || input.authorization_code.is_empty() {
            return Err(MailServiceError::InvalidAccount);
        }
        let username = input
            .username
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&email)
            .to_owned();
        let preset = provider_preset(&input.provider);
        let imap_host = input
            .imap_host
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| preset.as_ref().map(|value| value.imap_host.to_owned()))
            .ok_or(MailServiceError::InvalidAccount)?;
        let smtp_host = input
            .smtp_host
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| preset.as_ref().map(|value| value.smtp_host.to_owned()))
            .ok_or(MailServiceError::InvalidAccount)?;
        let imap_port = input
            .imap_port
            .or_else(|| preset.as_ref().map(|value| value.imap_port))
            .ok_or(MailServiceError::InvalidAccount)?;
        let smtp_port = input
            .smtp_port
            .or_else(|| preset.as_ref().map(|value| value.smtp_port))
            .ok_or(MailServiceError::InvalidAccount)?;
        let imap_security = input
            .imap_security
            .clone()
            .or_else(|| preset.as_ref().map(|value| value.imap_security.clone()))
            .ok_or(MailServiceError::InvalidAccount)?;
        let smtp_security = input
            .smtp_security
            .clone()
            .or_else(|| preset.as_ref().map(|value| value.smtp_security.clone()))
            .ok_or(MailServiceError::InvalidAccount)?;
        Ok(ResolvedAccount {
            email,
            username,
            imap_host,
            imap_port,
            imap_security,
            smtp_host,
            smtp_port,
            smtp_security,
        })
    }

    pub async fn create_account(
        &self,
        user_id: &UserId,
        input: MailAccountInput,
    ) -> Result<MailAccount, MailServiceError> {
        self.require_database()?;
        let user_id = Self::user_uuid(user_id)?;
        let resolved = Self::resolve_input(&input)?;
        let cipher = CredentialCipher::from_env()?;
        let (credential_ciphertext, credential_nonce) =
            cipher.encrypt(&input.authorization_code)?;
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO mail_accounts (
                id, user_id, provider, email_address, display_name,
                imap_host, imap_port, imap_security,
                smtp_host, smtp_port, smtp_security, username,
                credential_ciphertext, credential_nonce, status
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'validating')
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(input.provider.as_db())
        .bind(&resolved.email)
        .bind(
            input
                .display_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(&resolved.imap_host)
        .bind(i32::from(resolved.imap_port))
        .bind(resolved.imap_security.as_db())
        .bind(&resolved.smtp_host)
        .bind(i32::from(resolved.smtp_port))
        .bind(resolved.smtp_security.as_db())
        .bind(&resolved.username)
        .bind(credential_ciphertext)
        .bind(credential_nonce)
        .execute(&self.pool)
        .await?;

        let _ = self.test_account_uuid(user_id, id).await;
        self.account_by_id(user_id, id).await
    }

    pub async fn list_accounts(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<MailAccount>, MailServiceError> {
        self.require_database()?;
        let user_id = Self::user_uuid(user_id)?;
        sqlx::query_as::<_, MailAccount>(ACCOUNT_SELECT_LIST)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn account_by_id(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<MailAccount, MailServiceError> {
        sqlx::query_as::<_, MailAccount>(ACCOUNT_SELECT_ONE)
            .bind(user_id)
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(MailServiceError::AccountNotFound)
    }

    async fn account_secret(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<MailAccountSecret, MailServiceError> {
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
        .ok_or(MailServiceError::AccountNotFound)
    }

    fn decrypt_secret(account: &MailAccountSecret) -> Result<String, MailServiceError> {
        Ok(CredentialCipher::from_env()?
            .decrypt(&account.credential_ciphertext, &account.credential_nonce)?)
    }

    pub async fn test_account(
        &self,
        user_id: &UserId,
        account_id: Uuid,
    ) -> Result<ConnectionTestResult, MailServiceError> {
        self.require_database()?;
        self.test_account_uuid(Self::user_uuid(user_id)?, account_id)
            .await
    }

    async fn test_account_uuid(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<ConnectionTestResult, MailServiceError> {
        let account = self.account_secret(user_id, account_id).await?;
        let secret = Self::decrypt_secret(&account)?;
        let imap_probe = protocol::probe_imap(account.clone(), secret.clone()).await;
        let smtp_probe = protocol::probe_smtp(&account, &secret).await;
        let imap_ok = imap_probe.is_ok();
        let smtp_ok = smtp_probe.is_ok();
        let (idle_supported, folders) = match imap_probe {
            Ok(value) => (value.idle_supported, value.folders),
            Err(_) => (false, Vec::new()),
        };
        let status = if imap_ok && smtp_ok {
            "active"
        } else if imap_ok {
            "degraded"
        } else {
            "validating"
        };
        let error_code = if imap_ok && smtp_ok {
            None
        } else if !imap_ok {
            Some("MAIL_IMAP_CONNECTION_FAILED")
        } else {
            Some("MAIL_SMTP_CONNECTION_FAILED")
        };
        sqlx::query(
            r#"
            UPDATE mail_accounts
            SET status=$3,idle_supported=$4,last_validated_at=now(),last_error_code=$5,updated_at=now()
            WHERE user_id=$1 AND id=$2
            "#,
        )
        .bind(user_id)
        .bind(account_id)
        .bind(status)
        .bind(idle_supported)
        .bind(error_code)
        .execute(&self.pool)
        .await?;
        if imap_ok {
            self.upsert_folders(user_id, account_id, &folders).await?;
        }
        Ok(ConnectionTestResult {
            imap_ok,
            smtp_ok,
            idle_supported,
            folders,
        })
    }

    pub async fn disconnect_account(
        &self,
        user_id: &UserId,
        account_id: Uuid,
    ) -> Result<(), MailServiceError> {
        self.require_database()?;
        let result = sqlx::query(
            r#"
            UPDATE mail_accounts
            SET status='disabled',credential_ciphertext='\\x'::bytea,credential_nonce='\\x'::bytea,
                deleted_at=now(),updated_at=now()
            WHERE user_id=$1 AND id=$2 AND deleted_at IS NULL
            "#,
        )
        .bind(Self::user_uuid(user_id)?)
        .bind(account_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(MailServiceError::AccountNotFound);
        }
        Ok(())
    }

    async fn upsert_folders(
        &self,
        user_id: Uuid,
        account_id: Uuid,
        folders: &[String],
    ) -> Result<(), MailServiceError> {
        for name in folders {
            let role = folder_role(name);
            let sync_enabled = !matches!(role, "trash" | "spam" | "drafts");
            sqlx::query(
                r#"
                INSERT INTO mail_folders (id,user_id,account_id,remote_name,normalized_role,sync_enabled)
                VALUES ($1,$2,$3,$4,$5,$6)
                ON CONFLICT (account_id,remote_name)
                DO UPDATE SET normalized_role=EXCLUDED.normalized_role,updated_at=now()
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(account_id)
            .bind(name)
            .bind(role)
            .bind(sync_enabled)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn list_folders(
        &self,
        user_id: &UserId,
        account_id: Uuid,
    ) -> Result<Vec<MailFolder>, MailServiceError> {
        self.require_database()?;
        sqlx::query_as::<_, MailFolder>(
            r#"
            SELECT id,account_id,remote_name,normalized_role,uidvalidity,uidnext,last_seen_uid,last_sync_at,sync_enabled
            FROM mail_folders WHERE user_id=$1 AND account_id=$2 ORDER BY remote_name
            "#,
        )
        .bind(Self::user_uuid(user_id)?)
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn sync_account(
        &self,
        user_id: &UserId,
        account_id: Uuid,
    ) -> Result<usize, MailServiceError> {
        self.require_database()?;
        self.sync_account_uuid(Self::user_uuid(user_id)?, account_id)
            .await
    }

    pub async fn sync_due_accounts(&self, limit: i64) -> Result<usize, MailServiceError> {
        self.require_database()?;
        let accounts = sqlx::query_as::<_, (Uuid, Uuid)>(
            r#"
            SELECT user_id,id FROM mail_accounts
            WHERE deleted_at IS NULL AND status IN ('active','degraded')
              AND (last_sync_at IS NULL OR last_sync_at < now() - interval '2 minutes')
            ORDER BY last_sync_at NULLS FIRST LIMIT $1
            "#,
        )
        .bind(limit.clamp(1, 100))
        .fetch_all(&self.pool)
        .await?;
        let mut synced = 0;
        for (user_id, account_id) in accounts {
            if self.sync_account_uuid(user_id, account_id).await.is_ok() {
                synced += 1;
            }
        }
        Ok(synced)
    }

    async fn sync_account_uuid(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<usize, MailServiceError> {
        let account = self.account_secret(user_id, account_id).await?;
        let secret = Self::decrypt_secret(&account)?;
        let folders = sqlx::query_as::<_, MailFolder>(
            r#"
            SELECT id,account_id,remote_name,normalized_role,uidvalidity,uidnext,last_seen_uid,last_sync_at,sync_enabled
            FROM mail_folders WHERE user_id=$1 AND account_id=$2 AND sync_enabled=TRUE ORDER BY remote_name
            "#,
        )
        .bind(user_id)
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;
        if folders.is_empty() {
            let probe = protocol::probe_imap(account.clone(), secret.clone()).await?;
            self.upsert_folders(user_id, account_id, &probe.folders)
                .await?;
            return Box::pin(self.sync_account_uuid(user_id, account_id)).await;
        }

        let mut total = 0;
        let initial_since = Utc::now() - Duration::days(30);
        for folder in folders {
            let job_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO mail_sync_jobs (id,account_id,folder_id,kind,state,started_at,created_at) VALUES ($1,$2,$3,$4,'running',now(),now())",
            )
            .bind(job_id)
            .bind(account_id)
            .bind(folder.id)
            .bind(if folder.last_sync_at.is_none() { "initial" } else { "incremental" })
            .execute(&self.pool)
            .await?;
            let snapshot = protocol::fetch_folder(
                account.clone(),
                secret.clone(),
                folder.remote_name.clone(),
                folder.uidvalidity,
                folder.last_seen_uid,
                folder.last_sync_at.is_none().then_some(initial_since),
            )
            .await;
            match snapshot {
                Ok(snapshot) => {
                    let count = self
                        .persist_folder_snapshot(user_id, &folder, snapshot)
                        .await?;
                    total += count;
                    sqlx::query(
                        "UPDATE mail_sync_jobs SET state='success',finished_at=now() WHERE id=$1",
                    )
                    .bind(job_id)
                    .execute(&self.pool)
                    .await?;
                }
                Err(_) => {
                    sqlx::query("UPDATE mail_sync_jobs SET state='retry_wait',attempt=attempt+1,finished_at=now(),next_retry_at=now()+interval '3 minutes',error_code='MAIL_SYNC_FAILED',error_detail_redacted='protocol operation failed' WHERE id=$1")
                        .bind(job_id)
                        .execute(&self.pool)
                        .await?;
                }
            }
        }
        sqlx::query("UPDATE mail_accounts SET last_sync_at=now(),updated_at=now() WHERE user_id=$1 AND id=$2")
            .bind(user_id)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(total)
    }

    async fn persist_folder_snapshot(
        &self,
        user_id: Uuid,
        folder: &MailFolder,
        snapshot: RemoteFolderSnapshot,
    ) -> Result<usize, MailServiceError> {
        let mut transaction = self.pool.begin().await?;
        let uidvalidity_changed = folder.uidvalidity.is_some()
            && folder.uidvalidity != Some(i64::from(snapshot.uidvalidity));
        if uidvalidity_changed {
            // Existing rows remain as local historical mail, but the remote UID cursor is invalidated.
            sqlx::query("UPDATE mail_folders SET last_seen_uid=0 WHERE id=$1")
                .bind(folder.id)
                .execute(&mut *transaction)
                .await?;
        }
        let mut touched_threads = BTreeSet::new();
        let mut persisted = 0;
        let mut highest_uid = if uidvalidity_changed {
            0
        } else {
            folder.last_seen_uid
        };
        for remote in snapshot.messages {
            highest_uid = highest_uid.max(i64::from(remote.uid));
            let parsed = parse_message(&remote.raw).map_err(|_| MailServiceError::Parse)?;
            if let Some(thread_id) = persist_message(
                &mut transaction,
                user_id,
                folder,
                i64::from(snapshot.uidvalidity),
                &remote,
                &parsed,
            )
            .await?
            {
                touched_threads.insert(thread_id);
                persisted += 1;
            }
        }
        sqlx::query(
            "UPDATE mail_folders SET uidvalidity=$2,uidnext=$3,last_seen_uid=$4,last_sync_at=now(),updated_at=now() WHERE id=$1",
        )
        .bind(folder.id)
        .bind(i64::from(snapshot.uidvalidity))
        .bind(snapshot.uidnext.map(i64::from))
        .bind(highest_uid)
        .execute(&mut *transaction)
        .await?;
        for thread_id in touched_threads {
            refresh_thread(&mut transaction, thread_id).await?;
        }
        transaction.commit().await?;
        Ok(persisted)
    }

    pub async fn list_threads(
        &self,
        user_id: &UserId,
        query: MailListQuery,
    ) -> Result<Vec<MailThread>, MailServiceError> {
        self.require_database()?;
        let user_id = Self::user_uuid(user_id)?;
        let q = query
            .q
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        sqlx::query_as::<_, MailThread>(
            r#"
            SELECT id,account_id,normalized_subject,latest_message_at,message_count,unread_count,participant_summary,snippet
            FROM mail_threads
            WHERE user_id=$1
              AND ($2::uuid IS NULL OR account_id=$2)
              AND ($3::text IS NULL OR normalized_subject ILIKE '%' || $3 || '%' OR coalesce(snippet,'') ILIKE '%' || $3 || '%')
              AND ($4::boolean IS NULL OR ($4=TRUE AND unread_count>0) OR $4=FALSE)
            ORDER BY latest_message_at DESC NULLS LAST
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(user_id)
        .bind(query.account_id)
        .bind(q)
        .bind(query.unread_only)
        .bind(query.limit.unwrap_or(100).clamp(1, 200))
        .bind(query.offset.unwrap_or(0).max(0))
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn thread_messages(
        &self,
        user_id: &UserId,
        thread_id: Uuid,
    ) -> Result<Vec<MailMessage>, MailServiceError> {
        self.require_database()?;
        let rows = sqlx::query_as::<_, MailMessage>(MESSAGE_SELECT_BY_THREAD)
            .bind(Self::user_uuid(user_id)?)
            .bind(thread_id)
            .fetch_all(&self.pool)
            .await?;
        if rows.is_empty() {
            return Err(MailServiceError::ThreadNotFound);
        }
        Ok(rows)
    }

    pub async fn message(
        &self,
        user_id: &UserId,
        message_id: Uuid,
    ) -> Result<MailMessage, MailServiceError> {
        self.require_database()?;
        sqlx::query_as::<_, MailMessage>(MESSAGE_SELECT_ONE)
            .bind(Self::user_uuid(user_id)?)
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(MailServiceError::MessageNotFound)
    }

    pub async fn attachments(
        &self,
        user_id: &UserId,
        message_id: Uuid,
    ) -> Result<Vec<MailAttachment>, MailServiceError> {
        self.require_database()?;
        sqlx::query_as::<_, MailAttachment>(
            r#"
            SELECT a.id,a.message_id,a.part_id,a.filename,a.mime_type,a.size_bytes,a.content_id,a.disposition,a.checksum,a.storage_ref,a.download_state
            FROM mail_attachments a JOIN mail_messages m ON m.id=a.message_id
            WHERE m.user_id=$1 AND a.message_id=$2 ORDER BY a.part_id
            "#,
        )
        .bind(Self::user_uuid(user_id)?)
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn set_message_read(
        &self,
        user_id: &UserId,
        message_id: Uuid,
        read: bool,
    ) -> Result<(), MailServiceError> {
        self.require_database()?;
        let user_id = Self::user_uuid(user_id)?;
        let remote = self.remote_message_ref(user_id, message_id).await?;
        let account = self.account_secret(user_id, remote.account_id).await?;
        let secret = Self::decrypt_secret(&account)?;
        protocol::set_seen(account, secret, remote.folder_name, remote.uid as u32, read).await?;
        sqlx::query(
            "UPDATE mail_messages SET is_read=$3,updated_at=now() WHERE user_id=$1 AND id=$2",
        )
        .bind(user_id)
        .bind(message_id)
        .bind(read)
        .execute(&self.pool)
        .await?;
        refresh_thread_pool(&self.pool, remote.thread_id).await?;
        Ok(())
    }

    pub async fn archive_message(
        &self,
        user_id: &UserId,
        message_id: Uuid,
    ) -> Result<(), MailServiceError> {
        self.require_database()?;
        let user_id = Self::user_uuid(user_id)?;
        let remote = self.remote_message_ref(user_id, message_id).await?;
        let archive_folder: Option<String> = sqlx::query_scalar(
            "SELECT remote_name FROM mail_folders WHERE user_id=$1 AND account_id=$2 AND normalized_role='archive' LIMIT 1",
        )
        .bind(user_id)
        .bind(remote.account_id)
        .fetch_optional(&self.pool)
        .await?;
        let archive_folder = archive_folder.ok_or(MailServiceError::ArchiveUnavailable)?;
        let account = self.account_secret(user_id, remote.account_id).await?;
        let secret = Self::decrypt_secret(&account)?;
        protocol::archive_message(
            account,
            secret,
            remote.folder_name,
            remote.uid as u32,
            archive_folder,
        )
        .await?;
        sqlx::query(
            "UPDATE mail_messages SET is_archived=TRUE,updated_at=now() WHERE user_id=$1 AND id=$2",
        )
        .bind(user_id)
        .bind(message_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remote_message_ref(
        &self,
        user_id: Uuid,
        message_id: Uuid,
    ) -> Result<RemoteMessageRef, MailServiceError> {
        sqlx::query_as::<_, RemoteMessageRef>(
            r#"
            SELECT m.account_id,m.folder_id,m.thread_id,m.remote_uid AS uid,f.remote_name AS folder_name
            FROM mail_messages m JOIN mail_folders f ON f.id=m.folder_id
            WHERE m.user_id=$1 AND m.id=$2
            "#,
        )
        .bind(user_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MailServiceError::MessageNotFound)
    }

    pub async fn send(
        &self,
        user_id: &UserId,
        account_id: Uuid,
        input: SendMailInput,
    ) -> Result<String, MailServiceError> {
        self.require_database()?;
        let user_id = Self::user_uuid(user_id)?;
        if input.to.is_empty() || input.idempotency_key.trim().is_empty() {
            return Err(MailServiceError::InvalidAccount);
        }
        let account = self.account_secret(user_id, account_id).await?;
        let secret = Self::decrypt_secret(&account)?;
        let domain = account
            .email_address
            .split('@')
            .nth(1)
            .unwrap_or("lifetrace.local");
        let generated_message_id = format!("<{}@{}>", Uuid::new_v4(), domain);
        let outbox_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO mail_outbox (id,user_id,account_id,idempotency_key,state,generated_message_id)
            VALUES ($1,$2,$3,$4,'queued',$5)
            ON CONFLICT (user_id,idempotency_key) DO NOTHING
            "#,
        )
        .bind(outbox_id)
        .bind(user_id)
        .bind(account_id)
        .bind(input.idempotency_key.trim())
        .bind(&generated_message_id)
        .execute(&self.pool)
        .await?;
        let existing: (Uuid, String, Option<String>) = sqlx::query_as(
            "SELECT id,state,generated_message_id FROM mail_outbox WHERE user_id=$1 AND idempotency_key=$2",
        )
        .bind(user_id)
        .bind(input.idempotency_key.trim())
        .fetch_one(&self.pool)
        .await?;
        if existing.1 == "sent" {
            return Ok(existing.2.unwrap_or(generated_message_id));
        }
        let claimed = sqlx::query(
            "UPDATE mail_outbox SET state='sending',attempt=attempt+1,updated_at=now() WHERE id=$1 AND state IN ('queued','retry_wait','failed')",
        )
        .bind(existing.0)
        .execute(&self.pool)
        .await?;
        if claimed.rows_affected() == 0 {
            return Err(MailServiceError::SendInProgress);
        }
        let message_id = existing.2.unwrap_or(generated_message_id);
        let in_reply_to = if let Some(source_id) = input.in_reply_to_message_id {
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT message_id FROM mail_messages WHERE user_id=$1 AND account_id=$2 AND id=$3",
            )
            .bind(user_id)
            .bind(account_id)
            .bind(source_id)
            .fetch_optional(&self.pool)
            .await?
            .flatten()
        } else {
            None
        };
        match protocol::send_mail(
            &account,
            &secret,
            &input,
            &message_id,
            in_reply_to.as_deref(),
        )
        .await
        {
            Ok(()) => {
                sqlx::query("UPDATE mail_outbox SET state='sent',sent_at=now(),updated_at=now(),last_error_code=NULL WHERE id=$1")
                    .bind(existing.0)
                    .execute(&self.pool)
                    .await?;
                Ok(message_id)
            }
            Err(_) => {
                sqlx::query("UPDATE mail_outbox SET state='retry_wait',next_retry_at=now()+interval '3 minutes',updated_at=now(),last_error_code='MAIL_SEND_FAILED' WHERE id=$1")
                    .bind(existing.0)
                    .execute(&self.pool)
                    .await?;
                Err(MailServiceError::Protocol)
            }
        }
    }
}

#[derive(Debug)]
struct ResolvedAccount {
    email: String,
    username: String,
    imap_host: String,
    imap_port: u16,
    imap_security: MailSecurity,
    smtp_host: String,
    smtp_port: u16,
    smtp_security: MailSecurity,
}

#[derive(Debug, sqlx::FromRow)]
struct RemoteMessageRef {
    account_id: Uuid,
    #[allow(dead_code)]
    folder_id: Uuid,
    thread_id: Uuid,
    uid: i64,
    folder_name: String,
}

const ACCOUNT_SELECT_LIST: &str = r#"
SELECT id,user_id,provider,email_address,display_name,
       imap_host,imap_port,imap_security,smtp_host,smtp_port,smtp_security,username,
       status,idle_supported,last_validated_at,last_sync_at,last_error_code,created_at,updated_at
FROM mail_accounts WHERE user_id=$1 AND deleted_at IS NULL ORDER BY created_at
"#;

const ACCOUNT_SELECT_ONE: &str = r#"
SELECT id,user_id,provider,email_address,display_name,
       imap_host,imap_port,imap_security,smtp_host,smtp_port,smtp_security,username,
       status,idle_supported,last_validated_at,last_sync_at,last_error_code,created_at,updated_at
FROM mail_accounts WHERE user_id=$1 AND id=$2 AND deleted_at IS NULL
"#;

const MESSAGE_SELECT_FIELDS: &str = r#"
id,account_id,folder_id,thread_id,remote_uid,uidvalidity,message_id,in_reply_to,subject,
from_json,to_json,cc_json,reply_to_json,sent_at,received_at,flags_json,is_read,is_archived,
size_bytes,snippet,body_text,body_html_sanitized,has_attachments
"#;

const MESSAGE_SELECT_BY_THREAD: &str = r#"
SELECT id,account_id,folder_id,thread_id,remote_uid,uidvalidity,message_id,in_reply_to,subject,
       from_json,to_json,cc_json,reply_to_json,sent_at,received_at,flags_json,is_read,is_archived,
       size_bytes,snippet,body_text,body_html_sanitized,has_attachments
FROM mail_messages WHERE user_id=$1 AND thread_id=$2 ORDER BY received_at ASC
"#;

const MESSAGE_SELECT_ONE: &str = r#"
SELECT id,account_id,folder_id,thread_id,remote_uid,uidvalidity,message_id,in_reply_to,subject,
       from_json,to_json,cc_json,reply_to_json,sent_at,received_at,flags_json,is_read,is_archived,
       size_bytes,snippet,body_text,body_html_sanitized,has_attachments
FROM mail_messages WHERE user_id=$1 AND id=$2
"#;

async fn persist_message(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    folder: &MailFolder,
    uidvalidity: i64,
    remote: &protocol::RemoteMessage,
    parsed: &ParsedMessage,
) -> Result<Option<Uuid>, MailServiceError> {
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM mail_messages WHERE account_id=$1 AND folder_id=$2 AND uidvalidity=$3 AND remote_uid=$4",
    )
    .bind(folder.account_id)
    .bind(folder.id)
    .bind(uidvalidity)
    .bind(i64::from(remote.uid))
    .fetch_optional(&mut **transaction)
    .await?;
    let seen = remote
        .flags
        .iter()
        .any(|flag| flag.eq_ignore_ascii_case("\\Seen") || flag.eq_ignore_ascii_case("Seen"));
    if let Some(message_id) = existing {
        sqlx::query(
            "UPDATE mail_messages SET flags_json=$2,is_read=$3,updated_at=now() WHERE id=$1",
        )
        .bind(message_id)
        .bind(serde_json::to_value(&remote.flags).unwrap_or_default())
        .bind(seen)
        .execute(&mut **transaction)
        .await?;
        let thread_id: Uuid = sqlx::query_scalar("SELECT thread_id FROM mail_messages WHERE id=$1")
            .bind(message_id)
            .fetch_one(&mut **transaction)
            .await?;
        return Ok(Some(thread_id));
    }

    let mut thread_id = None;
    if let Some(in_reply_to) = parsed.in_reply_to.as_deref() {
        thread_id = sqlx::query_scalar(
            "SELECT thread_id FROM mail_messages WHERE user_id=$1 AND account_id=$2 AND message_id=$3 ORDER BY received_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(folder.account_id)
        .bind(in_reply_to)
        .fetch_optional(&mut **transaction)
        .await?;
    }
    if thread_id.is_none() && !parsed.normalized_subject.is_empty() {
        thread_id = sqlx::query_scalar(
            "SELECT id FROM mail_threads WHERE user_id=$1 AND account_id=$2 AND normalized_subject=$3 ORDER BY latest_message_at DESC NULLS LAST LIMIT 1",
        )
        .bind(user_id)
        .bind(folder.account_id)
        .bind(&parsed.normalized_subject)
        .fetch_optional(&mut **transaction)
        .await?;
    }
    let thread_id = match thread_id {
        Some(value) => value,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO mail_threads (id,user_id,account_id,normalized_subject) VALUES ($1,$2,$3,$4)")
                .bind(id)
                .bind(user_id)
                .bind(folder.account_id)
                .bind(&parsed.normalized_subject)
                .execute(&mut **transaction)
                .await?;
            id
        }
    };
    let message_id = Uuid::new_v4();
    let received_at = remote.internal_date.unwrap_or_else(Utc::now);
    sqlx::query(
        r#"
        INSERT INTO mail_messages (
            id,user_id,account_id,folder_id,thread_id,remote_uid,uidvalidity,
            message_id,in_reply_to,subject,normalized_subject,
            from_json,to_json,cc_json,bcc_json,reply_to_json,sent_at,received_at,
            flags_json,is_read,is_archived,size_bytes,snippet,body_text,body_html_sanitized,
            has_attachments,content_hash
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,FALSE,$21,$22,$23,$24,$25,$26)
        "#,
    )
    .bind(message_id)
    .bind(user_id)
    .bind(folder.account_id)
    .bind(folder.id)
    .bind(thread_id)
    .bind(i64::from(remote.uid))
    .bind(uidvalidity)
    .bind(&parsed.message_id)
    .bind(&parsed.in_reply_to)
    .bind(&parsed.subject)
    .bind(&parsed.normalized_subject)
    .bind(&parsed.from_json)
    .bind(&parsed.to_json)
    .bind(&parsed.cc_json)
    .bind(&parsed.bcc_json)
    .bind(&parsed.reply_to_json)
    .bind(parsed.sent_at)
    .bind(received_at)
    .bind(serde_json::to_value(&remote.flags).unwrap_or_default())
    .bind(seen)
    .bind(remote.size.map(i64::from))
    .bind(&parsed.snippet)
    .bind(&parsed.body_text)
    .bind(&parsed.body_html_sanitized)
    .bind(!parsed.attachments.is_empty())
    .bind(&parsed.content_hash)
    .execute(&mut **transaction)
    .await?;
    for attachment in &parsed.attachments {
        sqlx::query(
            r#"
            INSERT INTO mail_attachments (id,user_id,message_id,part_id,filename,mime_type,size_bytes,content_id,disposition,checksum)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (message_id,part_id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(message_id)
        .bind(&attachment.part_id)
        .bind(&attachment.filename)
        .bind(&attachment.mime_type)
        .bind(attachment.size_bytes)
        .bind(&attachment.content_id)
        .bind(&attachment.disposition)
        .bind(&attachment.checksum)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(Some(thread_id))
}

async fn refresh_thread(
    transaction: &mut Transaction<'_, Postgres>,
    thread_id: Uuid,
) -> Result<(), MailServiceError> {
    sqlx::query(
        r#"
        UPDATE mail_threads t SET
            latest_message_at=s.latest_message_at,
            message_count=s.message_count,
            unread_count=s.unread_count,
            snippet=s.snippet,
            participant_summary=s.participant_summary,
            updated_at=now()
        FROM (
            SELECT thread_id,max(received_at) AS latest_message_at,count(*)::int AS message_count,
                   count(*) FILTER (WHERE NOT is_read)::int AS unread_count,
                   (array_agg(snippet ORDER BY received_at DESC))[1] AS snippet,
                   (array_agg(from_json::text ORDER BY received_at DESC))[1] AS participant_summary
            FROM mail_messages WHERE thread_id=$1 GROUP BY thread_id
        ) s WHERE t.id=s.thread_id
        "#,
    )
    .bind(thread_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn refresh_thread_pool(pool: &PgPool, thread_id: Uuid) -> Result<(), MailServiceError> {
    sqlx::query(
        r#"
        UPDATE mail_threads t SET
            latest_message_at=s.latest_message_at,message_count=s.message_count,unread_count=s.unread_count,
            snippet=s.snippet,participant_summary=s.participant_summary,updated_at=now()
        FROM (
            SELECT thread_id,max(received_at) AS latest_message_at,count(*)::int AS message_count,
                   count(*) FILTER (WHERE NOT is_read)::int AS unread_count,
                   (array_agg(snippet ORDER BY received_at DESC))[1] AS snippet,
                   (array_agg(from_json::text ORDER BY received_at DESC))[1] AS participant_summary
            FROM mail_messages WHERE thread_id=$1 GROUP BY thread_id
        ) s WHERE t.id=s.thread_id
        "#,
    )
    .bind(thread_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn folder_role(value: &str) -> &'static str {
    let normalized = value.to_ascii_lowercase();
    if normalized == "inbox" || value.contains("收件") {
        "inbox"
    } else if normalized.contains("sent") || value.contains("已发送") || value.contains("发件")
    {
        "sent"
    } else if normalized.contains("draft") || value.contains("草稿") {
        "drafts"
    } else if normalized.contains("trash")
        || normalized.contains("deleted")
        || value.contains("垃圾箱")
        || value.contains("已删除")
    {
        "trash"
    } else if normalized.contains("spam")
        || normalized.contains("junk")
        || value.contains("垃圾邮件")
    {
        "spam"
    } else if normalized.contains("archive") || value.contains("归档") {
        "archive"
    } else {
        "other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_common_folder_roles() {
        assert_eq!(folder_role("INBOX"), "inbox");
        assert_eq!(folder_role("Sent Messages"), "sent");
        assert_eq!(folder_role("已发送"), "sent");
        assert_eq!(folder_role("垃圾邮件"), "spam");
        assert_eq!(folder_role("Archive"), "archive");
    }

    #[test]
    fn generic_provider_requires_explicit_endpoints() {
        let input = MailAccountInput {
            provider: MailProvider::Generic,
            email_address: "me@example.com".to_owned(),
            display_name: None,
            username: None,
            authorization_code: "secret".to_owned(),
            imap_host: None,
            imap_port: None,
            imap_security: None,
            smtp_host: None,
            smtp_port: None,
            smtp_security: None,
        };
        assert!(matches!(
            MailService::resolve_input(&input),
            Err(MailServiceError::InvalidAccount)
        ));
    }
}
