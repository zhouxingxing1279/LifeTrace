-- Snapshot 一致视图（items 使用 keyset 分页，禁止 OFFSET）。

CREATE TABLE sync_snapshots (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    scope_hash BYTEA NOT NULL,
    snapshot_cursor BIGINT NOT NULL,

    status TEXT NOT NULL
        CHECK (status IN ('building', 'ready', 'failed', 'expired')),

    item_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    error_message TEXT
);

CREATE TABLE sync_snapshot_items (
    snapshot_id UUID NOT NULL
        REFERENCES sync_snapshots(id)
        ON DELETE CASCADE,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_schema_version INTEGER NOT NULL,
    server_version BIGINT NOT NULL,
    payload JSONB NOT NULL,
    payload_hash BYTEA NOT NULL,
    server_modified_at TIMESTAMPTZ NOT NULL,

    PRIMARY KEY (snapshot_id, entity_type, entity_id)
);
