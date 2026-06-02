-- Add dep_type column to contract_dependencies for filtering by import/call/data (issue #726)
ALTER TABLE contract_dependencies
  ADD COLUMN IF NOT EXISTS dep_type VARCHAR(20) NOT NULL DEFAULT 'call';

CREATE INDEX IF NOT EXISTS idx_contract_dependencies_dep_type ON contract_dependencies(dep_type);
