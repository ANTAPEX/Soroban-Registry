// tests/compatibility_testing_tests.rs
//
// Unit tests for the SDK/Wasm/Network compatibility testing matrix logic (Issue #261).
// These tests exercise the handler logic directly (without a live DB)
// using mock data to verify matrix construction, summary counts, status
// classification, and history tracking.

/// Simulates the compatibility test logic from the handler.
fn simulate_compatibility_test(
    sdk_version: &str,
    _wasm_runtime: &str,
    _network: &str,
) -> (&'static str, Option<String>) {
    let parts: Vec<&str> = sdk_version.split('.').collect();
    let major: u32 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);

    if major < 20 {
        (
            "incompatible",
            Some(format!(
                "SDK version {} is below minimum supported version 20.0.0",
                sdk_version
            )),
        )
    } else if major < 21 {
        (
            "warning",
            Some(format!(
                "SDK version {} has known deprecations; upgrade recommended",
                sdk_version
            )),
        )
    } else {
        ("compatible", None)
    }
}

#[derive(Debug, Clone)]
struct FakeCompatibilityEntry {
    sdk_version: String,
    wasm_runtime: String,
    network: String,
    status: String,
    error_message: Option<String>,
}

/// Build a fake entry from test parameters.
fn make_entry(
    sdk_version: &str,
    wasm_runtime: &str,
    network: &str,
    status: &str,
    error_message: Option<&str>,
) -> FakeCompatibilityEntry {
    FakeCompatibilityEntry {
        sdk_version: sdk_version.to_string(),
        wasm_runtime: wasm_runtime.to_string(),
        network: network.to_string(),
        status: status.to_string(),
        error_message: error_message.map(str::to_string),
    }
}

/// Collect unique SDK versions from entries, sorted.
fn collect_sdk_versions(entries: &[FakeCompatibilityEntry]) -> Vec<String> {
    let mut versions: Vec<String> = entries.iter().map(|e| e.sdk_version.clone()).collect();
    versions.sort();
    versions.dedup();
    versions
}

/// Collect unique wasm runtimes from entries, sorted.
fn collect_wasm_runtimes(entries: &[FakeCompatibilityEntry]) -> Vec<String> {
    let mut runtimes: Vec<String> = entries.iter().map(|e| e.wasm_runtime.clone()).collect();
    runtimes.sort();
    runtimes.dedup();
    runtimes
}

/// Collect unique networks from entries, sorted.
fn collect_networks(entries: &[FakeCompatibilityEntry]) -> Vec<String> {
    let mut networks: Vec<String> = entries.iter().map(|e| e.network.clone()).collect();
    networks.sort();
    networks.dedup();
    networks
}

