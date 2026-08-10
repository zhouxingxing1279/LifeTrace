-- LifeTrace EPIC-17 PostgreSQL least-privilege baseline.
--
-- Run this as the migration/owner role AFTER migrations. Create the
-- `lifetrace_app` login separately through your secret manager so no password
-- is committed here. The Cloud DATABASE_URL must use `lifetrace_app`, while
-- schema migrations use a separate owner/migrator credential.

\set ON_ERROR_STOP on

REVOKE CREATE ON SCHEMA public FROM PUBLIC;

GRANT CONNECT ON DATABASE lifetrace TO lifetrace_app;
GRANT USAGE ON SCHEMA public TO lifetrace_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO lifetrace_app;
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO lifetrace_app;

-- Apply the same permissions to tables/sequences created by future migrations.
-- Replace `lifetrace_migrator` below if your migration owner uses a different
-- role name.
ALTER DEFAULT PRIVILEGES FOR ROLE lifetrace_migrator IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO lifetrace_app;
ALTER DEFAULT PRIVILEGES FOR ROLE lifetrace_migrator IN SCHEMA public
    GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO lifetrace_app;

-- The application role must not own schema objects or receive administrative
-- capabilities. These statements are safe to repeat if the role already has
-- the expected defaults.
ALTER ROLE lifetrace_app NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;

-- Operational verification:
-- SELECT current_user, rolsuper, rolcreatedb, rolcreaterole, rolreplication
-- FROM pg_roles WHERE rolname = current_user;
