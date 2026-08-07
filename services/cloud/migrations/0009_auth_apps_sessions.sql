CREATE TABLE auth_app_grants (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked')),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, app_id)
);

CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES cloud_devices(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    session_type TEXT NOT NULL CHECK (session_type IN ('native', 'web')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked', 'expired')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    idle_expires_at TIMESTAMPTZ NOT NULL,
    absolute_expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    revoked_reason TEXT,
    login_ip INET,
    last_ip INET,
    user_agent TEXT,
    public_device BOOLEAN NOT NULL DEFAULT FALSE
);
