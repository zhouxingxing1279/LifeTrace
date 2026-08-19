-- EPIC-12 unified file metadata. Raw file bytes are stored in S3-compatible object storage.

CREATE TABLE cloud_file_objects (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    domain TEXT NOT NULL CHECK (domain IN (
        'finance_import',
        'notes_attachment',
        'english_audio',
        'photo',
        'workout_import',
        'backup'
    )),
    original_name TEXT NOT NULL,
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    size_bytes BIGINT NOT NULL CHECK (size_bytes >= 0),
    mime_type TEXT NOT NULL,
    object_key TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'ready', 'failed', 'deleted')),
    upload_attempts INTEGER NOT NULL DEFAULT 1 CHECK (upload_attempts >= 0),
    storage_cleanup_pending BOOLEAN NOT NULL DEFAULT FALSE,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ready_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_cloud_file_objects_live_hash
ON cloud_file_objects(user_id, domain, sha256, size_bytes)
WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_cloud_file_objects_live_key
ON cloud_file_objects(object_key)
WHERE deleted_at IS NULL;

CREATE INDEX idx_cloud_file_objects_user_created
ON cloud_file_objects(user_id, created_at DESC)
WHERE deleted_at IS NULL;

CREATE INDEX idx_cloud_file_objects_integrity
ON cloud_file_objects(status, updated_at)
WHERE status IN ('pending', 'failed', 'ready') OR storage_cleanup_pending = TRUE;
