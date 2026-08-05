-- EPIC-04 registration uses ON CONFLICT (email_normalized) for atomic
-- duplicate-account prevention. PostgreSQL cannot infer a partial unique
-- index unless the statement repeats its predicate. A regular unique index
-- still permits multiple NULL values, preserving legacy/cloud placeholder
-- users while making the conflict target unambiguous.

DROP INDEX IF EXISTS idx_cloud_users_email_normalized;

CREATE UNIQUE INDEX idx_cloud_users_email_normalized
ON cloud_users(email_normalized);
