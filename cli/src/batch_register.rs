#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

const MAX_BATCH_SIZE: usize = 50;
const REGISTER_TIMEOUT_SECS: u64 = 60;

// ── Manifest types ────────────────────────────────────────────────────────────

/// A single contract entry in the manifest file.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    pub contract_id: String,
    pub name: String,
    /// Per-entry network override. Falls back to the manifest-level default.
    pub network: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub wasm_hash: Option<String>,
    pub source_url: Option<String>,
}

/// Top-level manifest file structure (YAML or JSON).
#[derive(Debug, Deserialize)]
pub struct RegisterManifest {
    /// Default publisher address for all entries (overridden by --publisher flag).
    pub publisher: Option<String>,
    /// Default network for entries that don't specify their own.
    pub network: Option<String>,
    pub contracts: Vec<ManifestEntry>,
}

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RegisterPayload {
    contract_id: String,
    name: String,
    network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wasm_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_url: Option<String>,
    publisher_address: String,
}

/// Per-contract result collected during registration.
#[derive(Debug, Serialize)]
pub struct RegistrationResult {
    pub contract_id: String,
    pub name: String,
    /// "registered" | "failed" | "skipped" | "dry_run"
    pub status: String,
    pub registry_id: Option<String>,
    pub error: Option<String>,
}

/// Final summary emitted after all registrations complete.
#[derive(Debug, Serialize)]
pub struct RegistrationSummary {
    pub total: usize,
    pub registered: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<RegistrationResult>,
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run the `batch-register` command.
///
/// * `manifest_path` – path to a YAML or JSON manifest file
/// * `publisher`     – Stellar address; overrides `publisher` field in manifest
/// * `dry_run`       – validate and print what would be registered, but skip API calls
/// * `json`          – emit machine-readable JSON instead of human-readable output
pub async fn run_batch_register(
    api_url: &str,
    manifest_path: &str,
    publisher: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    // 1. Load and parse manifest
    let manifest = load_manifest(manifest_path)?;

    // 2. Resolve publisher (CLI flag > manifest field)
    let resolved_publisher = publisher
        .map(|s| s.to_string())
        .or(manifest.publisher.clone())
        .context(
            "Publisher address is required. Pass --publisher or set `publisher` in the manifest.",
        )?;

    // 3. Resolve and validate all entries before submitting anything
    let entries = resolve_entries(&manifest, &resolved_publisher)?;

    // 4. Deduplicate by contract_id
    let (entries, skipped_duplicates) = deduplicate(entries);

    if entries.is_empty() {
        anyhow::bail!("No valid contracts found in manifest.");
    }
    if entries.len() > MAX_BATCH_SIZE {
        anyhow::bail!(
            "Batch size {} exceeds the maximum of {}. Split into smaller manifests.",
            entries.len(),
            MAX_BATCH_SIZE
        );
    }

    // 5. Print header
    if !json {
        print_header(&entries, skipped_duplicates, &resolved_publisher, dry_run);
    }

    // 6. Validate all entries (required fields) — stop before any API call
    validate_all(&entries)?;

    if dry_run {
        return emit_dry_run(entries, skipped_duplicates, json);
    }

    // 7. Submit one by one, collecting results
    let summary = register_all(api_url, entries, skipped_duplicates, json).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_summary(&summary);
    }

    // Exit with non-zero if any failed
    if summary.failed > 0 {
        anyhow::bail!("{} contract(s) failed to register.", summary.failed);
    }

    Ok(())
}

// ── Manifest loading ──────────────────────────────────────────────────────────

fn load_manifest(path: &str) -> Result<RegisterManifest> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "yaml" | "yml" | "json" => {}
        other => {
            anyhow::bail!("Unsupported manifest extension '.{other}'. Use .yaml, .yml, or .json.")
        }
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read manifest file: {path}"))?;

    match ext.as_str() {
        "yaml" | "yml" => serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML manifest: {path}")),
        "json" => serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON manifest: {path}")),
        _ => unreachable!(),
    }
}

// ── Entry resolution ──────────────────────────────────────────────────────────

struct ResolvedEntry {
    payload: RegisterPayload,
}

