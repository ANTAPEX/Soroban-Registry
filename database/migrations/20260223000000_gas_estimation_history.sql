-- Gas estimation history: tracks estimated vs actual gas to measure accuracy
CREATE TABLE IF NOT EXISTS gas_estimation_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    method_name VARCHAR(255) NOT NULL,
    estimated_gas BIGINT NOT NULL,
    actual_gas BIGINT,
    estimated_fee BIGINT NOT NULL,
    actual_fee BIGINT,
    deviation_percent DOUBLE PRECISION,
    cpu_instructions BIGINT NOT NULL DEFAULT 0,
    memory_bytes BIGINT NOT NULL DEFAULT 0,
    read_bytes BIGINT NOT NULL DEFAULT 0,
    write_bytes BIGINT NOT NULL DEFAULT 0,
    network VARCHAR(50) NOT NULL DEFAULT 'testnet',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gas_history_contract ON gas_estimation_history(contract_id);
CREATE INDEX idx_gas_history_method ON gas_estimation_history(contract_id, method_name);
CREATE INDEX idx_gas_history_created ON gas_estimation_history(created_at DESC);
CREATE INDEX idx_gas_history_network ON gas_estimation_history(network);

-- Materialized view for per-method accuracy stats
CREATE MATERIALIZED VIEW IF NOT EXISTS gas_estimation_accuracy AS
SELECT
    contract_id,
    method_name,
    network,
    COUNT(*) AS sample_count,
    AVG(deviation_percent) AS mean_deviation,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY ABS(COALESCE(deviation_percent, 0))) AS p95_deviation,
    COUNT(*) FILTER (WHERE ABS(COALESCE(deviation_percent, 0)) <= 10.0)::FLOAT
        / NULLIF(COUNT(*) FILTER (WHERE deviation_percent IS NOT NULL), 0) AS within_10_pct_ratio,
    MAX(created_at) AS last_updated
FROM gas_estimation_history
WHERE actual_gas IS NOT NULL
GROUP BY contract_id, method_name, network;

CREATE UNIQUE INDEX idx_gas_accuracy_pk
    ON gas_estimation_accuracy(contract_id, method_name, network);
