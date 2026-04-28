# Soroban Registry — Database Migrations

> **Audience:** Backend engineers adding schema changes, reviewers, and DevOps running deployments.  
> **Tool:** [`sqlx-cli`](https://github.com/launchbadge/sqlx) — plain SQL migrations, no ORM  
> **Location:** `database/migrations/`

---

## Table of Contents

1. [How It Works](#1-how-it-works)
2. [Setup](#2-setup)
3. [Naming Conventions](#3-naming-conventions)
4. [Creating a New Migration](#4-creating-a-new-migration)
5. [Applying Migrations](#5-applying-migrations)
6. [Rolling Back Migrations](#6-rolling-back-migrations)
7. [Testing Migrations Locally](#7-testing-migrations-locally)
8. [Migration Checklist for PRs](#8-migration-checklist-for-prs)
9. [Common Migration Examples](#9-common-migration-examples)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. How It Works

`sqlx` tracks applied migrations in a `_sqlx_migrations` table (created automatically on first run):

```sql
SELECT * FROM _sqlx_migrations ORDER BY installed_on;
```

Each migration file has a **checksum** stored at apply time. If a file is modified after being applied, `sqlx` will refuse to run subsequent migrations and report a checksum mismatch — protecting against silent schema drift.

```mermaid
flowchart LR
    A["New .sql file\nadded to database/migrations/"] -->|sqlx migrate run| B{"Already applied?"}
    B -- No --> C[Execute SQL\nRecord in _sqlx_migrations]
    B -- Yes --> D[Skip — idempotent]
    C --> E[Next unapplied file]
    E --> B
```

---

## 2. Setup

### Install `sqlx-cli`

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

### Set database URL

```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry"
```

Or store it in a local `.env` file — `sqlx-cli` reads it automatically:

```dotenv
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/soroban_registry
```

### Verify connection

```bash
sqlx database ping
```

---

## 3. Naming Conventions

All migration files live in `database/migrations/`. Two naming formats are accepted:

### Sequential (preferred for main branch)

```
NNN_short_description.sql
```

| Part | Rule |
|---|---|
| `NNN` | Zero-padded three-digit integer, incremented from the last file. Check existing files before picking a number. **MUST be unique** — duplicates cause non-deterministic ordering. |
| `_` | Literal underscore separator |
| `short_description` | lowercase, words separated by underscores, ≤ 40 chars total |

> ⚠️ **CRITICAL:** Migration prefixes MUST be unique. Duplicate prefixes create non-deterministic ordering based on alphabetical tiebreakers, which can cause:
> - Schema inconsistencies between environments
> - Migration checksum mismatches
> - Failed migrations due to dependency order issues
>
> A CI check validates prefix uniqueness on every pull request.

**Examples:**

```
001_initial.sql
026_full_text_search.sql
035_add_database_performance_indexes.sql
```

### Timestamp (for concurrent feature branches)

When multiple engineers are working on separate branches simultaneously, use timestamps to avoid numbering conflicts:

```
YYYYMMDDHHMMSS_short_description.sql
```

**Examples:**

```
20260221000000_add_custom_metrics.sql
20260223000000_add_disaster_recovery_tables.sql
```

> **Rule:** Always check `database/migrations/` for the highest existing sequential number **before** creating a new one. If in doubt, use the timestamp format on your branch and let it be renumbered during merge-to-main review.

### Consolidation Rules

When cleaning up old migrations, keep the history deterministic:

1. Keep one canonical file for each schema concern and remove exact duplicate migrations.
2. Renumber distinct migrations with duplicate prefixes instead of relying on filename tiebreakers.
3. Prefer additive follow-up migrations for already-shipped databases; only consolidate applied files before a release cut or with an explicit migration plan.
4. Run `.github/scripts/validate-migrations.sh` before opening a PR.
5. Add or verify indexes for every new high-cardinality filter, join key, and time-series lookup.

---

## 4. Creating a New Migration

### Step 1 — Determine the next number

```bash
ls database/migrations/ | sort | tail -5
```

Note the highest `NNN` value and increment by one.

### Step 2 — Create the file

```bash
# Sequential example
touch database/migrations/039_add_publisher_verification.sql

# Or use the timestamp format on a feature branch
touch database/migrations/20260224120000_add_publisher_verification.sql
```

### Step 3 — Write the SQL

Open the file and write your migration. Follow these rules:

1. **One concern per file.** Don't mix unrelated changes.
2. **Idempotent DDL.** Use `IF NOT EXISTS` / `IF EXISTS` everywhere possible.
3. **Comment your intent.** A short comment at the top explaining _why_ the change was made is mandatory.
4. **Never modify existing applied migrations.** Create a new file instead.
5. **Always add indexes with `CREATE INDEX CONCURRENTLY`** when adding to large tables (requires running outside a transaction).

**Template:**

```sql
-- Migration: NNN_short_description.sql
-- Purpose: <one sentence why this migration exists>
-- Issue: #<issue number if applicable>

-- ── 1. New types (if any) ─────────────────────────────────────────────────
-- (ENUM additions must be outside a transaction in PostgreSQL 11+)

-- ── 2. Schema changes ─────────────────────────────────────────────────────

-- ── 3. Indexes ────────────────────────────────────────────────────────────

-- ── 4. Data backfills / seed data (if any) ────────────────────────────────
```

### Step 4 — Validate offline (optional but recommended)

Check that the SQL parses correctly before applying it:

```bash
sqlx migrate info --source database/migrations
```

### Step 5 — Apply locally

```bash
sqlx migrate run --source database/migrations
```

---

## 5. Applying Migrations

### Development

```bash
sqlx migrate run --source database/migrations
```

### Docker Compose

```bash
# One-off container that exits after migrations complete
docker compose run --rm api sqlx migrate run --source /migrations
```

### CI / CD

Migrations are applied automatically before the test suite runs. See `.github/workflows/` for the exact step. The typical pattern:

```bash
sqlx migrate run --source database/migrations
cargo test
```

### Check migration status

```bash
# Lists every migration and whether it has been applied
sqlx migrate info --source database/migrations
```

Example output:

```
Version         Description                          Installed On            Checksum
20231201000000  initial                              2024-01-15 10:00:00     abc123...
20231210000000  add_abi                              2024-01-15 10:00:01     def456...
pending         039_add_publisher_verification       —                       —
```

---

## 6. Rolling Back Migrations

`sqlx` does **not** support automatic down-migrations. The project follows a **forward-only** migration strategy: to undo a change, write a new migration that reverses it.

### Why forward-only?

- Down migrations are error-prone and rarely tested.
- Production databases almost never allow running backwards.
- PostgreSQL DDL is transactional — a failed migration leaves no partial state.

### Emergency rollback pattern

If a bad migration reaches a shared environment:

```sql
-- In a new migration file, example: 039_revert_publisher_verification.sql

-- Revert: undo the changes made in 039_add_publisher_verification.sql

ALTER TABLE publishers DROP COLUMN IF EXISTS verification_level;
DROP INDEX IF EXISTS idx_publishers_verification_level;
```

Then apply it:

```bash
sqlx migrate run --source database/migrations
```

### Local development only — `revert`

`sqlx migrate revert` **is** available for local development. It runs the last applied migration's conceptual revert — but since this project only maintains forward migrations, you should only use this during local development **before** pushing your branch:

```bash
# Reverts only the most recently applied migration (local dev only)
sqlx migrate revert --source database/migrations
```

> **Never run `sqlx migrate revert` in staging or production.**

---

## 7. Testing Migrations Locally

### 7.1 Full clean-run test

The safest test: destroy the local database and apply all migrations from scratch.

```bash
# Drop and recreate
dropdb soroban_registry && createdb soroban_registry

# Apply all migrations
sqlx migrate run --source database/migrations

# Verify the API starts without errors
cargo run --bin api
```

### 7.2 Incremental test (for a new migration file)

```bash
# Apply only the new file against an already-migrated database
sqlx migrate run --source database/migrations

# Confirm the new table/column exists
psql $DATABASE_URL -c "\d your_new_table"
```

### 7.3 Test with Docker Compose

```bash
# Bring up a clean Postgres container
docker compose up -d postgres

# Run migrations
docker compose run --rm api sqlx migrate run

# Seed test data
cargo run --bin seeder -- --count=20 --seed=42

# Run integration tests
cargo test --workspace
```

### 7.4 Validate SQLx query macros

After any schema change, offline query metadata must be regenerated so compile-time query checking stays accurate:

```bash
# Regenerate .sqlx/ offline cache
cargo sqlx prepare --workspace -- --all-targets

# Commit the updated .sqlx/ directory
git add .sqlx/
```

---

## 8. Migration Checklist for PRs

Before requesting review on a PR that includes a migration, confirm every item below:

### Schema changes

- [ ] File placed in `database/migrations/` with the correct naming convention
- [ ] File number / timestamp is unique and higher than all existing files
- [ ] Existing applied migration files were **not** modified
- [ ] `IF NOT EXISTS` / `IF EXISTS` guards used on all DDL
- [ ] A comment at the top of the file describes the purpose

### Indexes

- [ ] Every new foreign key column has an index
- [ ] Large table index additions use `CONCURRENTLY` (outside transaction)
- [ ] No duplicate indexes created (check existing `CREATE INDEX` statements)

### Data integrity

- [ ] New `NOT NULL` columns either have a `DEFAULT` value or a backfill step
- [ ] Foreign keys specify `ON DELETE` behaviour explicitly
- [ ] `CHECK` constraints added where business rules apply

### Testing

- [ ] Clean-slate apply tested locally (`dropdb` → `createdb` → `migrate run`)
- [ ] `sqlx prepare --workspace` run and `.sqlx/` cache committed
- [ ] Existing integration tests still pass
- [ ] Seeder still runs without errors (`cargo run --bin seeder -- --count=10`)

### Review

- [ ] Migration is **reversible** — a forward revert migration exists or one is not needed
- [ ] No sensitive data hard-coded in the migration file
- [ ] Performance implications considered for large tables (use `CONCURRENTLY`, batch updates)

---

## 9. Common Migration Examples

### 9.1 Add a nullable column

```sql
-- Migration: 039_add_homepage_url_to_publishers.sql
-- Purpose: Allow publishers to specify a homepage URL

ALTER TABLE publishers
    ADD COLUMN IF NOT EXISTS homepage_url VARCHAR(500);
```

### 9.2 Add a NOT NULL column with a default

```sql
-- Migration: 040_add_download_count_to_contracts.sql
-- Purpose: Track cumulative download count per contract

ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS download_count BIGINT NOT NULL DEFAULT 0;

-- Backfill from existing interaction records (non-blocking read)
UPDATE contracts c
SET download_count = (
    SELECT COUNT(*) FROM contract_interactions ci
    WHERE ci.contract_id = c.id
      AND ci.interaction_type = 'download'
);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_contracts_download_count
    ON contracts(download_count DESC);
```

### 9.3 Add a new table with foreign keys

```sql
-- Migration: 041_add_contract_bookmarks.sql
-- Purpose: Let users bookmark contracts for quick access

CREATE TABLE IF NOT EXISTS contract_bookmarks (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID  NOT NULL,
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, contract_id)
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_user_id     ON contract_bookmarks(user_id);
CREATE INDEX IF NOT EXISTS idx_bookmarks_contract_id ON contract_bookmarks(contract_id);
```

### 9.4 Add a new ENUM value

PostgreSQL allows adding values to an existing ENUM, but **not inside a transaction**. Use `DO $$ BEGIN ... EXCEPTION ... END $$` to make it idempotent:

```sql
-- Migration: 042_add_deprecated_to_maturity_levels.sql
-- Purpose: New maturity level for explicitly deprecated contracts

DO $$ BEGIN
    ALTER TYPE maturity_level ADD VALUE IF NOT EXISTS 'deprecated' AFTER 'legacy';
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;
```

### 9.5 Add a GIN index for JSONB containment queries

```sql
-- Migration: 043_add_gin_index_on_build_params.sql
-- Purpose: Speed up queries filtering by compiler flags inside build_params

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_build_params
    ON verifications USING GIN (build_params);
```

### 9.6 Rename a column safely

PostgreSQL supports `RENAME COLUMN` without data rewrite. Keep the old column as an alias (view or generated column) if the API still references it:

```sql
-- Migration: 044_rename_wasm_hash_to_bytecode_hash.sql
-- Purpose: Align column name with industry terminology

ALTER TABLE contracts
    RENAME COLUMN wasm_hash TO bytecode_hash;

-- Add a generated alias so existing queries keep working during transition
ALTER TABLE contracts
    ADD COLUMN wasm_hash VARCHAR(64) GENERATED ALWAYS AS (bytecode_hash) STORED;
```

### 9.7 Create a partial index

Partial indexes are smaller and faster for queries with a fixed filter condition:

```sql
-- Migration: 045_add_partial_index_verified_contracts.sql
-- Purpose: Speed up "list all verified contracts" endpoint

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_contracts_verified_only
    ON contracts(created_at DESC)
    WHERE is_verified = TRUE;
```

### 9.8 Full-text search column

```sql
-- Migration: 046_add_fts_to_publishers.sql
-- Purpose: Enable full-text search on publisher usernames

ALTER TABLE publishers
    ADD COLUMN IF NOT EXISTS username_search tsvector
        GENERATED ALWAYS AS (
            to_tsvector('english', COALESCE(username, ''))
        ) STORED;

CREATE INDEX IF NOT EXISTS idx_publishers_username_search
    ON publishers USING GIN (username_search);
```

### 9.9 Forward revert (undo a previous migration)

```sql
-- Migration: 047_revert_homepage_url_from_publishers.sql
-- Purpose: Removing homepage_url — not used by any feature

ALTER TABLE publishers DROP COLUMN IF EXISTS homepage_url;
```

---

## 10. Troubleshooting

### Checksum mismatch

```
error: migration 001/migrate/initial was previously applied but has been modified
```

**Cause:** A migration file that was already applied to the database has been edited.  
**Fix:** Never edit applied migrations. If the change is needed, create a new migration file.

```bash
# Confirm what's in the tracking table
psql $DATABASE_URL -c "SELECT version, description, checksum FROM _sqlx_migrations;"
```

### Migration takes too long (large table DDL)

Adding indexes on large tables within a regular transaction blocks reads. Use `CONCURRENTLY`:

```sql
-- Bad (blocks reads/writes on large table)
CREATE INDEX idx_contracts_foo ON contracts(foo);

-- Good (non-blocking, runs outside transaction)
CREATE INDEX CONCURRENTLY idx_contracts_foo ON contracts(foo);
```

Note: `CREATE INDEX CONCURRENTLY` cannot run inside an explicit `BEGIN ... COMMIT` block.

### `DATABASE_URL` not set

```
error: no DATABASE_URL set
```

```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry"
# or
echo 'DATABASE_URL=postgresql://...' >> .env
```

### sqlx offline mode errors in CI

If CI fails with `error: sqlx query data not found`, the `.sqlx/` cache is stale:

```bash
# Regenerate against a running database
cargo sqlx prepare --workspace -- --all-targets
git add .sqlx/
git commit -m "chore: regenerate sqlx query cache"
```

### Apply hangs / connection refused

```bash
# Check Postgres is running
pg_isready -h localhost -p 5432

# Docker
docker compose ps postgres
docker compose logs postgres
```

---

## 11. Migration Prefix Validation

### Automated CI Check

A GitHub Actions workflow validates migration file naming on every pull request:

**Script Location:** `.github/scripts/validate-migrations.sh`

**Validations performed:**
1. All migration prefixes are unique
2. Prefixes are 3-digit zero-padded numbers
3. Filenames follow naming convention (NNN_description.sql or YYYYMMDDHHMMSS_description.sql)
4. Warning for gaps in sequential numbering

### Running Locally

```bash
# Validate migrations before committing
bash .github/scripts/validate-migrations.sh
```

**Example output:**
```
✅ All migration prefixes are unique
✅ All prefixes are properly formatted
⚠️  Gap detected: 012 → 014 (013 missing)
✅ All migration filenames follow conventions
```

### Migration History - Fixed Duplicate Prefixes

The following migrations were renumbered to fix duplicate prefix issues (Issue #319):

| Old Filename | New Filename | Reason | Date Fixed |
|--------------|--------------|--------|------------|
| `003_indexer_state.sql` | `046_indexer_state.sql` | Duplicate prefix 003 (conflicted with `003_analytics.sql`) | 2026-02-25 |
| `004_blue_green_deployments.sql` | `047_blue_green_deployments.sql` | Duplicate prefix 004 (conflicted with `004_add_upgrade_strategy.sql`) | 2026-02-25 |
| `005_state_schema_and_migration_scripts.sql` | `048_state_schema_and_migration_scripts.sql` | Duplicate prefix 005 (conflicted with `005_create_migrations_table.sql`) | 2026-02-25 |
| `038_contract_compatibility_testing.sql` | `049_contract_compatibility_testing.sql` | Duplicate prefix 038 (conflicted with `038_add_health_score.sql`) | 2026-02-25 |

**Note:** These renames do not affect the database schema or data — they only fix the file ordering to be deterministic. The migrations themselves remain unchanged.