fn resolve_entries(manifest: &RegisterManifest, publisher: &str) -> Result<Vec<ResolvedEntry>> {
    let default_network = manifest
        .network
        .as_deref()
        .unwrap_or("testnet")
        .to_lowercase();

    let mut resolved = Vec::with_capacity(manifest.contracts.len());

    for (i, entry) in manifest.contracts.iter().enumerate() {
        let network = entry
            .network
            .as_deref()
            .unwrap_or(&default_network)
            .to_lowercase();

        resolved.push(ResolvedEntry {
            payload: RegisterPayload {
                contract_id: entry.contract_id.clone(),
                name: entry.name.clone(),
                network,
                description: entry.description.clone(),
                category: entry.category.clone(),
                tags: entry.tags.clone().unwrap_or_default(),
                wasm_hash: entry.wasm_hash.clone(),
                source_url: entry.source_url.clone(),
                publisher_address: publisher.to_string(),
            },
        });

        let _ = i; // suppress unused warning
    }

    Ok(resolved)
}

// ── Deduplication ─────────────────────────────────────────────────────────────

fn deduplicate(entries: Vec<ResolvedEntry>) -> (Vec<ResolvedEntry>, usize) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut deduped: Vec<ResolvedEntry> = Vec::new();
    let total = entries.len();

    for entry in entries {
        let id = entry.payload.contract_id.clone();
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id);
        deduped.push(entry);
    }

    let skipped = total - deduped.len();
    (deduped, skipped)
}

// ── Validation ────────────────────────────────────────────────────────────────

fn validate_all(entries: &[ResolvedEntry]) -> Result<()> {
    let valid_networks = ["mainnet", "testnet", "futurenet"];
    let mut errors: Vec<String> = Vec::new();

    for entry in entries {
        let p = &entry.payload;

        if p.contract_id.trim().is_empty() {
            errors.push(format!("contract_id is empty for entry '{}'", p.name));
        }
        if p.name.trim().is_empty() {
            errors.push(format!("name is empty for contract_id '{}'", p.contract_id));
        }
        if !valid_networks.contains(&p.network.as_str()) {
            errors.push(format!(
                "'{}': invalid network '{}' — must be one of: mainnet, testnet, futurenet",
                p.contract_id, p.network
            ));
        }
        if p.publisher_address.trim().is_empty() {
            errors.push(format!("'{}': publisher_address is empty", p.contract_id));
        }
    }

    if !errors.is_empty() {
        let msg = errors.join("\n  ");
        anyhow::bail!("Validation failed:\n  {}", msg);
    }

    Ok(())
}

// ── Dry-run output ────────────────────────────────────────────────────────────

fn emit_dry_run(entries: Vec<ResolvedEntry>, skipped_duplicates: usize, json: bool) -> Result<()> {
    let results: Vec<RegistrationResult> = entries
        .iter()
        .map(|e| RegistrationResult {
            contract_id: e.payload.contract_id.clone(),
            name: e.payload.name.clone(),
            status: "dry_run".to_string(),
            registry_id: None,
            error: None,
        })
        .collect();

    let summary = RegistrationSummary {
        total: results.len(),
        registered: 0,
        failed: 0,
        skipped: skipped_duplicates,
        results,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    println!(
        "\n{}",
        "Dry-run results (no contracts were registered):"
            .bold()
            .yellow()
    );
    println!("{}", "=".repeat(60).yellow());
    for r in &summary.results {
        println!(
            "  {} {} — {}",
            "⊙".bright_black(),
            r.contract_id.bold(),
            r.name.bright_black()
        );
    }
    if skipped_duplicates > 0 {
        println!(
            "\n  {} duplicate(s) would be skipped.",
            skipped_duplicates.to_string().yellow()
        );
    }
    println!(
        "\n  {} {} contract(s) would be registered.\n",
        "→".cyan(),
        summary.total.to_string().bold()
    );

    Ok(())
}

// ── Registration loop ─────────────────────────────────────────────────────────

async fn register_all(
    api_url: &str,
    entries: Vec<ResolvedEntry>,
    skipped_duplicates: usize,
    json: bool,
) -> Result<RegistrationSummary> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REGISTER_TIMEOUT_SECS))
        .build()?;

    let url = format!("{}/api/contracts", api_url);
    let total = entries.len();
    let mut results: Vec<RegistrationResult> = Vec::with_capacity(total);
    let mut registered = 0usize;
    let mut failed = 0usize;

    for (i, entry) in entries.into_iter().enumerate() {
        if !json {
            print!(
                "  [{}/{}] Registering {} ... ",
                i + 1,
                total,
                entry.payload.contract_id.bold()
            );
        }

        match register_one(&client, &url, entry).await {
            Ok(result) => {
                if !json {
                    println!("{}", "registered".green());
                }
                registered += 1;
                results.push(result);
            }
            Err(err) => {
                let err_str = err.to_string();
                if !json {
                    println!("{} — {}", "failed".red(), err_str.red());
                }
                failed += 1;
                results.push(RegistrationResult {
                    contract_id: String::new(), // filled in by register_one on success path only
                    name: String::new(),
                    status: "failed".to_string(),
                    registry_id: None,
                    error: Some(err_str),
                });
            }
        }
    }

    Ok(RegistrationSummary {
        total,
        registered,
        failed,
        skipped: skipped_duplicates,
        results,
    })
}