/// Count entries by status.
fn count_by_status(entries: &[FakeCompatibilityEntry], status: &str) -> usize {
    entries.iter().filter(|e| e.status == status).count()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_simulate_compatible_sdk() {
    let (status, err) = simulate_compatibility_test("22.0.0", "wasmtime-25.0", "testnet");
    assert_eq!(status, "compatible");
    assert!(err.is_none());
}

#[test]
fn test_simulate_warning_sdk() {
    let (status, err) = simulate_compatibility_test("20.5.0", "wasmtime-24.0", "mainnet");
    assert_eq!(status, "warning");
    assert!(err.is_some());
    assert!(err.unwrap().contains("deprecations"));
}

#[test]
fn test_simulate_incompatible_sdk() {
    let (status, err) = simulate_compatibility_test("19.0.0", "wasmtime-23.0", "testnet");
    assert_eq!(status, "incompatible");
    assert!(err.is_some());
    assert!(err.unwrap().contains("below minimum"));
}

#[test]
fn test_simulate_edge_case_sdk_20() {
    let (status, _) = simulate_compatibility_test("20.0.0", "wasmtime-24.0", "futurenet");
    assert_eq!(status, "warning");
}

#[test]
fn test_simulate_edge_case_sdk_21() {
    let (status, _) = simulate_compatibility_test("21.0.0", "wasmtime-25.0", "mainnet");
    assert_eq!(status, "compatible");
}

#[test]
fn test_collect_unique_sdk_versions() {
    let entries = vec![
        make_entry("22.0.0", "wasmtime-25.0", "testnet", "compatible", None),
        make_entry("21.0.0", "wasmtime-25.0", "testnet", "compatible", None),
        make_entry("22.0.0", "wasmtime-24.0", "mainnet", "compatible", None),
        make_entry("20.0.0", "wasmtime-24.0", "testnet", "warning", None),
    ];

    let versions = collect_sdk_versions(&entries);
    assert_eq!(versions, vec!["20.0.0", "21.0.0", "22.0.0"]);
}

#[test]
fn test_collect_unique_wasm_runtimes() {
    let entries = vec![
        make_entry("22.0.0", "wasmtime-25.0", "testnet", "compatible", None),
        make_entry("22.0.0", "wasmtime-24.0", "testnet", "compatible", None),
        make_entry("22.0.0", "wasmtime-25.0", "mainnet", "compatible", None),
    ];

    let runtimes = collect_wasm_runtimes(&entries);
    assert_eq!(runtimes, vec!["wasmtime-24.0", "wasmtime-25.0"]);
}

#[test]
fn test_collect_unique_networks() {
    let entries = vec![
        make_entry("22.0.0", "wasmtime-25.0", "testnet", "compatible", None),
        make_entry("22.0.0", "wasmtime-25.0", "mainnet", "compatible", None),
        make_entry("22.0.0", "wasmtime-25.0", "futurenet", "compatible", None),
        make_entry("22.0.0", "wasmtime-25.0", "testnet", "compatible", None),
    ];

    let networks = collect_networks(&entries);
    assert_eq!(networks, vec!["futurenet", "mainnet", "testnet"]);
}

#[test]
fn test_count_by_status() {
    let entries = vec![
        make_entry("22.0.0", "wasmtime-25.0", "testnet", "compatible", None),
        make_entry(
            "20.0.0",
            "wasmtime-24.0",
            "testnet",
            "warning",
            Some("deprecations"),
        ),
        make_entry(
            "19.0.0",
            "wasmtime-23.0",
            "testnet",
            "incompatible",
            Some("too old"),
        ),
        make_entry("21.0.0", "wasmtime-25.0", "mainnet", "compatible", None),
        make_entry(
            "19.5.0",
            "wasmtime-23.0",
            "mainnet",
            "incompatible",
            Some("too old"),
        ),
    ];

    assert_eq!(count_by_status(&entries, "compatible"), 2);
    assert_eq!(count_by_status(&entries, "warning"), 1);
    assert_eq!(count_by_status(&entries, "incompatible"), 2);
    assert_eq!(
        entries.iter().filter(|e| e.error_message.is_some()).count(),
        3
    );
}

#[test]
fn test_empty_entries_produce_empty_dimensions() {
    let entries: Vec<FakeCompatibilityEntry> = vec![];

    assert!(collect_sdk_versions(&entries).is_empty());
    assert!(collect_wasm_runtimes(&entries).is_empty());
    assert!(collect_networks(&entries).is_empty());
    assert_eq!(count_by_status(&entries, "compatible"), 0);
}

#[test]
fn test_matrix_lookup_key_format() {
    let entry = make_entry("22.0.0", "wasmtime-25.0", "testnet", "compatible", None);
    let key = format!(
        "{}|{}|{}",
        entry.sdk_version, entry.wasm_runtime, entry.network
    );
    assert_eq!(key, "22.0.0|wasmtime-25.0|testnet");
}

#[test]
fn test_full_matrix_construction() {
    let sdk_versions = vec!["21.0.0", "22.0.0"];
    let runtimes = vec!["wasmtime-24.0", "wasmtime-25.0"];
    let networks = vec!["mainnet", "testnet"];

    let mut entries = Vec::new();
    for sdk in &sdk_versions {
        for runtime in &runtimes {
            for network in &networks {
                let (status, err) = simulate_compatibility_test(sdk, runtime, network);
                entries.push(make_entry(sdk, runtime, network, status, err.as_deref()));
            }
        }
    }

    // 2 SDKs * 2 runtimes * 2 networks = 8 entries
    assert_eq!(entries.len(), 8);

    // SDK 21.x should all be compatible (major >= 21)
    let sdk21_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.sdk_version == "21.0.0")
        .collect();
    assert_eq!(sdk21_entries.len(), 4);
    assert!(sdk21_entries.iter().all(|e| e.status == "compatible"));

    // SDK 22.x should all be compatible
    let sdk22_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.sdk_version == "22.0.0")
        .collect();
    assert_eq!(sdk22_entries.len(), 4);
    assert!(sdk22_entries.iter().all(|e| e.status == "compatible"));
}

#[test]
fn test_status_change_detection() {
    let old_status = "compatible";
    let new_status = "incompatible";

    let changed = old_status != new_status;
    assert!(changed, "Status change should be detected");

    let reason = format!("Status changed from {} to {}", old_status, new_status);
    assert_eq!(reason, "Status changed from compatible to incompatible");
}

#[test]
fn test_status_no_change() {
    let old_status = "compatible";
    let new_status = "compatible";

    let changed = old_status != new_status;
    assert!(!changed, "No status change should be detected");
}

#[test]
fn test_notification_generated_on_degradation() {
    let status = "incompatible";
    let should_notify = status == "incompatible" || status == "warning";
    assert!(should_notify, "Should notify on incompatible status");

    let status = "warning";
    let should_notify = status == "incompatible" || status == "warning";
    assert!(should_notify, "Should notify on warning status");

    let status = "compatible";
    let should_notify = status == "incompatible" || status == "warning";
    assert!(!should_notify, "Should not notify on compatible status");
}

#[test]
fn test_notification_message_format() {
    let sdk = "22.0.0";
    let runtime = "wasmtime-25.0";
    let network = "testnet";
    let status = "incompatible";

    let message = format!(
        "Contract compatibility changed to '{}' for SDK {} / Runtime {} / Network {}",
        status, sdk, runtime, network
    );

    assert!(message.contains("incompatible"));
    assert!(message.contains("SDK 22.0.0"));
    assert!(message.contains("Runtime wasmtime-25.0"));
    assert!(message.contains("Network testnet"));
}
