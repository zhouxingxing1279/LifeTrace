CREATE TABLE IF NOT EXISTS photo_staging_items (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    client_asset_id TEXT,
    sha256 TEXT NOT NULL,
    original_name TEXT NOT NULL,
    media_type TEXT NOT NULL DEFAULT 'image',
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
    captured_at TIMESTAMPTZ,
    content BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS photo_staging_items_user_created_idx
    ON photo_staging_items(user_id, created_at);
CREATE INDEX IF NOT EXISTS photo_staging_items_expires_idx
    ON photo_staging_items(expires_at);
CREATE INDEX IF NOT EXISTS photo_staging_items_user_hash_idx
    ON photo_staging_items(user_id, sha256);
CREATE UNIQUE INDEX IF NOT EXISTS photo_staging_items_client_asset_uq
    ON photo_staging_items(user_id, source, client_asset_id)
    WHERE client_asset_id IS NOT NULL;

-- Disposable feature data. The long-lived photo relay above is intentionally
-- independent so it can remain after the photography challenge is removed.
CREATE TABLE IF NOT EXISTS photo_challenge_scores (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    staging_id UUID REFERENCES photo_staging_items(id) ON DELETE SET NULL,
    image_hash TEXT NOT NULL,
    file_name TEXT,
    captured_at TIMESTAMPTZ,
    score INTEGER NOT NULL CHECK (score BETWEEN 0 AND 100),
    qualified BOOLEAN NOT NULL,
    breakdown JSONB NOT NULL,
    feedback TEXT NOT NULL,
    model TEXT NOT NULL,
    thumbnail_data_url TEXT,
    scored_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, image_hash)
);

CREATE INDEX IF NOT EXISTS photo_challenge_scores_user_scored_idx
    ON photo_challenge_scores(user_id, scored_at DESC);
CREATE INDEX IF NOT EXISTS photo_challenge_scores_user_qualified_idx
    ON photo_challenge_scores(user_id, qualified, scored_at DESC);
