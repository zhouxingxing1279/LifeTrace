-- Durable binary objects for the BeeCount compatibility boundary.
--
-- Accounting entities remain authoritative in sync_entities/sync_change_log.
-- This table only stores file bytes and the BeeCount lookup dimensions that
-- cannot live in the public file.metadata payload.

CREATE TABLE cloud_file_blobs (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,
    file_entity_id TEXT NOT NULL,
    ledger_id TEXT,
    attachment_kind TEXT NOT NULL
        CHECK (attachment_kind IN ('transaction_attachment', 'category_icon')),
    sha256 TEXT NOT NULL,
    size_bytes BIGINT NOT NULL
        CHECK (size_bytes > 0),
    mime_type TEXT,
    file_name TEXT NOT NULL,
    content BYTEA NOT NULL,
    created_by_device_id UUID
        REFERENCES cloud_devices(id)
        ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, file_entity_id),
    CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    CHECK (length(file_name) BETWEEN 1 AND 255),
    CHECK (octet_length(content) = size_bytes),
    CHECK (
        (attachment_kind = 'transaction_attachment' AND ledger_id IS NOT NULL)
        OR (attachment_kind = 'category_icon' AND ledger_id IS NULL)
    )
);

CREATE UNIQUE INDEX idx_cloud_file_blobs_transaction_dedup
ON cloud_file_blobs(user_id, ledger_id, sha256)
WHERE attachment_kind = 'transaction_attachment';

CREATE UNIQUE INDEX idx_cloud_file_blobs_category_icon_dedup
ON cloud_file_blobs(user_id, sha256)
WHERE attachment_kind = 'category_icon';

CREATE INDEX idx_cloud_file_blobs_user_created
ON cloud_file_blobs(user_id, created_at DESC);
