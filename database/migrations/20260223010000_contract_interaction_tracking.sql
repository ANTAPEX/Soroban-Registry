-- Issue #250: Contract interaction tracking, daily aggregation, and archival

-- 1) Expand contract_interactions to include explicit analytics dimensions.
ALTER TABLE contract_interactions
    ADD COLUMN IF NOT EXISTS interaction_timestamp TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS interaction_count BIGINT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS network network_type;

UPDATE contract_interactions
SET interaction_timestamp = created_at
WHERE interaction_timestamp IS NULL;

UPDATE contract_interactions ci
SET network = c.network
FROM contracts c
WHERE ci.contract_id = c.id
  AND ci.network IS NULL;

-- Normalize historical values into the canonical interaction set.
UPDATE contract_interactions
SET interaction_type = 'invoke'
WHERE interaction_type = 'invocation';

UPDATE contract_interactions
SET interaction_type = 'invoke'
WHERE interaction_type NOT IN (
    'deploy',
    'invoke',
    'transfer',
    'query',
    'publish_success',
    'publish_failed'
);

ALTER TABLE contract_interactions
    ALTER COLUMN interaction_timestamp SET NOT NULL,
    ALTER COLUMN network SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'contract_interactions_interaction_type_check'
    ) THEN
        ALTER TABLE contract_interactions
            ADD CONSTRAINT contract_interactions_interaction_type_check
            CHECK (
                interaction_type IN (
                    'deploy',
                    'invoke',
                    'transfer',
                    'query',
                    'publish_success',
                    'publish_failed'
                )
            );
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_contract_interactions_contract_timestamp
    ON contract_interactions(contract_id, interaction_timestamp DESC);

-- 2) Daily interaction rollup table (contract + type + network + day).
CREATE TABLE IF NOT EXISTS contract_interaction_daily_aggregates (
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    interaction_type VARCHAR(50) NOT NULL,
    network network_type NOT NULL,
    day DATE NOT NULL,
    count BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (contract_id, interaction_type, network, day),
    CONSTRAINT contract_interaction_daily_aggregates_type_check
        CHECK (
            interaction_type IN (
                'deploy',
                'invoke',
                'transfer',
                'query',
                'publish_success',
                'publish_failed'
            )
        )
);

CREATE INDEX IF NOT EXISTS idx_contract_interaction_daily_aggregates_contract_day
    ON contract_interaction_daily_aggregates(contract_id, day DESC);

-- 3) Archive table for raw interactions older than retention window.
CREATE TABLE IF NOT EXISTS contract_interactions_archive (
    id UUID PRIMARY KEY,
    contract_id UUID NOT NULL,
    user_address VARCHAR(56),
    interaction_type VARCHAR(50) NOT NULL,
    transaction_hash VARCHAR(64),
    method VARCHAR(128),
    parameters JSONB,
    return_value JSONB,
    interaction_timestamp TIMESTAMPTZ NOT NULL,
    interaction_count BIGINT NOT NULL,
    network network_type NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contract_interactions_archive_contract_timestamp
    ON contract_interactions_archive(contract_id, interaction_timestamp DESC);

-- 4) Helper function: refresh daily aggregates for recent data.
CREATE OR REPLACE FUNCTION refresh_contract_interaction_daily_aggregates(p_days INTEGER DEFAULT 30)
RETURNS BIGINT AS $$
DECLARE
    v_rows BIGINT := 0;
BEGIN
    INSERT INTO contract_interaction_daily_aggregates (
        contract_id,
        interaction_type,
        network,
        day,
        count,
        updated_at
    )
    SELECT
        contract_id,
        interaction_type,
        network,
        DATE(interaction_timestamp) AS day,
        SUM(interaction_count) AS count,
        NOW()
    FROM contract_interactions
    WHERE interaction_timestamp >= NOW() - make_interval(days => GREATEST(p_days, 1))
    GROUP BY contract_id, interaction_type, network, DATE(interaction_timestamp)
    ON CONFLICT (contract_id, interaction_type, network, day)
    DO UPDATE SET
        count = EXCLUDED.count,
        updated_at = NOW();

    GET DIAGNOSTICS v_rows = ROW_COUNT;
    RETURN v_rows;
END;
$$ LANGUAGE plpgsql;

-- 5) Helper function: archive interactions older than retention window.
CREATE OR REPLACE FUNCTION archive_old_contract_interactions(p_retention_days INTEGER DEFAULT 90)
RETURNS BIGINT AS $$
DECLARE
    v_rows BIGINT := 0;
BEGIN
    WITH moved AS (
        INSERT INTO contract_interactions_archive (
            id,
            contract_id,
            user_address,
            interaction_type,
            transaction_hash,
            method,
            parameters,
            return_value,
            interaction_timestamp,
            interaction_count,
            network,
            created_at
        )
        SELECT
            id,
            contract_id,
            user_address,
            interaction_type,
            transaction_hash,
            method,
            parameters,
            return_value,
            interaction_timestamp,
            interaction_count,
            network,
            created_at
        FROM contract_interactions
        WHERE interaction_timestamp < NOW() - make_interval(days => GREATEST(p_retention_days, 1))
        ON CONFLICT (id) DO NOTHING
        RETURNING id
    )
    DELETE FROM contract_interactions ci
    WHERE ci.id IN (SELECT id FROM moved);

    GET DIAGNOSTICS v_rows = ROW_COUNT;
    RETURN v_rows;
END;
$$ LANGUAGE plpgsql;

-- Seed daily aggregates from existing rows.
SELECT refresh_contract_interaction_daily_aggregates(3650);
