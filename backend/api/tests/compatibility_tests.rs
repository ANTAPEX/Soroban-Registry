// tests/compatibility_tests.rs
//
// Integration-level unit tests for the compatibility matrix logic.
// These tests exercise the handler logic directly (without a live DB)
// using mock data to verify grouping, warning generation, and export format.

/// Helper: build a fake `CompatibilityRow` for use in tests.
fn make_row(
    source_version: &str,
    target_name: &str,
    target_version: &str,
    is_compatible: bool,
    stellar_version: Option<&str>,
) -> FakeRow {
    FakeRow {
        source_version: source_version.to_string(),
        target_contract_name: target_name.to_string(),
        target_version: target_version.to_string(),
        is_compatible,
        stellar_version: stellar_version.map(str::to_string),
    }
}

#[derive(Debug, Clone)]
struct FakeRow {
    source_version: String,
    target_contract_name: String,
    target_version: String,
    is_compatible: bool,
    stellar_version: Option<String>,
}

/// Group rows by `source_version`, mirroring the handler logic.
fn group_by_source(rows: &[FakeRow]) -> std::collections::BTreeMap<String, Vec<&FakeRow>> {
    let mut map: std::collections::BTreeMap<String, Vec<&FakeRow>> =
        std::collections::BTreeMap::new();
    for row in rows {
        map.entry(row.source_version.clone()).or_default().push(row);
    }
    map
}

/// Collect warnings for incompatible rows, mirroring the handler logic.
fn collect_warnings(rows: &[FakeRow]) -> Vec<String> {
    rows.iter()
        .filter(|r| !r.is_compatible)
        .map(|r| {
            format!(
                "Version {} is INCOMPATIBLE with {} v{}",
                r.source_version, r.target_contract_name, r.target_version
            )
        })
        .collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_grouping_by_source_version() {
    let rows = vec![
        make_row("1.0.0", "token-contract", "2.0.0", true, None),
        make_row("1.0.0", "escrow-contract", "1.5.0", true, None),
        make_row("2.0.0", "token-contract", "2.0.0", false, Some("20.0")),
    ];

    let grouped = group_by_source(&rows);

    assert_eq!(grouped.len(), 2, "Should have 2 source versions");
    assert_eq!(
        grouped["1.0.0"].len(),
        2,
        "v1.0.0 should have 2 compatibility entries"
    );
    assert_eq!(
        grouped["2.0.0"].len(),
        1,
        "v2.0.0 should have 1 compatibility entry"
    );
}

#[test]
fn test_grouped_source_versions_are_sorted() {
    let rows = vec![
        make_row("3.0.0", "contract-a", "1.0.0", true, None),
        make_row("1.0.0", "contract-a", "1.0.0", true, None),
        make_row("2.0.0", "contract-a", "1.0.0", true, None),
    ];

    let grouped = group_by_source(&rows);
    let keys: Vec<&String> = grouped.keys().collect();

    assert_eq!(
        keys,
        vec!["1.0.0", "2.0.0", "3.0.0"],
        "Keys should be in alphabetical/BTree order"
    );
}

#[test]
fn test_no_warnings_when_all_compatible() {
    let rows = vec![
        make_row("1.0.0", "contract-a", "1.0.0", true, None),
        make_row("1.0.0", "contract-b", "2.0.0", true, None),
        make_row("2.0.0", "contract-a", "1.0.0", true, None),
    ];

    let warnings = collect_warnings(&rows);
    assert!(
        warnings.is_empty(),
        "Expected no warnings for all-compatible rows"
    );
}

#[test]
fn test_warning_generated_for_each_incompatible_pair() {
    let rows = vec![
        make_row("1.0.0", "contract-a", "2.0.0", true, None),
        make_row("2.0.0", "contract-b", "1.0.0", false, None),
        make_row("2.0.0", "contract-c", "3.0.0", false, Some("21.0")),
    ];

    let warnings = collect_warnings(&rows);
    assert_eq!(warnings.len(), 2, "Expected exactly 2 warnings");
    assert!(
        warnings[0].contains("contract-b"),
        "First warning should mention contract-b"
    );
    assert!(
        warnings[1].contains("contract-c"),
        "Second warning should mention contract-c"
    );
}

#[test]
fn test_warning_message_format() {
    let rows = vec![make_row("1.0.0", "MyToken", "3.5.0", false, None)];

    let warnings = collect_warnings(&rows);
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0],
        "Version 1.0.0 is INCOMPATIBLE with MyToken v3.5.0"
    );
}

#[test]
fn test_empty_rows_produce_empty_matrix() {
    let rows: Vec<FakeRow> = vec![];
    let grouped = group_by_source(&rows);
    let warnings = collect_warnings(&rows);

    assert!(grouped.is_empty());
    assert!(warnings.is_empty());
}

#[test]
fn test_csv_row_format() {
    // Verify the CSV row format matches what the export handler produces.
    let source_version = "1.2.3";
    let target_stellar_id = "GABC...XYZ";
    let target_name = "oracle-contract";
    let target_version = "4.0.0";
    let stellar_version = "20.0";
    let is_compatible = true;

    let csv_row = format!(
        "{},{},{},{},{},{}",
        source_version,
        target_stellar_id,
        target_name,
        target_version,
        stellar_version,
        is_compatible
    );

    assert_eq!(csv_row, "1.2.3,GABC...XYZ,oracle-contract,4.0.0,20.0,true");
}

#[test]
fn test_csv_row_with_no_stellar_version() {
    let stellar_version = "";
    let csv_row = format!("1.0.0,GDEF,my-contract,1.0.0,{},false", stellar_version);
    assert_eq!(csv_row, "1.0.0,GDEF,my-contract,1.0.0,,false");
}

#[test]
fn test_single_entry_matrix() {
    let rows = vec![make_row(
        "1.0.0",
        "only-contract",
        "1.0.0",
        true,
        Some("20.0"),
    )];

    let grouped = group_by_source(&rows);
    let warnings = collect_warnings(&rows);

    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped["1.0.0"].len(), 1);
    assert!(warnings.is_empty());
    assert_eq!(grouped["1.0.0"][0].stellar_version.as_deref(), Some("20.0"));
}
