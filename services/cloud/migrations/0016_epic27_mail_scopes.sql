-- Existing desktop/web grants were created before EPIC-27 introduced mail scopes.
-- Upgrade only clients whose allowlist already includes the mail domain.

UPDATE auth_app_grants
SET scopes = ARRAY(
        SELECT DISTINCT value
        FROM unnest(auth_app_grants.scopes || ARRAY['mail:read', 'mail:write']::TEXT[]) AS value
        ORDER BY value
    ),
    updated_at = now()
WHERE app_id IN ('lifetrace-desktop', 'lifetrace-web')
  AND status = 'active';

UPDATE auth_sessions
SET scopes = ARRAY(
        SELECT DISTINCT value
        FROM unnest(auth_sessions.scopes || ARRAY['mail:read', 'mail:write']::TEXT[]) AS value
        ORDER BY value
    )
WHERE app_id IN ('lifetrace-desktop', 'lifetrace-web')
  AND status = 'active'
  AND revoked_at IS NULL;
