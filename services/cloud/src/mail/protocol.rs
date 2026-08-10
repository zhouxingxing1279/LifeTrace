use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use imap::{ConnectionMode, TlsKind};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use thiserror::Error;

use super::domain::{MailAccountSecret, SendMailInput};

#[derive(Debug, Error)]
pub enum MailProtocolError {
    #[error("mail server connection failed")]
    Connect,
    #[error("mail authentication failed")]
    Authentication,
    #[error("mail folder operation failed")]
    Folder,
    #[error("mail message fetch failed")]
    Fetch,
    #[error("mail message state update failed")]
    State,
    #[error("mail server does not support the requested capability")]
    Capability,
    #[error("mail address is invalid")]
    InvalidAddress,
    #[error("mail message could not be built")]
    MessageBuild,
    #[error("mail send failed")]
    Send,
    #[error("mail protocol task failed")]
    Task,
}

#[derive(Debug, Clone)]
pub struct ImapProbe {
    pub idle_supported: bool,
    pub move_supported: bool,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteFolderSnapshot {
    pub uidvalidity: u32,
    pub uidnext: Option<u32>,
    pub messages: Vec<RemoteMessage>,
}

#[derive(Debug, Clone)]
pub struct RemoteMessage {
    pub uid: u32,
    pub size: Option<u32>,
    pub internal_date: Option<DateTime<Utc>>,
    pub flags: Vec<String>,
    pub raw: Vec<u8>,
}

fn connection_mode(value: &str, port: i32) -> ConnectionMode {
    match value {
        "starttls" => ConnectionMode::StartTls,
        "tls" => ConnectionMode::Tls,
        _ if port == 143 => ConnectionMode::StartTls,
        _ => ConnectionMode::Tls,
    }
}

fn imap_client(
    account: &MailAccountSecret,
) -> Result<imap::Client<imap::Connection>, MailProtocolError> {
    imap::ClientBuilder::new(&account.imap_host, account.imap_port as u16)
        .mode(connection_mode(&account.imap_security, account.imap_port))
        .tls_kind(TlsKind::Rust)
        .connect()
        .map_err(|_| MailProtocolError::Connect)
}

fn requires_imap_client_id(account: &MailAccountSecret) -> bool {
    matches!(account.provider.as_str(), "126" | "163" | "yeah")
        || matches!(
            account.imap_host.to_ascii_lowercase().as_str(),
            "imap.126.com" | "imap.163.com" | "imap.yeah.net"
        )
}

fn identify_imap_session(
    session: &mut imap::Session<imap::Connection>,
    account: &MailAccountSecret,
) -> Result<(), MailProtocolError> {
    if !requires_imap_client_id(account) {
        return Ok(());
    }
    let command = format!(
        "ID (\"name\" \"LifeTrace\" \"version\" \"{}\" \"vendor\" \"LifeTrace\")",
        env!("CARGO_PKG_VERSION")
    );
    session
        .run_command_and_check_ok(command)
        .map_err(|_| MailProtocolError::Capability)
}

pub async fn probe_imap(
    account: MailAccountSecret,
    secret: String,
) -> Result<ImapProbe, MailProtocolError> {
    tokio::task::spawn_blocking(move || {
        let client = imap_client(&account)?;
        let mut session = client
            .login(&account.username, &secret)
            .map_err(|_| MailProtocolError::Authentication)?;
        identify_imap_session(&mut session, &account)?;
        let capabilities = session
            .capabilities()
            .map_err(|_| MailProtocolError::Capability)?;
        let idle_supported = capabilities.has_str("IDLE");
        let move_supported = capabilities.has_str("MOVE");
        let folders = session
            .list(None, Some("*"))
            .map_err(|_| MailProtocolError::Folder)?
            .iter()
            .map(|name| name.name().to_owned())
            .collect();
        session
            .examine("INBOX")
            .map_err(|_| MailProtocolError::Folder)?;
        let _ = session.logout();
        Ok(ImapProbe {
            idle_supported,
            move_supported,
            folders,
        })
    })
    .await
    .map_err(|_| MailProtocolError::Task)?
}

pub async fn fetch_folder(
    account: MailAccountSecret,
    secret: String,
    folder: String,
    previous_uidvalidity: Option<i64>,
    last_seen_uid: i64,
    initial_since: Option<DateTime<Utc>>,
) -> Result<RemoteFolderSnapshot, MailProtocolError> {
    tokio::task::spawn_blocking(move || {
        let client = imap_client(&account)?;
        let mut session = client
            .login(&account.username, &secret)
            .map_err(|_| MailProtocolError::Authentication)?;
        identify_imap_session(&mut session, &account)?;
        let mailbox = session
            .select(&folder)
            .map_err(|_| MailProtocolError::Folder)?;
        let uidvalidity = mailbox.uid_validity.ok_or(MailProtocolError::Capability)?;

        let uid_query = if previous_uidvalidity == Some(i64::from(uidvalidity)) && last_seen_uid > 0
        {
            format!("{}:*", last_seen_uid.saturating_add(1))
        } else if let Some(since) = initial_since {
            let query = format!("SINCE {}", since.format("%d-%b-%Y"));
            let mut uids: Vec<u32> = session
                .uid_search(query)
                .map_err(|_| MailProtocolError::Fetch)?
                .into_iter()
                .collect();
            uids.sort_unstable();
            if uids.is_empty() {
                let _ = session.logout();
                return Ok(RemoteFolderSnapshot {
                    uidvalidity,
                    uidnext: mailbox.uid_next,
                    messages: Vec::new(),
                });
            }
            uids.into_iter()
                .map(|uid| uid.to_string())
                .collect::<Vec<_>>()
                .join(",")
        } else {
            "1:*".to_owned()
        };

        let fetched = session
            .uid_fetch(
                uid_query,
                "(UID FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[])",
            )
            .map_err(|_| MailProtocolError::Fetch)?;
        let messages = fetched
            .iter()
            .filter_map(|item| {
                let uid = item.uid?;
                let raw = item.body()?.to_vec();
                let internal_date = item
                    .internal_date()
                    .map(|value: DateTime<FixedOffset>| value.with_timezone(&Utc));
                Some(RemoteMessage {
                    uid,
                    size: item.size,
                    internal_date,
                    flags: item.flags().iter().map(ToString::to_string).collect(),
                    raw,
                })
            })
            .collect();
        let _ = session.logout();
        Ok(RemoteFolderSnapshot {
            uidvalidity,
            uidnext: mailbox.uid_next,
            messages,
        })
    })
    .await
    .map_err(|_| MailProtocolError::Task)?
}

pub async fn fetch_raw_message(
    account: MailAccountSecret,
    secret: String,
    folder: String,
    uid: u32,
) -> Result<Vec<u8>, MailProtocolError> {
    tokio::task::spawn_blocking(move || {
        let client = imap_client(&account)?;
        let mut session = client
            .login(&account.username, &secret)
            .map_err(|_| MailProtocolError::Authentication)?;
        identify_imap_session(&mut session, &account)?;
        session
            .select(&folder)
            .map_err(|_| MailProtocolError::Folder)?;
        let fetched = session
            .uid_fetch(uid.to_string(), "(UID BODY.PEEK[])")
            .map_err(|_| MailProtocolError::Fetch)?;
        let raw = fetched
            .iter()
            .next()
            .and_then(|item| item.body())
            .map(ToOwned::to_owned)
            .ok_or(MailProtocolError::Fetch)?;
        let _ = session.logout();
        Ok(raw)
    })
    .await
    .map_err(|_| MailProtocolError::Task)?
}

pub async fn set_seen(
    account: MailAccountSecret,
    secret: String,
    folder: String,
    uid: u32,
    seen: bool,
) -> Result<(), MailProtocolError> {
    tokio::task::spawn_blocking(move || {
        let client = imap_client(&account)?;
        let mut session = client
            .login(&account.username, &secret)
            .map_err(|_| MailProtocolError::Authentication)?;
        identify_imap_session(&mut session, &account)?;
        session
            .select(&folder)
            .map_err(|_| MailProtocolError::Folder)?;
        let operation = if seen {
            "+FLAGS.SILENT (\\Seen)"
        } else {
            "-FLAGS.SILENT (\\Seen)"
        };
        session
            .uid_store(uid.to_string(), operation)
            .map_err(|_| MailProtocolError::State)?;
        let _ = session.logout();
        Ok(())
    })
    .await
    .map_err(|_| MailProtocolError::Task)?
}

pub async fn archive_message(
    account: MailAccountSecret,
    secret: String,
    folder: String,
    uid: u32,
    archive_folder: String,
) -> Result<(), MailProtocolError> {
    tokio::task::spawn_blocking(move || {
        let client = imap_client(&account)?;
        let mut session = client
            .login(&account.username, &secret)
            .map_err(|_| MailProtocolError::Authentication)?;
        identify_imap_session(&mut session, &account)?;
        session
            .select(&folder)
            .map_err(|_| MailProtocolError::Folder)?;
        let capabilities = session
            .capabilities()
            .map_err(|_| MailProtocolError::Capability)?;
        if capabilities.has_str("MOVE") {
            session
                .uid_mv(uid.to_string(), archive_folder)
                .map_err(|_| MailProtocolError::State)?;
        } else {
            session
                .uid_copy(uid.to_string(), &archive_folder)
                .map_err(|_| MailProtocolError::State)?;
            session
                .uid_store(uid.to_string(), "+FLAGS.SILENT (\\Deleted)")
                .map_err(|_| MailProtocolError::State)?;
            session.expunge().map_err(|_| MailProtocolError::State)?;
        }
        let _ = session.logout();
        Ok(())
    })
    .await
    .map_err(|_| MailProtocolError::Task)?
}

pub async fn wait_for_inbox_change(
    account: MailAccountSecret,
    secret: String,
    timeout: Duration,
) -> Result<bool, MailProtocolError> {
    tokio::task::spawn_blocking(move || {
        let client = imap_client(&account)?;
        let mut session = client
            .login(&account.username, &secret)
            .map_err(|_| MailProtocolError::Authentication)?;
        identify_imap_session(&mut session, &account)?;
        let capabilities = session
            .capabilities()
            .map_err(|_| MailProtocolError::Capability)?;
        if !capabilities.has_str("IDLE") {
            return Err(MailProtocolError::Capability);
        }
        session
            .select("INBOX")
            .map_err(|_| MailProtocolError::Folder)?;
        let outcome = {
            let mut idle = session.idle();
            idle.timeout(timeout).keepalive(false);
            idle.wait_while(imap::extensions::idle::stop_on_any)
                .map_err(|_| MailProtocolError::Connect)?
        };
        let changed = matches!(outcome, imap::extensions::idle::WaitOutcome::MailboxChanged);
        let _ = session.logout();
        Ok(changed)
    })
    .await
    .map_err(|_| MailProtocolError::Task)?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(provider: &str, host: &str) -> MailAccountSecret {
        MailAccountSecret {
            id: uuid::Uuid::nil(),
            user_id: uuid::Uuid::nil(),
            provider: provider.to_owned(),
            email_address: "test@example.com".to_owned(),
            display_name: None,
            imap_host: host.to_owned(),
            imap_port: 993,
            imap_security: "tls".to_owned(),
            smtp_host: "smtp.example.com".to_owned(),
            smtp_port: 465,
            smtp_security: "tls".to_owned(),
            username: "test@example.com".to_owned(),
            credential_ciphertext: Vec::new(),
            credential_nonce: Vec::new(),
            status: "active".to_owned(),
        }
    }

    #[test]
    fn netease_accounts_require_imap_client_identity() {
        assert!(requires_imap_client_id(&account("126", "imap.126.com")));
        assert!(requires_imap_client_id(&account("generic", "imap.163.com")));
        assert!(!requires_imap_client_id(&account("qq", "imap.qq.com")));
    }
}

fn smtp_transport(
    account: &MailAccountSecret,
    secret: &str,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailProtocolError> {
    let credentials = Credentials::new(account.username.clone(), secret.to_owned());
    let builder = if account.smtp_security == "starttls" {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&account.smtp_host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&account.smtp_host)
    }
    .map_err(|_| MailProtocolError::Connect)?;
    Ok(builder
        .port(account.smtp_port as u16)
        .credentials(credentials)
        .timeout(Some(Duration::from_secs(30)))
        .build())
}

pub async fn probe_smtp(
    account: &MailAccountSecret,
    secret: &str,
) -> Result<(), MailProtocolError> {
    let transport = smtp_transport(account, secret)?;
    match tokio::time::timeout(Duration::from_secs(35), transport.test_connection()).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err(MailProtocolError::Connect),
        Ok(Err(_)) => Err(MailProtocolError::Authentication),
        Err(_) => Err(MailProtocolError::Connect),
    }
}

