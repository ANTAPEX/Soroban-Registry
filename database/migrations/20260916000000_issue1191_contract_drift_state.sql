-- Migration: 20260916000000_issue1191_contract_drift_state.sql
-- Add three-way WASM drift detection state (Issue #1191)

CREATE TABLE IF NOT EXISTS contract_drift_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id VARCHAR(56) NOT NULL,
    network VARCHAR(32) NOT NULL DEFAULT 'testnet',
    registry_wasm_hash VARCHAR(64) NOT NULL,
    onchain_wasm_hash VARCHAR(64),
    status VARCHAR(32) NOT NULL CHECK (status IN ('match', 'drift', 'not_on_chain', 'unknown')),
    diverged_leg VARCHAR(64),
    first_detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    observed_at_ledger BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, network)
);

CREATE INDEX IF NOT EXISTS idx_contract_drift_state_status ON contract_drift_state(status);
CREATE INDEX IF NOT EXISTS idx_contract_drift_state_contract_id ON contract_drift_state(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_drift_state_network ON contract_drift_state(network);
CREATE INDEX IF NOT EXISTS idx_contract_drift_state_last_checked ON contract_drift_state(last_checked_at);
CREATE INDEX IF NOT EXISTS idx_contract_drift_state_first_detected ON contract_drift_state(first_detected_at);
