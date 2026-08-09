-- EPIC-27 mail aggregation foundation. Non-AI scope only.

CREATE TABLE mail_accounts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('qq', '163', '126', 'yeah', 'generic')),
    email_address TEXT NOT NULL,
    display_name TEXT,
    imap_host TEXT NOT NULL,
    imap_port INTEGER NOT NULL CHECK (imap_port BETWEEN 1 AND 65535),
    imap_security TEXT NOT NULL CHECK (imap_security IN ('tls', 'starttls')),
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL CHECK (smtp_port BETWEEN 1 AND 65535),
    smtp_security TEXT NOT NULL CHECK (smtp_security IN ('tls', 'starttls')),
    username TEXT NOT NULL,
    credential_ciphertext BYTEA NOT NULL,
    credential_nonce BYTEA NOT NULL,
    status TEXT NOT NULL DEFAULT 'validating'
        CHECK (status IN ('validating', 'active', 'degraded', 'disabled')),
    idle_supported BOOLEAN NOT NULL DEFAULT FALSE,
    last_validated_at TIMESTAMPTZ,
    last_sync_at TIMESTAMPTZ,
    last_error_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_mail_accounts_user_address_provider
ON mail_accounts(user_id, lower(email_address), provider)
WHERE deleted_at IS NULL;
CREATE INDEX idx_mail_accounts_worker
ON mail_accounts(status, last_sync_at)
WHERE deleted_at IS NULL;

CREATE TABLE mail_folders (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    remote_name TEXT NOT NULL,
    normalized_role TEXT NOT NULL DEFAULT 'other'
        CHECK (normalized_role IN ('inbox', 'sent', 'drafts', 'trash', 'spam', 'archive', 'other')),
    uidvalidity BIGINT,
    uidnext BIGINT,
    highest_modseq BIGINT,
    last_seen_uid BIGINT NOT NULL DEFAULT 0,
    last_sync_at TIMESTAMPTZ,
    sync_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(account_id, remote_name)
);
CREATE INDEX idx_mail_folders_account ON mail_folders(account_id, sync_enabled);

CREATE TABLE mail_threads (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    normalized_subject TEXT NOT NULL DEFAULT '',
    latest_message_at TIMESTAMPTZ,
    message_count INTEGER NOT NULL DEFAULT 0,
    unread_count INTEGER NOT NULL DEFAULT 0,
    participant_summary TEXT,
    snippet TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_mail_threads_user_latest ON mail_threads(user_id, latest_message_at DESC);
CREATE INDEX idx_mail_threads_account_latest ON mail_threads(account_id, latest_message_at DESC);

CREATE TABLE mail_messages (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    folder_id UUID NOT NULL REFERENCES mail_folders(id) ON DELETE CASCADE,
    thread_id UUID NOT NULL REFERENCES mail_threads(id) ON DELETE CASCADE,
    remote_uid BIGINT NOT NULL,
    uidvalidity BIGINT NOT NULL,
    message_id TEXT,
    in_reply_to TEXT,
    references_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    subject TEXT NOT NULL DEFAULT '',
    normalized_subject TEXT NOT NULL DEFAULT '',
    from_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    to_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    cc_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    bcc_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    reply_to_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    sent_at TIMESTAMPTZ,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    flags_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    size_bytes BIGINT,
    snippet TEXT,
    body_text TEXT,
    body_html_sanitized TEXT,
    has_attachments BOOLEAN NOT NULL DEFAULT FALSE,
    content_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(account_id, folder_id, uidvalidity, remote_uid)
);
CREATE INDEX idx_mail_messages_account_received ON mail_messages(account_id, received_at DESC);
CREATE INDEX idx_mail_messages_thread_received ON mail_messages(thread_id, received_at ASC);
CREATE INDEX idx_mail_messages_account_message_id ON mail_messages(account_id, message_id);
CREATE INDEX idx_mail_messages_user_unread ON mail_messages(user_id, is_read, received_at DESC);
CREATE INDEX idx_mail_messages_search ON mail_messages USING GIN (to_tsvector('simple', coalesce(subject, '') || ' ' || coalesce(body_text, '')));

CREATE TABLE mail_attachments (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    message_id UUID NOT NULL REFERENCES mail_messages(id) ON DELETE CASCADE,
    part_id TEXT NOT NULL,
    filename TEXT,
    mime_type TEXT,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    content_id TEXT,
    disposition TEXT,
    checksum TEXT,
    storage_ref TEXT,
    download_state TEXT NOT NULL DEFAULT 'metadata_only'
        CHECK (download_state IN ('metadata_only', 'downloading', 'ready', 'failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(message_id, part_id)
);

CREATE TABLE mail_sync_jobs (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    folder_id UUID REFERENCES mail_folders(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('initial', 'incremental', 'reconcile', 'idle_recovery')),
    state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'success', 'partial', 'retry_wait', 'dead')),
    cursor_before_json JSONB,
    cursor_after_json JSONB,
    attempt INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    next_retry_at TIMESTAMPTZ,
    error_code TEXT,
    error_detail_redacted TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_mail_sync_jobs_retry ON mail_sync_jobs(state, next_retry_at);

CREATE TABLE mail_drafts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    thread_id UUID REFERENCES mail_threads(id) ON DELETE SET NULL,
    in_reply_to_message_id UUID REFERENCES mail_messages(id) ON DELETE SET NULL,
    to_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    cc_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    bcc_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    subject TEXT NOT NULL DEFAULT '',
    body_text TEXT NOT NULL DEFAULT '',
    state TEXT NOT NULL DEFAULT 'draft' CHECK (state IN ('draft', 'queued', 'sent', 'canceled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE mail_outbox (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    draft_id UUID REFERENCES mail_drafts(id) ON DELETE SET NULL,
    idempotency_key TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'queued'
        CHECK (state IN ('queued', 'sending', 'sent', 'retry_wait', 'failed', 'canceled')),
    generated_message_id TEXT,
    attempt INTEGER NOT NULL DEFAULT 0,
    next_retry_at TIMESTAMPTZ,
    last_error_code TEXT,
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, idempotency_key)
);
CREATE INDEX idx_mail_outbox_pending ON mail_outbox(state, next_retry_at);
