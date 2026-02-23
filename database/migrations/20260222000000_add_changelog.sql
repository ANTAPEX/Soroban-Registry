-- Changelog auto-generation and release history

CREATE TYPE changelog_change_type AS ENUM (
    'feat',
    'fix',
    'docs',
    'style',
    'refactor',
    'perf',
    'test',
    'build',
    'ci',
    'chore',
    'revert',
    'breaking'
);

CREATE TABLE contract_changelogs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version         TEXT NOT NULL,
    title           TEXT,
    release_date    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_prerelease   BOOLEAN NOT NULL DEFAULT FALSE,
    markdown        TEXT NOT NULL DEFAULT '',
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (contract_id, version)
);

CREATE TABLE changelog_entries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    changelog_id    UUID NOT NULL REFERENCES contract_changelogs(id) ON DELETE CASCADE,
    change_type     changelog_change_type NOT NULL,
    scope           TEXT,
    description     TEXT NOT NULL,
    commit_hash     TEXT,
    is_breaking     BOOLEAN NOT NULL DEFAULT FALSE,
    author          TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contract_changelogs_contract_id ON contract_changelogs(contract_id);
CREATE INDEX idx_contract_changelogs_version ON contract_changelogs(contract_id, version);
CREATE INDEX idx_changelog_entries_changelog_id ON changelog_entries(changelog_id);
CREATE INDEX idx_changelog_entries_breaking ON changelog_entries(changelog_id) WHERE is_breaking = TRUE;

CREATE TRIGGER set_contract_changelogs_updated_at
    BEFORE UPDATE ON contract_changelogs
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();
