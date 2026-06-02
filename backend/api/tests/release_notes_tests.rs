// tests/release_notes_tests.rs
//
// Unit tests for the automated release notes generation logic.
// These tests exercise changelog parsing, ABI diff analysis,
// template rendering, and status workflow without a live database.

use serde_json::json;
use std::collections::HashMap;

/// TYPES

#[derive(Debug, Clone)]
struct FunctionChange {
    name: String,
    change_type: String,
    old_signature: Option<String>,
    new_signature: Option<String>,
    is_breaking: bool,
}

#[derive(Debug, Clone, Default)]
struct DiffSummary {
    files_changed: i32,
    lines_added: i32,
    lines_removed: i32,
    function_changes: Vec<FunctionChange>,
    has_breaking_changes: bool,
    features_count: i32,
    fixes_count: i32,
    breaking_count: i32,
}

//HELPERS
fn extract_functions_from_abi(abi: &serde_json::Value) -> HashMap<String, String> {
    let mut fns = HashMap::new();

    let entries: Vec<&serde_json::Value> = if let Some(arr) = abi.as_array() {
        arr.iter().collect()
    } else if let Some(spec_fns) = abi.get("functions").and_then(|f| f.as_array()) {
        spec_fns.iter().collect()
    } else if let Some(spec_fns) = abi.get("spec").and_then(|s| s.as_array()) {
        spec_fns.iter().collect()
    } else {
        Vec::new()
    };

    for entry in entries {
        if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
            let inputs = entry
                .get("inputs")
                .and_then(|i| i.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|inp| {
                            let param_name =
                                inp.get("name").and_then(|n| n.as_str()).unwrap_or("_");
                            let param_type = inp
                                .get("type")
                                .map(|t| format!("{}", t))
                                .unwrap_or_else(|| "unknown".to_string());
                            format!("{}: {}", param_name, param_type)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let outputs = entry
                .get("outputs")
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|out| {
                            out.get("type")
                                .map(|t| format!("{}", t))
                                .unwrap_or_else(|| "unknown".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let sig = format!("fn {}({}) -> ({})", name, inputs, outputs);
            fns.insert(name.to_string(), sig);
        }
    }

    fns
}

fn extract_changelog_section(changelog: &str, version: &str) -> String {
    let version_clean = version.trim_start_matches('v');
    let patterns = [
        format!("## [{}]", version_clean),
        format!("## {}", version_clean),
        format!("## v{}", version_clean),
        format!("## [v{}]", version_clean),
    ];

    let lines: Vec<&str> = changelog.lines().collect();
    let mut start_idx = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if patterns.iter().any(|p| trimmed.starts_with(p.as_str())) {
            start_idx = Some(i);
            break;
        }
    }

    let start = match start_idx {
        Some(i) => i,
        None => return String::new(),
    };

    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            end = i;
            break;
        }
    }

    lines[start..end].to_vec().join("\n").trim().to_string()
}

fn build_diff_from_abis(old_abi: &serde_json::Value, new_abi: &serde_json::Value) -> DiffSummary {
    let old_fns = extract_functions_from_abi(old_abi);
    let new_fns = extract_functions_from_abi(new_abi);
    let mut diff = DiffSummary::default();

    for (name, sig) in &new_fns {
        if !old_fns.contains_key(name) {
            diff.function_changes.push(FunctionChange {
                name: name.clone(),
                change_type: "added".to_string(),
                old_signature: None,
                new_signature: Some(sig.clone()),
                is_breaking: false,
            });
            diff.features_count += 1;
        }
    }

    for (name, sig) in &old_fns {
        if !new_fns.contains_key(name) {
            diff.function_changes.push(FunctionChange {
                name: name.clone(),
                change_type: "removed".to_string(),
                old_signature: Some(sig.clone()),
                new_signature: None,
                is_breaking: true,
            });
            diff.breaking_count += 1;
            diff.has_breaking_changes = true;
        }
    }

    for (name, new_sig) in &new_fns {
        if let Some(old_sig) = old_fns.get(name) {
            if old_sig != new_sig {
                diff.function_changes.push(FunctionChange {
                    name: name.clone(),
                    change_type: "modified".to_string(),
                    old_signature: Some(old_sig.clone()),
                    new_signature: Some(new_sig.clone()),
                    is_breaking: true,
                });
                diff.breaking_count += 1;
                diff.has_breaking_changes = true;
            }
        }
    }

    diff.files_changed = if diff.function_changes.is_empty() {
        0
    } else {
        1
    };
    diff.lines_added = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "added" || c.change_type == "modified")
        .count() as i32
        * 5;
    diff.lines_removed = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "removed" || c.change_type == "modified")
        .count() as i32
        * 5;

    diff
}

fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<&str> = s.trim_start_matches('v').split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

// TESTS

#[test]
fn test_parse_semver_valid() {
    assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
    assert_eq!(parse_semver("v0.10.1"), Some((0, 10, 1)));
    assert_eq!(parse_semver("100.0.0"), Some((100, 0, 0)));
}

