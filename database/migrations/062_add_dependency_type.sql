-- `contract_dependencies` is read by the graph/dependency-tree handlers
-- (dependency.rs, dependency_handlers.rs, handlers.rs, interoperability.rs)
-- but no earlier migration ever created it — 006 and 007 created two
-- differently-named, differently-shaped tables instead
-- (contract_static_dependencies, contract_call_dependencies). Create the
-- table the application code actually expects, unifying the columns both
-- read paths need: declared package-style deps (contract_id,
-- dependency_name, dependency_contract_id, version_constraint) and
-- on-chain call-graph edges (caller_id, callee_contract_id, call_volume,
-- is_verified).
CREATE TABLE IF NOT EXISTS contract_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID REFERENCES contracts(id) ON DELETE CASCADE,
    dependency_name VARCHAR(255),
    dependency_contract_id UUID REFERENCES contracts(id),
    version_constraint VARCHAR(100),
    caller_id UUID REFERENCES contracts(id) ON DELETE CASCADE,
    callee_contract_id VARCHAR(56),
    call_volume INT NOT NULL DEFAULT 0,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, dependency_name)
);

CREATE INDEX IF NOT EXISTS idx_contract_dependencies_contract_id ON contract_dependencies(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_dependencies_dependency_contract_id ON contract_dependencies(dependency_contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_dependencies_caller_id ON contract_dependencies(caller_id);
CREATE INDEX IF NOT EXISTS idx_contract_dependencies_callee_contract_id ON contract_dependencies(callee_contract_id);

CREATE TRIGGER update_contract_dependencies_updated_at BEFORE UPDATE ON contract_dependencies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add dep_type column to contract_dependencies for filtering by import/call/data (issue #726)
ALTER TABLE contract_dependencies
  ADD COLUMN IF NOT EXISTS dep_type VARCHAR(20) NOT NULL DEFAULT 'call';

CREATE INDEX IF NOT EXISTS idx_contract_dependencies_dep_type ON contract_dependencies(dep_type);
