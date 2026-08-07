-- Change ID 幂等记录（userId + changeId 唯一）。

CREATE TABLE sync_processed_changes (
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    change_id UUID NOT NULL,
    request_hash BYTEA NOT NULL,

    result_status TEXT NOT NULL,
    result_json JSONB NOT NULL,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,

    processed_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, change_id)
);