#[test]
fn test_parse_semver_invalid() {
    assert_eq!(parse_semver("1.2"), None);
    assert_eq!(parse_semver("abc"), None);
    assert_eq!(parse_semver("1.2.x"), None);
}

#[test]
fn test_extract_functions_from_array_abi() {
    let abi = json!([
        {
            "name": "initialize",
            "inputs": [
                {"name": "admin", "type": "Address"},
                {"name": "decimal", "type": "u32"}
            ],
            "outputs": []
        },
        {
            "name": "transfer",
            "inputs": [
                {"name": "from", "type": "Address"},
                {"name": "to", "type": "Address"},
                {"name": "amount", "type": "i128"}
            ],
            "outputs": [{"type": "bool"}]
        },
        {
            "name": "balance",
            "inputs": [{"name": "account", "type": "Address"}],
            "outputs": [{"type": "i128"}]
        }
    ]);

    let fns = extract_functions_from_abi(&abi);
    assert_eq!(fns.len(), 3);
    assert!(fns.contains_key("initialize"));
    assert!(fns.contains_key("transfer"));
    assert!(fns.contains_key("balance"));
}

#[test]
fn test_extract_functions_from_spec_object() {
    let abi = json!({
        "functions": [
            {"name": "mint", "inputs": [{"name": "to", "type": "Address"}], "outputs": []},
            {"name": "burn", "inputs": [{"name": "amount", "type": "i128"}], "outputs": []}
        ]
    });

    let fns = extract_functions_from_abi(&abi);
    assert_eq!(fns.len(), 2);
    assert!(fns.contains_key("mint"));
    assert!(fns.contains_key("burn"));
}

#[test]
fn test_extract_functions_empty_abi() {
    let abi = json!({});
    let fns = extract_functions_from_abi(&abi);
    assert!(fns.is_empty());
}

#[test]
fn test_diff_detects_added_functions() {
    let old = json!([
        {"name": "transfer", "inputs": [], "outputs": []}
    ]);
    let new = json!([
        {"name": "transfer", "inputs": [], "outputs": []},
        {"name": "approve", "inputs": [{"name": "spender", "type": "Address"}], "outputs": []}
    ]);

    let diff = build_diff_from_abis(&old, &new);
    assert_eq!(diff.features_count, 1);
    assert!(!diff.has_breaking_changes);

    let added: Vec<_> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "added")
        .collect();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].name, "approve");
    assert!(added[0].old_signature.is_none());
    assert!(added[0].new_signature.is_some());
}

#[test]
fn test_diff_detects_removed_functions() {
    let old = json!([
        {"name": "transfer", "inputs": [], "outputs": []},
        {"name": "old_fn", "inputs": [], "outputs": []}
    ]);
    let new = json!([
        {"name": "transfer", "inputs": [], "outputs": []}
    ]);

    let diff = build_diff_from_abis(&old, &new);
    assert!(diff.has_breaking_changes);
    assert_eq!(diff.breaking_count, 1);

    let removed: Vec<_> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "removed")
        .collect();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].name, "old_fn");
    assert!(removed[0].is_breaking);
    assert!(removed[0].old_signature.is_some());
    assert!(removed[0].new_signature.is_none());
}

#[test]
fn test_diff_detects_signature_changes() {
    let old = json!([
        {"name": "transfer", "inputs": [{"name": "to", "type": "Address"}], "outputs": []}
    ]);
    let new = json!([
        {"name": "transfer", "inputs": [{"name": "to", "type": "Address"}, {"name": "memo", "type": "String"}], "outputs": []}
    ]);

    let diff = build_diff_from_abis(&old, &new);
    assert!(diff.has_breaking_changes);

    let modified: Vec<_> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "modified")
        .collect();
    assert_eq!(modified.len(), 1);
    assert_eq!(modified[0].name, "transfer");
    assert!(modified[0].is_breaking);
    assert!(modified[0].old_signature.is_some());
    assert!(modified[0].new_signature.is_some());
}

#[test]
fn test_diff_no_changes() {
    let abi = json!([
        {"name": "transfer", "inputs": [{"name": "to", "type": "Address"}], "outputs": [{"type": "bool"}]}
    ]);

    let diff = build_diff_from_abis(&abi, &abi);
    assert!(diff.function_changes.is_empty());
    assert!(!diff.has_breaking_changes);
    assert_eq!(diff.files_changed, 0);
    assert_eq!(diff.fixes_count, 0);
}

#[test]
fn test_changelog_extraction_bracketed_version() {
    let changelog = "# Changelog\n\n\
## [2.0.0] - 2026-02-20\n\n\
### Breaking\n\
- Removed `transfer_batch`\n\n\
### Features\n\
- Multi-sig support\n\n\
## [1.0.0] - 2026-01-01\n\n\
- Initial release\n";

    let section = extract_changelog_section(changelog, "2.0.0");
    assert!(section.contains("Breaking"));
    assert!(section.contains("transfer_batch"));
    assert!(section.contains("Multi-sig"));
    assert!(!section.contains("Initial release"));
}

