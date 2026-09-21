// tests/usage_count_migration_tests.rs
//
// Unit tests for the usage_count migration.
// Tests validate SQL syntax, constraint behavior, and index creation

use std::path::PathBuf;

/// Resolve the migration by its name suffix rather than its full filename.
///
/// The numeric version prefix is not stable: these migrations have already been
/// renumbered once to resolve duplicate version numbers, which moved this file
/// from 20260427000000 to 20260427000002 and left the hardcoded path here
/// pointing at an unrelated migration. Matching on the suffix survives the next
/// renumbering.
fn migration_path() -> PathBuf {
    const SUFFIX: &str = "_add_usage_count.sql";
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../database/migrations")
        .canonicalize()
        .expect("migrations directory should exist");

    let mut matches: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("migrations directory should be readable")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(SUFFIX))
        })
        .collect();
    matches.sort();

    assert_eq!(
        matches.len(),
        1,
        "expected exactly one migration ending in {SUFFIX} under {}, found {matches:?}",
        dir.display()
    );
    matches.remove(0)
}

#[test]
fn test_migration_file_exists() {
    let path = migration_path();
    assert!(path.exists(), "Migration file should exist at {path:?}");
}

#[test]
fn test_migration_contains_required_elements() {
    let migration_path = migration_path();
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    // Check for required SQL statements
    assert!(
        content.contains("ADD COLUMN usage_count BIGINT NOT NULL DEFAULT 0"),
        "Migration should add usage_count column with correct type and default"
    );

    assert!(
        content.contains("CHECK (usage_count >= 0)"),
        "Migration should add non-negative constraint"
    );

    assert!(
        content.contains("CREATE INDEX idx_contracts_usage_count ON contracts(usage_count DESC)"),
        "Migration should create index for efficient queries"
    );
}

#[test]
fn test_migration_sql_syntax() {
    let migration_path = migration_path();
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    // Basic SQL syntax validation
    assert!(
        content.contains("ALTER TABLE contracts"),
        "Should modify contracts table"
    );
    assert!(content.contains("ADD CONSTRAINT"), "Should add constraint");
    assert!(!content.contains("DROP"), "Should not drop anything");
    assert!(!content.contains("DELETE"), "Should not delete data");
}

#[test]
fn test_constraint_name_follows_convention() {
    let migration_path = migration_path();
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    assert!(
        content.contains("chk_contracts_usage_count_non_negative"),
        "Constraint should follow naming convention"
    );
}

#[test]
fn test_index_name_follows_convention() {
    let migration_path = migration_path();
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    assert!(
        content.contains("idx_contracts_usage_count"),
        "Index should follow naming convention"
    );
}
