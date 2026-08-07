-- EPIC-03 云端身份锚点（登录凭据属于 EPIC-04）。
-- 只创建身份锚点，不存 email/password_hash/refresh_token。

CREATE TABLE cloud_users (
    id UUID PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE cloud_devices (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    app_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    client_version TEXT,
    protocol_version INTEGER,
    schema_version INTEGER,

    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'revoked')),

    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
