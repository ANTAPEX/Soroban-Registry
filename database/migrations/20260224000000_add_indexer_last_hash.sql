-- Add ledger hash tracking to indexer_state table
ALTER TABLE indexer_state ADD COLUMN last_indexed_ledger_hash VARCHAR(64);

-- Comment for clarity
COMMENT ON COLUMN indexer_state.last_indexed_ledger_hash IS 'Hash of the last ledger successfully processed by the indexer';