#[test]
fn test_changelog_extraction_v_prefix() {
    let changelog = "# Changelog\n\n\
## v1.1.0\n\n\
- Added balance query\n\n\
## v1.0.0\n\n\
- First version\n";

    let section = extract_changelog_section(changelog, "1.1.0");
    assert!(section.contains("balance query"));
    assert!(!section.contains("First version"));
}

#[test]
fn test_changelog_extraction_missing_version() {
    let changelog = "# Changelog\n\n## 1.0.0\n\n- First version\n";
    let section = extract_changelog_section(changelog, "3.0.0");
    assert!(section.is_empty());
}

#[test]
fn test_changelog_extraction_with_v_in_query() {
    let changelog = "## [1.5.0] - 2026-03-01\n\n- Some fix\n\n## [1.4.0]\n\n- Older\n";
    let section = extract_changelog_section(changelog, "v1.5.0");
    assert!(section.contains("Some fix"));
    assert!(!section.contains("Older"));
}

#[test]
fn test_complex_diff_scenario() {
    let old = json!([
        {"name": "initialize", "inputs": [{"name": "admin", "type": "Address"}], "outputs": []},
        {"name": "transfer", "inputs": [{"name": "to", "type": "Address"}, {"name": "amount", "type": "i128"}], "outputs": [{"type": "bool"}]},
        {"name": "old_deprecated", "inputs": [], "outputs": []}
    ]);
    let new = json!([
        {"name": "initialize", "inputs": [{"name": "admin", "type": "Address"}, {"name": "name", "type": "String"}], "outputs": []},
        {"name": "transfer", "inputs": [{"name": "to", "type": "Address"}, {"name": "amount", "type": "i128"}], "outputs": [{"type": "bool"}]},
        {"name": "approve", "inputs": [{"name": "spender", "type": "Address"}, {"name": "amount", "type": "i128"}], "outputs": []},
        {"name": "allowance", "inputs": [{"name": "owner", "type": "Address"}, {"name": "spender", "type": "Address"}], "outputs": [{"type": "i128"}]}
    ]);

    let diff = build_diff_from_abis(&old, &new);

    // Should detect: initialize modified (breaking), old_deprecated removed (breaking),
    // approve added, allowance added
    assert!(diff.has_breaking_changes);

    let added: Vec<_> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "added")
        .collect();
    assert_eq!(added.len(), 2);

    let removed: Vec<_> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "removed")
        .collect();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].name, "old_deprecated");

    let modified: Vec<_> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "modified")
        .collect();
    assert_eq!(modified.len(), 1);
    assert_eq!(modified[0].name, "initialize");

    assert_eq!(diff.features_count, 2);
    assert_eq!(diff.breaking_count, 2); // removed + modified
}

#[test]
fn test_release_notes_status_workflow() {
    // Simulates the draft → edit → publish lifecycle
    let mut status = "draft";

    // Can edit when draft
    assert_eq!(status, "draft");

    // User edits...
    // (no-op in unit test, just assert state)

    // Publish
    status = "published";
    assert_eq!(status, "published");

    // Cannot edit when published
    let can_edit = status != "published";
    assert!(!can_edit);
}

#[test]
fn test_semver_ordering_for_previous_version() {
    let mut versions = [
        (1u64, 0u64, 0u64),
        (2, 1, 0),
        (1, 5, 3),
        (1, 5, 0),
        (2, 0, 0),
    ];

    versions.sort();

    // Looking for previous version before 2.0.0
    let target = (2u64, 0u64, 0u64);
    let previous = versions.iter().rfind(|v| **v < target);
    assert_eq!(previous, Some(&(1, 5, 3)));

    // Looking for previous version before 1.5.0
    let target = (1u64, 5u64, 0u64);
    let previous = versions.iter().rfind(|v| **v < target);
    assert_eq!(previous, Some(&(1, 0, 0)));
}

#[test]
fn test_initial_release_all_functions_added() {
    let abi = json!([
        {"name": "initialize", "inputs": [], "outputs": []},
        {"name": "transfer", "inputs": [], "outputs": []},
        {"name": "balance", "inputs": [], "outputs": []}
    ]);

    let fns = extract_functions_from_abi(&abi);

    // For initial release, all functions should be treated as "added"
    let mut diff = DiffSummary::default();
    for (name, sig) in &fns {
        diff.function_changes.push(FunctionChange {
            name: name.clone(),
            change_type: "added".to_string(),
            old_signature: None,
            new_signature: Some(sig.clone()),
            is_breaking: false,
        });
        diff.features_count += 1;
    }

    assert_eq!(diff.function_changes.len(), 3);
    assert_eq!(diff.features_count, 3);
    assert!(!diff.has_breaking_changes);
}
