-- EPIC-04 account fields and device metadata. Additive only.

ALTER TABLE cloud_users
    ADD COLUMN email TEXT,
    ADD COLUMN email_normalized TEXT,
    ADD COLUMN display_name TEXT,
    ADD COLUMN password_hash TEXT,
    ADD COLUMN password_version INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN email_verified_at TIMESTAMPTZ,
    ADD COLUMN password_changed_at TIMESTAMPTZ,
    ADD COLUMN disabled_at TIMESTAMPTZ,
    ADD COLUMN registration_source TEXT,
    ADD COLUMN failed_login_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN locked_until TIMESTAMPTZ,
    ADD COLUMN auth_state TEXT NOT NULL DEFAULT 'pending'
        CHECK (auth_state IN ('pending', 'active', 'password_reset_required', 'disabled'));

CREATE UNIQUE INDEX idx_cloud_users_email_normalized
ON cloud_users(email_normalized)
WHERE email_normalized IS NOT NULL;

ALTER TABLE cloud_devices
    ADD COLUMN device_group_id TEXT,
    ADD COLUMN device_name TEXT,
    ADD COLUMN last_sync_at TIMESTAMPTZ,
    ADD COLUMN last_login_at TIMESTAMPTZ,
    ADD COLUMN last_login_ip INET,
    ADD COLUMN last_user_agent TEXT,
    ADD COLUMN revoked_at TIMESTAMPTZ,
    ADD COLUMN revoked_reason TEXT;

UPDATE cloud_users
SET auth_state = CASE WHEN status = 'active' THEN 'active' ELSE 'disabled' END
WHERE email_normalized IS NULL;

UPDATE cloud_devices
SET device_name = COALESCE(device_name, external_device_id);
