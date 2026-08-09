use std::time::Duration;

use lifetrace_cloud::mail::credential::CredentialCipher;
use lifetrace_cloud::mail::domain::MailAccountSecret;
use lifetrace_cloud::mail::{protocol, MailService};
use lifetrace_cloud::{AppState, Config};
use lifetrace_contracts::UserId;
use tokio::task::JoinSet;
use uuid::Uuid;

const IDLE_WINDOW: Duration = Duration::from_secs(55);
const EMPTY_IDLE_SLEEP: Duration = Duration::from_secs(30);
const MAX_IDLE_ACCOUNTS: i64 = 32;
const MAX_POLL_ACCOUNTS: i64 = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    config.validate().map_err(|message| {
        eprintln!("[lifetrace-mail-worker] invalid configuration: {message}");
        message
    })?;
    let state = AppState::new(config);
    state.initialize().await?;
    if !state.database_enabled {
        return Err("mail worker requires DATABASE_URL".into());
    }
    // Fail before entering the worker loop when the external envelope key is absent or malformed.
    CredentialCipher::from_env()?;

    let service = MailService::new(state.pool.clone(), true);
    println!("[lifetrace-mail-worker] started");

    loop {
        let idle_accounts = load_idle_accounts(&state, MAX_IDLE_ACCOUNTS).await?;
        let mut idle_tasks = JoinSet::new();
        for account in idle_accounts.iter().cloned() {
            idle_tasks.spawn(async move {
                let cipher = CredentialCipher::from_env().ok()?;
                let secret = cipher
                    .decrypt(&account.credential_ciphertext, &account.credential_nonce)
                    .ok()?;
                match protocol::wait_for_inbox_change(account.clone(), secret, IDLE_WINDOW).await {
                    Ok(true) => Some((account.user_id, account.id)),
                    Ok(false) | Err(_) => None,
                }
            });
        }

        while let Some(result) = idle_tasks.join_next().await {
            if let Ok(Some((user_id, account_id))) = result {
                let user = UserId::new(user_id.to_string());
                if let Err(error) = service.sync_account(&user, account_id).await {
                    eprintln!(
                        "[lifetrace-mail-worker] idle-triggered sync failed account_id={account_id} error={error}"
                    );
                }
            }
        }

        match service.sync_due_accounts(MAX_POLL_ACCOUNTS).await {
            Ok(count) if count > 0 => {
                println!("[lifetrace-mail-worker] polling sync completed accounts={count}");
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("[lifetrace-mail-worker] polling sync failed error={error}");
            }
        }

        if idle_accounts.is_empty() {
            tokio::time::sleep(EMPTY_IDLE_SLEEP).await;
        }
    }
}

async fn load_idle_accounts(
    state: &AppState,
    limit: i64,
) -> Result<Vec<MailAccountSecret>, sqlx::Error> {
    sqlx::query_as::<_, MailAccountSecret>(
        r#"
        SELECT id,user_id,provider,email_address,display_name,
               imap_host,imap_port,imap_security,smtp_host,smtp_port,smtp_security,
               username,credential_ciphertext,credential_nonce,status
        FROM mail_accounts
        WHERE deleted_at IS NULL AND status IN ('active','degraded') AND idle_supported=TRUE
        ORDER BY last_sync_at NULLS FIRST
        LIMIT $1
        "#,
    )
    .bind(limit.clamp(1, 100))
    .fetch_all(&state.pool)
    .await
}

#[allow(dead_code)]
fn _assert_uuid_is_send_sync(_: Uuid) {}
