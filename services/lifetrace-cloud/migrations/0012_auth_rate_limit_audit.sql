CREATE TABLE auth_login_attempts (
    id BIGSERIAL PRIMARY KEY,
    email_hash BYTEA NOT NULL,
    ip_address INET,
    succeeded BOOLEAN NOT NULL,
    failure_reason TEXT,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE auth_audit_log (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID REFERENCES cloud_users(id) ON DELETE SET NULL,
    session_id UUID REFERENCES auth_sessions(id) ON DELETE SET NULL,
    device_id UUID REFERENCES cloud_devices(id) ON DELETE SET NULL,
    app_id TEXT,
    event_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    ip_address INET,
    user_agent TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
