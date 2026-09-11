-- Postgres forbids using a new enum value in the same transaction that adds
-- it. The next migration (20260329093000) needs 'unverified' as a DEFAULT
-- and in backfill UPDATEs, so add the value here in its own migration
-- (sqlx runs each migration in its own transaction) and commit before it's
-- used.
ALTER TYPE verification_status ADD VALUE IF NOT EXISTS 'unverified';