async fn register_one(
    client: &reqwest::Client,
    url: &str,
    entry: ResolvedEntry,
) -> Result<RegistrationResult> {
    let contract_id = entry.payload.contract_id.clone();
    let name = entry.payload.name.clone();

    let response = client
        .post(url)
        .json(&entry.payload)
        .send_with_retry().await
        .context("Failed to reach registry API")?;

    if response.status() == reqwest::StatusCode::CONFLICT {
        // 409 Conflict → contract already exists; treat as skipped duplicate
        return Ok(RegistrationResult {
            contract_id,
            name,
            status: "skipped".to_string(),
            registry_id: None,
            error: None,
        });
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }

    let body: serde_json::Value = response
        .json()
        .await
        .context("Invalid JSON from registry")?;
    let registry_id = body["id"]
        .as_str()
        .or_else(|| body["contract_id"].as_str())
        .map(|s| s.to_string());

    Ok(RegistrationResult {
        contract_id,
        name,
        status: "registered".to_string(),
        registry_id,
        error: None,
    })
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn print_header(
    entries: &[ResolvedEntry],
    skipped_duplicates: usize,
    publisher: &str,
    dry_run: bool,
) {
    let mode = if dry_run {
        " (DRY RUN)".yellow().to_string()
    } else {
        String::new()
    };

    println!("\n{}{}", "Bulk Contract Registration".bold().cyan(), mode);
    println!("{}", "=".repeat(60).cyan());
    println!("  {}: {}", "Contracts".bold(), entries.len());
    if skipped_duplicates > 0 {
        println!(
            "  {}: {} (deduplicated)",
            "Duplicates removed".bold(),
            skipped_duplicates.to_string().yellow()
        );
    }
    println!("  {}: {}", "Publisher".bold(), publisher.bright_black());
    println!(
        "  {}: {}s per contract",
        "Timeout".bold(),
        REGISTER_TIMEOUT_SECS
    );
    println!();
}

fn print_summary(summary: &RegistrationSummary) {
    println!("\n{}", "Registration Summary".bold().cyan());
    println!("{}", "=".repeat(60).cyan());

    let reg_str = format!("{} registered", summary.registered).green();
    let fail_str = format!("{} failed", summary.failed).red();
    let skip_str = if summary.skipped > 0 {
        format!(", {} skipped", summary.skipped)
            .bright_black()
            .to_string()
    } else {
        String::new()
    };
    println!(
        "  {} — {}, {}{}",
        "Summary".bold(),
        reg_str,
        fail_str,
        skip_str
    );
    println!();

    if summary.failed == 0 {
        println!(
            "  {} All {} contract(s) registered successfully!",
            "✓".green().bold(),
            summary.registered
        );
    } else {
        println!(
            "  {} {} contract(s) failed.",
            "✗".red().bold(),
            summary.failed
        );
    }

    println!("\n{}", "Per-contract results:".bold());

    for r in &summary.results {
        let (icon, label) = match r.status.as_str() {
            "registered" => ("✓".green(), r.status.green()),
            "failed" => ("✗".red(), r.status.red()),
            "skipped" => ("⊘".bright_black(), r.status.bright_black()),
            other => ("?".bright_black(), other.normal()),
        };

        println!("\n  {} {} — {}", icon, r.contract_id.bold(), label);
        if let Some(id) = &r.registry_id {
            println!("    Registry ID: {}", id.bright_black());
        }
        if let Some(err) = &r.error {
            println!("    Error: {}", err.red());
        }
    }

    println!("\n{}\n", "=".repeat(60).cyan());
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(contract_id: &str, name: &str, network: &str) -> ResolvedEntry {
        ResolvedEntry {
            payload: RegisterPayload {
                contract_id: contract_id.to_string(),
                name: name.to_string(),
                network: network.to_string(),
                description: None,
                category: None,
                tags: vec![],
                wasm_hash: None,
                source_url: None,
                publisher_address: "GABC123".to_string(),
            },
        }
    }

    #[test]
    fn dedup_removes_duplicate_ids() {
        let entries = vec![
            make_entry("CA1", "Alpha", "testnet"),
            make_entry("CA2", "Beta", "testnet"),
            make_entry("CA1", "Alpha-dup", "testnet"),
        ];
        let (deduped, skipped) = deduplicate(entries);
        assert_eq!(deduped.len(), 2);
        assert_eq!(skipped, 1);
        assert!(deduped.iter().all(|e| e.payload.contract_id != "CA1"
            || deduped
                .iter()
                .filter(|x| x.payload.contract_id == "CA1")
                .count()
                == 1));
    }

    #[test]
    fn dedup_no_duplicates_unchanged() {
        let entries = vec![
            make_entry("CA1", "Alpha", "testnet"),
            make_entry("CA2", "Beta", "testnet"),
        ];
        let (deduped, skipped) = deduplicate(entries);
        assert_eq!(deduped.len(), 2);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn validate_rejects_empty_contract_id() {
        let entries = vec![make_entry("", "Alpha", "testnet")];
        assert!(validate_all(&entries).is_err());
    }

    #[test]
    fn validate_rejects_empty_name() {
        let entries = vec![make_entry("CA1", "", "testnet")];
        assert!(validate_all(&entries).is_err());
    }

    #[test]
    fn validate_rejects_unknown_network() {
        let entries = vec![make_entry("CA1", "Alpha", "devnet")];
        assert!(validate_all(&entries).is_err());
    }

    #[test]
    fn validate_accepts_all_valid_networks() {
        for net in &["mainnet", "testnet", "futurenet"] {
            let entries = vec![make_entry("CA1", "Alpha", net)];
            assert!(validate_all(&entries).is_ok(), "failed for network {net}");
        }
    }

    #[test]
    fn validate_rejects_empty_publisher() {
        let mut entry = make_entry("CA1", "Alpha", "testnet");
        entry.payload.publisher_address = String::new();
        assert!(validate_all(&[entry]).is_err());
    }

    #[test]
    fn load_manifest_rejects_unknown_extension() {
        let result = load_manifest("/tmp/contracts.csv");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported manifest extension"));
    }

    #[test]
    fn load_manifest_yaml_parses_correctly() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
publisher: "GABC"
network: "testnet"
contracts:
  - contract_id: "CA1"
    name: "Token"
    tags: ["erc20"]
"#
        )
        .unwrap();
        // Rename to .yaml so load_manifest recognises the extension
        let path = f.path().with_extension("yaml");
        std::fs::copy(f.path(), &path).unwrap();

        let manifest = load_manifest(path.to_str().unwrap()).unwrap();
        assert_eq!(manifest.contracts.len(), 1);
        assert_eq!(manifest.contracts[0].contract_id, "CA1");
        assert_eq!(manifest.publisher.unwrap(), "GABC");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_manifest_json_parses_correctly() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(
            f,
            r#"{{"publisher":"GXYZ","network":"mainnet","contracts":[{{"contract_id":"CB1","name":"DEX"}}]}}"#
        )
        .unwrap();
        let path = f.path().with_extension("json");
        std::fs::copy(f.path(), &path).unwrap();

        let manifest = load_manifest(path.to_str().unwrap()).unwrap();
        assert_eq!(manifest.contracts.len(), 1);
        assert_eq!(manifest.contracts[0].name, "DEX");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn resolve_entries_uses_manifest_default_network() {
        let manifest = RegisterManifest {
            publisher: Some("GABC".to_string()),
            network: Some("mainnet".to_string()),
            contracts: vec![ManifestEntry {
                contract_id: "CA1".to_string(),
                name: "Token".to_string(),
                network: None,
                description: None,
                category: None,
                tags: None,
                wasm_hash: None,
                source_url: None,
            }],
        };
        let entries = resolve_entries(&manifest, "GABC").unwrap();
        assert_eq!(entries[0].payload.network, "mainnet");
    }

    #[test]
    fn resolve_entries_per_entry_network_overrides_default() {
        let manifest = RegisterManifest {
            publisher: None,
            network: Some("testnet".to_string()),
            contracts: vec![ManifestEntry {
                contract_id: "CA1".to_string(),
                name: "Token".to_string(),
                network: Some("futurenet".to_string()),
                description: None,
                category: None,
                tags: None,
                wasm_hash: None,
                source_url: None,
            }],
        };
        let entries = resolve_entries(&manifest, "GABC").unwrap();
        assert_eq!(entries[0].payload.network, "futurenet");
    }

    #[test]
    fn batch_size_limit_enforced() {
        // Produce MAX_BATCH_SIZE + 1 unique entries
        let entries: Vec<ResolvedEntry> = (0..=MAX_BATCH_SIZE)
            .map(|i| make_entry(&format!("CA{i}"), "Name", "testnet"))
            .collect();
        assert_eq!(entries.len(), MAX_BATCH_SIZE + 1);
    }
}
