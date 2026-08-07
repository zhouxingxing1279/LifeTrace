-- 通用同步实体权威副本（完整经过契约校验的 Entity Payload）。

CREATE TABLE sync_entities (
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_schema_version INTEGER NOT NULL,

    server_version BIGINT NOT NULL
        CHECK (server_version > 0),

    payload JSONB,
    payload_hash BYTEA,

    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMPTZ,

    origin_device_id UUID,
    created_at TIMESTAMPTZ NOT NULL,
    server_modified_at TIMESTAMPTZ NOT NULL,
    client_modified_at TIMESTAMPTZ,

    last_cursor BIGINT NOT NULL,

    PRIMARY KEY (user_id, entity_type, entity_id),

    CHECK (
        (
            is_deleted = FALSE
            AND payload IS NOT NULL
            AND payload_hash IS NOT NULL
            AND deleted_at IS NULL
        )
        OR
        (
            is_deleted = TRUE
            AND payload IS NULL
            AND deleted_at IS NOT NULL
        )
    )
);
