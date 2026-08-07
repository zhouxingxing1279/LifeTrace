-- 服务端变更日志：cursor 严格升序，删除保留 tombstone。

CREATE TABLE sync_change_log (
    cursor BIGSERIAL PRIMARY KEY,

    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,

    operation TEXT NOT NULL
        CHECK (operation IN ('upsert', 'delete')),

    entity_schema_version INTEGER NOT NULL,
    server_version BIGINT NOT NULL,

    payload JSONB,
    payload_hash BYTEA,
    tombstone JSONB,

    origin_device_id UUID,
    client_modified_at TIMESTAMPTZ,
    server_modified_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