fn mailbox(value: &str) -> Result<Mailbox, MailProtocolError> {
    value.parse().map_err(|_| MailProtocolError::InvalidAddress)
}

pub async fn send_mail(
    account: &MailAccountSecret,
    secret: &str,
    input: &SendMailInput,
    message_id: &str,
    in_reply_to: Option<&str>,
) -> Result<(), MailProtocolError> {
    let from = mailbox(&account.email_address)?;
    let first_to = input.to.first().ok_or(MailProtocolError::InvalidAddress)?;
    let mut builder = Message::builder()
        .from(from)
        .to(mailbox(first_to)?)
        .subject(&input.subject)
        .header(ContentType::TEXT_PLAIN)
        .message_id(Some(message_id.to_owned()));
    for value in input.to.iter().skip(1) {
        builder = builder.to(mailbox(value)?);
    }
    for value in &input.cc {
        builder = builder.cc(mailbox(value)?);
    }
    for value in &input.bcc {
        builder = builder.bcc(mailbox(value)?);
    }
    if let Some(value) = in_reply_to {
        builder = builder.in_reply_to(value.to_owned());
    }
    let message = builder
        .body(input.body_text.clone())
        .map_err(|_| MailProtocolError::MessageBuild)?;
    let transport = smtp_transport(account, secret)?;
    tokio::time::timeout(Duration::from_secs(35), transport.send(message))
        .await
        .map_err(|_| MailProtocolError::Send)?
        .map_err(|_| MailProtocolError::Send)?;
    Ok(())
}
