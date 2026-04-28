# Database Indexing Strategy

This document outlines the indexing strategy for the Soroban Registry to ensure consistent performance, scalability, and maintainable database schema.

## 1. Indexing Philosophy

Indexes are essential for read performance but come with a cost in terms of disk space and write latency (INSERT/UPDATE/DELETE). Our strategy focuses on:
*   **Covering common filter paths**: Identifying the most frequent `WHERE` clauses.
*   **Optimizing sort orders**: Ensuring `ORDER BY` columns are indexed when combined with filters.
*   **Minimizing index bloat**: Avoiding redundant or overlapping indexes.

## 2. Naming Conventions

All indexes must follow the `idx_<table_name>_<column_names>` format:
*   **Single-column**: `idx_contracts_network`
*   **Composite**: `idx_contracts_network_category`
*   **Partial**: `idx_tags_trending` (include a suffix if it clarifies the condition)
*   **GIN (JSONB/FTS)**: `idx_contracts_name_search`

## 3. Core Patterns

### 3.1 Single-Column B-tree Indexes
Standard for primary lookups and foreign keys.
*   **Example**: `idx_publishers_stellar_address`
*   **Usage**: Equality checks (`=`) and range queries (`>`, `<`).

### 3.2 Composite B-tree Indexes
Used for queries that filter on multiple columns simultaneously.
*   **Example**: `idx_contracts_network_category`
*   **Important**: Column order matters. Place the column with the highest selectivity (or the one most frequently used alone) first.

### 3.3 Partial Indexes
Ideal for sparse data or status flags.
*   **Example**: `CREATE INDEX idx_contracts_verified_only ON contracts(created_at DESC) WHERE is_verified = TRUE;`
*   **Benefit**: Significant reduction in index size and maintenance cost.

### 3.4 GIN Indexes
Used for unstructured data and full-text search.
*   **JSONB**: `idx_contract_events_data` for containment (`@>`) queries.
*   **Full-Text Search**: `idx_contracts_name_search` for `tsvector` lookups.

## 4. Best Practices for Migrations

### 4.1 Non-Blocking Creation
When adding indexes to large tables (e.g., `contracts`, `contract_interactions`), always use `CONCURRENTLY`.
```sql
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_large_table_col ON large_table(col);
```
*Note: `CONCURRENTLY` cannot be used inside a transaction.*

### 4.2 Idempotency
Always use `IF NOT EXISTS` to ensure migrations can be re-run safely.

### 4.3 Benchmarking
Before adding a new index, use `EXPLAIN ANALYZE` to confirm it actually improves the targeted query.

## 5. Current Strategy Focus

As of `2026-04-28`, we have optimized the following hot paths:
1.  **Network-based filtering**: Essential for the Indexer and cross-network contract resolution.
2.  **Category-based discovery**: Core for the frontend explore page and analytics.
3.  **Composite filtering**: Optimized combined (network + category) lookups for aggregated views.
4.  **Metadata History**: Indexed category in `contract_metadata_versions` to speed up administrative and audit lookups.
