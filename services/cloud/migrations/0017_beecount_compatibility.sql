-- BeeCount protocol compatibility and SQLite -> PostgreSQL cutover state.
--
-- LifeTrace sync_entities/sync_change_log remain the only authoritative entity
-- store. These tables contain compatibility metadata that cannot be represented
-- by the LifeTrace optimistic-version protocol itself: legacy identity links,
-- BeeCount's (updated_at, device_id) LWW clock and resumable import checkpoints.

CREATE TABLE beecount_identity_links (
    user_id UUID PRIMARY KEY
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    beecount_user_id TEXT NOT NULL UNIQUE,
    source_email_normalized TEXT,
    source_kind TEXT NOT NULL DEFAULT 'native'
        CHECK (source_kind IN ('native', 'sqlite_import')),
    linked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    source_created_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    CHECK (length(trim(beecount_user_id)) > 0)
);

CREATE UNIQUE INDEX idx_beecount_identity_source_email
ON beecount_identity_links(source_email_normalized)
WHERE source_email_normalized IS NOT NULL;

CREATE TABLE beecount_entity_clocks (
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_sync_id TEXT NOT NULL,
    ledger_id TEXT,
    scope TEXT NOT NULL
        CHECK (scope IN ('user', 'ledger')),
    updated_at TIMESTAMPTZ NOT NULL,
    updated_by_device_id TEXT NOT NULL,
    lifetrace_entity_type TEXT NOT NULL,
    lifetrace_entity_id TEXT NOT NULL,
    lifetrace_server_version BIGINT NOT NULL
        CHECK (lifetrace_server_version > 0),
    lifetrace_cursor BIGINT NOT NULL
        CHECK (lifetrace_cursor >= 0),
    source_change_id BIGINT,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, entity_type, entity_sync_id),
    CHECK (length(trim(entity_type)) > 0),
    CHECK (length(trim(entity_sync_id)) > 0),
    CHECK (length(trim(updated_by_device_id)) > 0),
    CHECK (
        (scope = 'user' AND ledger_id IS NULL)
        OR (scope = 'ledger' AND ledger_id IS NOT NULL)
    )
);

CREATE INDEX idx_beecount_entity_clocks_ledger
ON beecount_entity_clocks(user_id, ledger_id, lifetrace_cursor)
WHERE ledger_id IS NOT NULL;

CREATE INDEX idx_beecount_entity_clocks_cursor
ON beecount_entity_clocks(user_id, lifetrace_cursor);

CREATE UNIQUE INDEX idx_beecount_entity_clocks_lifetrace_entity
ON beecount_entity_clocks(
    user_id,
    lifetrace_entity_type,
    lifetrace_entity_id
);

CREATE TABLE beecount_migration_runs (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    source_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN (
            'planned',
            'importing',
            'shadow_read',
            'ready_for_cutover',
            'cutover',
            'rolled_back',
            'failed'
        )),
    source_cursor BIGINT NOT NULL DEFAULT 0
        CHECK (source_cursor >= 0),
    imported_source_cursor BIGINT NOT NULL DEFAULT 0
        CHECK (imported_source_cursor >= 0),
    imported_entities BIGINT NOT NULL DEFAULT 0
        CHECK (imported_entities >= 0),
    imported_tombstones BIGINT NOT NULL DEFAULT 0
        CHECK (imported_tombstones >= 0),
    comparison_mismatches BIGINT NOT NULL DEFAULT 0
        CHECK (comparison_mismatches >= 0),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, source_fingerprint),
    CHECK (imported_source_cursor <= source_cursor)
);

CREATE INDEX idx_beecount_migration_runs_status
ON beecount_migration_runs(user_id, status, updated_at DESC);
