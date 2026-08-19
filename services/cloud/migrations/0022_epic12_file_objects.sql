-- EPIC-12 unified file metadata. Object bytes live in S3-compatible storage.
-- Private vault media and transient photo_staging bytes are intentionally excluded.

CREATE TABLE file_objects (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    domain TEXT NOT NULL CHECK (domain IN (
        'finance_imports',
        'notes_attachments',
        'english_audio',
        'photos',
        'workout_imports',
        'backups'
    )),
    original_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    storage_key TEXT NOT NULL UNIQUE,
    entity_type TEXT,
    entity_id TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'available', 'failed')),
    upload_attempts INTEGER NOT NULL DEFAULT 0 CHECK (upload_attempts >= 0),
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    available_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    CHECK ((entity_type IS NULL) = (entity_id IS NULL))
);

CREATE UNIQUE INDEX idx_file_objects_owner_domain_hash
ON file_objects(user_id, domain, sha256, size_bytes)
WHERE deleted_at IS NULL;

CREATE INDEX idx_file_objects_owner_domain_created
ON file_objects(user_id, domain, created_at DESC)
WHERE deleted_at IS NULL;

CREATE INDEX idx_file_objects_owner_entity
ON file_objects(user_id, entity_type, entity_id)
WHERE deleted_at IS NULL AND entity_type IS NOT NULL;

CREATE INDEX idx_file_objects_orphan_candidates
ON file_objects(created_at)
WHERE deleted_at IS NULL AND entity_type IS NULL AND status IN ('pending', 'failed');
