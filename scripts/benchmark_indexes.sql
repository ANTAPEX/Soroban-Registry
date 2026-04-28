-- Performance Benchmark for Network and Category Indexes
-- This script uses EXPLAIN ANALYZE to measure actual execution time.

-- 1. Test Category Filtering
\echo '--- Testing Category Filter ---'
EXPLAIN ANALYZE SELECT COUNT(*) FROM contracts WHERE category = 'Defi';

-- 2. Test Network Filtering
\echo '--- Testing Network Filter ---'
EXPLAIN ANALYZE SELECT * FROM contracts WHERE network = 'testnet' AND is_verified = false;

-- 3. Test Composite Filtering (Network + Category)
\echo '--- Testing Composite Filter (Network + Category) ---'
EXPLAIN ANALYZE SELECT * FROM contracts WHERE network = 'mainnet' AND category = 'Stablecoin';

-- 4. Test Metadata History Lookups
\echo '--- Testing Metadata History lookup by Category ---'
EXPLAIN ANALYZE SELECT * FROM contract_metadata_versions WHERE category = 'Oracle' ORDER BY created_at DESC LIMIT 10;

-- 5. Test Interaction Aggregates by Network
\echo '--- Testing Interaction Aggregates by Network ---'
EXPLAIN ANALYZE SELECT network, SUM(deployment_count) 
FROM contract_interaction_daily_aggregates 
WHERE network = 'testnet' 
GROUP BY network;
