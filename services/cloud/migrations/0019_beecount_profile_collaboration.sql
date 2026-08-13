-- BeeCount profile, device detail and shared-ledger compatibility metadata.
--
-- Financial entities remain authoritative in sync_entities/sync_change_log.
-- The shared-ledger tables below only describe access, roles and invitations.

ALTER TABLE cloud_devices
    ADD COLUMN os_version TEXT,
    ADD COLUMN device_model TEXT;

CREATE TABLE beecount_user_profiles (
    user_id UUID PRIMARY KEY
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    income_is_red BOOLEAN,
    theme_primary_color TEXT
        CHECK (theme_primary_color IS NULL OR theme_primary_color ~ '^#[0-9A-F]{6}$'),
    appearance JSONB,
    ai_config JSONB,
    primary_currency TEXT
        CHECK (primary_currency IS NULL OR primary_currency ~ '^[A-Z]{3,8}$'),
    avatar_version BIGINT NOT NULL DEFAULT 0
        CHECK (avatar_version >= 0),
    avatar_mime_type TEXT,
    avatar_file_name TEXT,
    avatar_content BYTEA,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        (avatar_content IS NULL AND avatar_mime_type IS NULL AND avatar_file_name IS NULL)
        OR
        (avatar_content IS NOT NULL AND avatar_mime_type IS NOT NULL AND avatar_file_name IS NOT NULL)
    )
);

CREATE TABLE beecount_shared_ledgers (
    ledger_id TEXT PRIMARY KEY,
    storage_user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (length(trim(ledger_id)) BETWEEN 1 AND 256)
);

CREATE INDEX idx_beecount_shared_ledgers_owner
ON beecount_shared_ledgers(storage_user_id, created_at);

CREATE TABLE beecount_ledger_members (
    ledger_id TEXT NOT NULL
        REFERENCES beecount_shared_ledgers(ledger_id)
        ON DELETE CASCADE,
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    role TEXT NOT NULL
        CHECK (role IN ('owner', 'editor')),
    invited_by UUID
        REFERENCES cloud_users(id)
        ON DELETE SET NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (ledger_id, user_id)
);

CREATE UNIQUE INDEX idx_beecount_ledger_one_owner
ON beecount_ledger_members(ledger_id)
WHERE role = 'owner';

CREATE INDEX idx_beecount_ledger_members_user
ON beecount_ledger_members(user_id, joined_at);

CREATE TABLE beecount_ledger_invites (
    code TEXT PRIMARY KEY,
    ledger_id TEXT NOT NULL
        REFERENCES beecount_shared_ledgers(ledger_id)
        ON DELETE CASCADE,
    invited_by UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    target_role TEXT NOT NULL DEFAULT 'editor'
        CHECK (target_role = 'editor'),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    used_by UUID
        REFERENCES cloud_users(id)
        ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (code ~ '^[A-HJ-NP-Z2-9]{6}$'),
    CHECK ((used_at IS NULL AND used_by IS NULL) OR used_at IS NOT NULL)
);

CREATE INDEX idx_beecount_ledger_invites_active
ON beecount_ledger_invites(ledger_id, expires_at DESC)
WHERE used_at IS NULL;
