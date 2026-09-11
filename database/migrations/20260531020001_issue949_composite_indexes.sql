-- Migration for #949 Composite Index Optimization
--
-- Originally used CREATE INDEX CONCURRENTLY, which Postgres refuses to run
-- inside a transaction block — sqlx wraps every migration in one, so this
-- always failed. Dropped CONCURRENTLY; fine for a migration-time index build.

-- (network, category)
CREATE INDEX IF NOT EXISTS idx_contracts_network_category ON contracts (network, category);

-- (network, verified)
CREATE INDEX IF NOT EXISTS idx_contracts_network_verified ON contracts (network, is_verified);

-- (network, created_at DESC)
CREATE INDEX IF NOT EXISTS idx_contracts_network_created_at ON contracts (network, created_at DESC);

-- (network, updated_at DESC)
CREATE INDEX IF NOT EXISTS idx_contracts_network_updated_at ON contracts (network, updated_at DESC);

-- (category, verified)
CREATE INDEX IF NOT EXISTS idx_contracts_category_verified ON contracts (category, is_verified);

-- (publisher_id, created_at DESC)
CREATE INDEX IF NOT EXISTS idx_contracts_publisher_created_at ON contracts (publisher_id, created_at DESC);
