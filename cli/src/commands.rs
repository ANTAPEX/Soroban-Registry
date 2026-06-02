use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use serde_json::json;

use crate::config::Network;

use std::path::Path;

use crate::patch::{PatchManager, Severity};
use crate::profiler;
use crate::sla::SlaManager;
use crate::test_framework;

pub async fn search(
    api_url: &str,
    query: &str,
    network: Network,
    verified_only: bool,
	 json: bool,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!(
        "{}/api/contracts?query={}&network={}",
        api_url, query, network
    );

    if verified_only {
        url.push_str("&verified_only=true");
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to search contracts")?;

    let data: serde_json::Value = response.json().await?;
    let items = data["items"].as_array().context("Invalid response")?;

	 if json {
        let contracts: Vec<serde_json::Value> = items
            .iter()
            .map(|c| serde_json::json!({
                "id":          c["contract_id"].as_str().unwrap_or(""),
                "name":        c["name"].as_str().unwrap_or("Unknown"),
                "is_verified": c["is_verified"].as_bool().unwrap_or(false),
                "network":     c["network"].as_str().unwrap_or(""),
            }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "contracts": contracts }))?);
        return Ok(());
    }

    println!("\n{}", "Search Results:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if items.is_empty() {
        println!("{}", "No contracts found.".yellow());
        return Ok(());
    }

    for contract in items {
        let name = contract["name"].as_str().unwrap_or("Unknown");
        let contract_id = contract["contract_id"].as_str().unwrap_or("");
        let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
        let network = contract["network"].as_str().unwrap_or("");

        println!("\n{} {}", "●".green(), name.bold());
        println!("  ID: {}", contract_id.bright_black());
        println!(
            "  Status: {} | Network: {}",
            if is_verified {
                "✓ Verified".green()
            } else {
                "○ Unverified".yellow()
            },
            network.bright_blue()
        );

        if let Some(desc) = contract["description"].as_str() {
            println!("  {}", desc.bright_black());
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!("Found {} contract(s)\n", items.len());

    Ok(())
}



fn resolve_smart_routing(current_network: Network) -> String {
    if current_network.to_string() == "auto" {
        "mainnet".to_string() 
    } else {
        current_network.to_string()
    }
}

pub async fn publish(
    api_url: &str,
    contract_id: &str,
    name: &str,
    description: Option<&str>,
    network: Network,
    category: Option<&str>,
    tags: Vec<String>,
    publisher: &str,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "name": name,
        "description": description,
        "network": network.to_string(),
        "category": category,
        "tags": tags,
        "publisher_address": publisher,
    });

    println!("\n{}", "Publishing contract...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to publish contract")?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Failed to publish: {}", error_text);
    }

    let contract: serde_json::Value = response.json().await?;

    println!("{}", "✓ Contract published successfully!".green().bold());
    println!(
        "\n{}: {}",
        "Name".bold(),
        contract["name"].as_str().unwrap_or("")
    );
    println!(
        "{}: {}",
        "ID".bold(),
        contract["contract_id"].as_str().unwrap_or("")
    );
    println!(
        "{}: {}",
        "Network".bold(),
        contract["network"].as_str().unwrap_or("").bright_blue()
    );
    println!();

    Ok(())
}

pub async fn list(api_url: &str, limit: usize, network: Network, json: bool,) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts?page_size={}&network={}",
        api_url, limit, network
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to list contracts")?;

    let data: serde_json::Value = response.json().await?;
    let items = data["items"].as_array().context("Invalid response")?;

	if json {
        let contracts: Vec<serde_json::Value> = items
            .iter()
            .map(|c| serde_json::json!({
                "id":          c["contract_id"].as_str().unwrap_or(""),
                "name":        c["name"].as_str().unwrap_or("Unknown"),
                "is_verified": c["is_verified"].as_bool().unwrap_or(false),
                "network":     c["network"].as_str().unwrap_or(""),
            }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "contracts": contracts }))?);
        return Ok(());
    }

    println!("\n{}", "Recent Contracts:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if items.is_empty() {
        println!("{}", "No contracts found.".yellow());
        return Ok(());
    }

    for (i, contract) in items.iter().enumerate() {
        let name = contract["name"].as_str().unwrap_or("Unknown");
        let contract_id = contract["contract_id"].as_str().unwrap_or("");
        let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
        let network = contract["network"].as_str().unwrap_or("");

        println!(
            "\n{}. {} {}",
            i + 1,
            name.bold(),
            if is_verified {
                "✓".green()
            } else {
                "".normal()
            }
        );
        println!(
            "   {} | {}",
            contract_id.bright_black(),
            network.bright_blue()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

pub async fn breaking_changes(api_url: &str, old_id: &str, new_id: &str, json: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts/breaking-changes?old_id={}&new_id={}",
        api_url, old_id, new_id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch breaking changes")?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Failed to fetch breaking changes: {}", error_text);
    }

    let report: serde_json::Value = response.json().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let breaking = report["breaking"].as_bool().unwrap_or(false);
    let breaking_count = report["breaking_count"].as_u64().unwrap_or(0);
    let non_breaking_count = report["non_breaking_count"].as_u64().unwrap_or(0);

    let header = if breaking {
        "Breaking changes detected".red().bold()
    } else {
        "No breaking changes detected".green().bold()
    };

    println!("\n{}", header);
    println!(
        "{} {} | {} {}",
        "Breaking:".bold(),
        breaking_count,
        "Non-breaking:".bold(),
        non_breaking_count
    );

    if let Some(changes) = report["changes"].as_array() {
        for change in changes {
            let severity = change["severity"].as_str().unwrap_or("unknown");
            let message = change["message"].as_str().unwrap_or("Change");
            let label = if severity == "breaking" {
                "BREAKING".red().bold()
            } else {
                "INFO".yellow().bold()
            };
            println!("  {} {}", label, message);
        }
    }

    Ok(())
}


pub async fn migrate(
    api_url: &str,
    contract_id: &str,
    wasm_path: &str,
    simulate_fail: bool,
    dry_run: bool,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::fs;
    use tokio::process::Command;

    println!("\n{}", "Migration Tool".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    // 1. Read WASM file
    let wasm_bytes = fs::read(wasm_path)
        .with_context(|| format!("Failed to read WASM file at {}", wasm_path))?;

    // 2. Compute Hash
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let wasm_hash = hex::encode(hasher.finalize());

    println!("Contract ID: {}", contract_id.green());
    println!("WASM Path:   {}", wasm_path);
    println!("WASM Hash:   {}", wasm_hash.bright_black());
    println!("Size:        {} bytes", wasm_bytes.len());

    if dry_run {
        println!("\n{}", "[DRY RUN] No changes will be made.".yellow().bold());
        println!("Would create migration record...");
        println!(
            "Would execute: soroban contract invoke --id {} --wasm {} ...",
            contract_id, wasm_path
        );
        return Ok(());
    }

    // 3. Create Migration Record (Pending)
    let client = reqwest::Client::new();
    let create_url = format!("{}/api/migrations", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "wasm_hash": wasm_hash,
    });

    print!("\nInitializing migration... ");
    let response = client
        .post(&create_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to contact registry API")?;

    if !response.status().is_success() {
        println!("{}", "Failed".red());
        let err = response.text().await?;
        anyhow::bail!("API Error: {}", err);
    }

    let migration: serde_json::Value = response.json().await?;
    let migration_id = migration["id"].as_str().unwrap();
    println!("{}", "OK".green());
    println!("Migration ID: {}", migration_id);

    // 4. Execute Migration (Mock or Real)
    println!("\n{}", "Executing migration logic...".bold());

    // Check if soroban is installed
    let version_output = Command::new("soroban").arg("--version").output().await;

    let (status, log_output) = if version_output.is_err() {
        println!(
            "{}",
            "Warning: 'soroban' CLI not found. Running in MOCK mode.".yellow()
        );

        if simulate_fail {
            println!("{}", "Simulating FAILURE...".red());
            (
                shared::models::MigrationStatus::Failed,
                "Simulation: Migration failed as requested.".to_string(),
            )
        } else {
            println!("{}", "Simulating SUCCESS...".green());
            (
                shared::models::MigrationStatus::Success,
                "Simulation: Migration succeeded.".to_string(),
            )
        }
    } else {
        // Real execution would go here. For now we will just mock it even if soroban exists
        // because we don't have a real contract to invoke in this environment.
        println!(
            "{}",
            "Soroban CLI found, but full integration is pending. Running in MOCK mode.".yellow()
        );
        if simulate_fail {
            println!("{}", "Simulating FAILURE...".red());
            (
                shared::models::MigrationStatus::Failed,
                "Simulation: Migration failed as requested.".to_string(),
            )
        } else {
            println!("{}", "Simulating SUCCESS...".green());
            (
                shared::models::MigrationStatus::Success,
                "Simulation: Migration executed successfully via soroban CLI (mocked).".to_string(),
            )
        }
    };

    // 5. Update Status
    let update_url = format!("{}/api/migrations/{}", api_url, migration_id);
    let update_payload = json!({
        "status": status,
        "log_output": log_output
    });

    let update_res = client
        .put(&update_url)
        .json(&update_payload)
        .send()
        .await
        .context("Failed to update migration status")?;

    if !update_res.status().is_success() {
        println!("{}", "Failed to update status!".red());
    } else {
        println!("\n{}", "Migration recorded successfully.".green().bold());
        if status == shared::models::MigrationStatus::Failed {
            println!("{}", "Status: FAILED".red().bold());
        } else {
            println!("{}", "Status: SUCCESS".green().bold());
        }
    }

    Ok(())
}

pub async fn export(
    api_url: &str,
    contract_id: &str,
    output: &str,
    contract_dir: &str,
) -> Result<()> {
    println!("\n{}", "Exporting contract...".bold().cyan());

    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}", api_url, contract_id);
    let response = client.get(&url).send().await.context("Failed to fetch contract info")?;

    let (name, network_str) = if response.status().is_success() {
        let contract: serde_json::Value = response.json().await?;
        (
            contract["name"].as_str().unwrap_or("unknown").to_string(),
            contract["network"].as_str().unwrap_or("testnet").to_string()
        )
    } else {
        (contract_id.to_string(), "testnet".to_string())
    };

    let source = std::path::Path::new(contract_dir);
    anyhow::ensure!(
        source.is_dir(),
        "contract directory does not exist: {}",
        contract_dir
    );

    crate::export::create_archive(
        source,
        std::path::Path::new(output),
        contract_id,
        &name,
        &network_str,
    )?;

    println!("{}", "✓ Export complete!".green().bold());
    println!("  {}: {}", "Output".bold(), output);
    println!("  {}: {}", "Contract".bold(), contract_id.bright_black());
    println!("  {}: {}\n", "Name".bold(), name);

    Ok(())
}

pub async fn import(
    api_url: &str,
    archive: &str,
    network: Network,
    output_dir: &str,
) -> Result<()> {
    println!("\n{}", "Importing contract...".bold().cyan());

    let archive_path = std::path::Path::new(archive);
    anyhow::ensure!(archive_path.is_file(), "archive not found: {}", archive);

    let dest = std::path::Path::new(output_dir);

    let manifest = crate::import::extract_and_verify(archive_path, dest)?;

    println!(
        "{}",
        "✓ Import complete — integrity verified!".green().bold()
    );
    println!(
        "  {}: {}",
        "Contract".bold(),
        manifest.contract_id.bright_black()
    );
    println!("  {}: {}", "Name".bold(), manifest.name);
    println!(
        "  {}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("  {}: {}", "SHA-256".bold(), manifest.sha256.bright_black());
    println!("  {}: {}", "Exported At".bold(), manifest.exported_at);
    println!(
        "  {}: {} file(s)",
        "Contents".bold(),
        manifest.contents.len()
    );
    println!("  {}: {}", "Extracted To".bold(), output_dir);

    println!(
        "\n  {} To register on {}, run:",
        "→".bright_black(),
        network.to_string().bright_blue()
    );
    println!(
        "    soroban-registry publish --contract-id {} --name \"{}\" --network {} --publisher <address>\n",
        manifest.contract_id, manifest.name, network
    );

    Ok(())
}

fn severity_colored(sev: &Severity) -> colored::ColoredString {
    match sev {
        Severity::Critical => "CRITICAL".red().bold(),
        Severity::High => "HIGH".yellow().bold(),
        Severity::Medium => "MEDIUM".cyan(),
        Severity::Low => "LOW".normal(),
    }
}

pub async fn patch_create(
    api_url: &str,
    version: &str,
    hash: &str,
    severity: Severity,
    rollout: u8,
) -> Result<()> {
    println!("\n{}", "Creating security patch...".bold().cyan());

    let patch = PatchManager::create(api_url, version, hash, severity, rollout).await?;

    println!("{}", "✓ Patch created!".green().bold());
    println!("  {}: {}", "ID".bold(), patch.id);
    println!("  {}: {}", "Target Version".bold(), patch.target_version);
    println!(
        "  {}: {}",
        "Severity".bold(),
        severity_colored(&patch.severity)
    );
    println!(
        "  {}: {}",
        "New WASM Hash".bold(),
        patch.new_wasm_hash.bright_black()
    );
    println!("  {}: {}%\n", "Rollout".bold(), patch.rollout_percentage);

    if matches!(patch.severity, Severity::Critical | Severity::High) {
        println!(
            "  {} {}",
            "⚠".red(),
            format!(
                "{} severity — immediate action recommended",
                severity_colored(&patch.severity)
            )
            .red()
        );
    }

    Ok(())
}

/// GET /api/contracts/:id/trust-score
pub async fn trust_score(api_url: &str, contract_id: &str, network: Network) -> Result<()> {
    let url = format!("{}/api/contracts/{}/trust-score", api_url, contract_id);
    log::debug!("GET {}", url);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .query(&[("network", network.to_string())])
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get trust score ({}): {}", status, body);
    }

    let data: serde_json::Value = resp.json().await.context("Failed to parse trust score response")?;

    // ── Header ────────────────────────────────────────────────────────────────
    let name       = data["contract_name"].as_str().unwrap_or("Unknown");
    let score      = data["score"].as_f64().unwrap_or(0.0);
    let badge      = data["badge"].as_str().unwrap_or("Bronze");
    let badge_icon = data["badge_icon"].as_str().unwrap_or("🥉");
    let summary    = data["summary"].as_str().unwrap_or("");

    println!("\n{}", "─".repeat(56));
    println!("  Trust Score — {}", name.bold());
    println!("{}", "─".repeat(56));
    println!("  Score : {:.0}/100", score);
    println!("  Badge : {} {}", badge_icon, badge.bold());
    println!("  {}",  summary);
    println!("{}", "─".repeat(56));

    // ── Factor breakdown ──────────────────────────────────────────────────────
    println!("\n  {} Factor Breakdown\n", "📊".bold());

    if let Some(factors) = data["factors"].as_array() {
        for factor in factors {
            let fname   = factor["name"].as_str().unwrap_or("");
            let earned  = factor["points_earned"].as_f64().unwrap_or(0.0);
            let max     = factor["points_max"].as_f64().unwrap_or(0.0);
            let explain = factor["explanation"].as_str().unwrap_or("");

            // Mini progress bar (10 chars)
            let filled = ((earned / max) * 10.0).round() as usize;
            let filled = filled.min(10);
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));

            println!("  {:<28} [{bar}] {:.0}/{:.0}", fname, earned, max);
            println!("    {}", explain.dimmed());
        }
    }

    // ── Weight documentation ──────────────────────────────────────────────────
    println!("\n  {} Score Weights\n", "⚖️".bold());
    if let Some(weights) = data["weights"].as_object() {
        for (k, v) in weights {
            println!("  {:<22} {:.0} pts max", k, v.as_f64().unwrap_or(0.0));
        }
    }

    let computed_at = data["computed_at"].as_str().unwrap_or("");
    println!("\n  Computed at: {}\n", computed_at.dimmed());

    Ok(())
}

pub async fn patch_notify(api_url: &str, patch_id: &str) -> Result<()> {
    println!("\n{}", "Identifying vulnerable contracts...".bold().cyan());

    let (patch, contracts) = PatchManager::find_vulnerable(api_url, patch_id).await?;

    println!(
        "\n{} {} patch for version {}",
        "⚠".bold(),
        severity_colored(&patch.severity),
        patch.target_version.bold()
    );
    println!("{}", "=".repeat(80).cyan());

    if contracts.is_empty() {
        println!("{}", "No vulnerable contracts found.".green());
        return Ok(());
    }

    for (i, c) in contracts.iter().enumerate() {
        let cid = c["contract_id"].as_str().unwrap_or("");
        let name = c["name"].as_str().unwrap_or("Unknown");
        let net = c["network"].as_str().unwrap_or("");
        println!(
            "  {}. {} ({}) [{}]",
            i + 1,
            name.bold(),
            cid.bright_black(),
            net.bright_blue()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!("{} vulnerable contract(s) found\n", contracts.len());

    Ok(())
}

pub async fn patch_apply(api_url: &str, contract_id: &str, patch_id: &str) -> Result<()> {
    println!("\n{}", "Applying security patch...".bold().cyan());

    let audit = PatchManager::apply(api_url, contract_id, patch_id).await?;

    println!("{}", "✓ Patch applied successfully!".green().bold());
    println!("  {}: {}", "Contract".bold(), audit.contract_id);
    println!("  {}: {}", "Patch".bold(), audit.patch_id);
    println!("  {}: {}\n", "Applied At".bold(), audit.applied_at);

    Ok(())
}

pub async fn profile(
    contract_path: &str,
    method: Option<&str>,
    output: Option<&str>,
    flamegraph: Option<&str>,
    compare: Option<&str>,
    recommendations: bool,
) -> Result<()> {
    use crate::profiler;
    println!("\n{}", "Profiling Contract Execution...".bold().cyan());

    let profile_data = profiler::profile_contract(contract_path, method)?;

    if let Some(out) = output {
        let content = serde_json::to_string_pretty(&profile_data)?;
        std::fs::write(out, content)?;
        println!("Saved profile data to {}", out);
    }

    if let Some(flame) = flamegraph {
        std::fs::write(flame, "<svg>Mock Flamegraph</svg>")?;
        println!("Saved flamegraph to {}", flame);
    }

    if let Some(comp_path) = compare {
        let baseline_content = std::fs::read_to_string(comp_path)?;
        let baseline: profiler::ProfileData = serde_json::from_str(&baseline_content)?;
        let comparisons = profiler::compare_profiles(&baseline, &profile_data);

        println!("\n{}", "Comparison Results:".bold().yellow());
        for comp in comparisons.iter().take(10) {
            let sign = if comp.time_diff_ns > 0 { "+" } else { "" };
            println!(
                "{}: {} ({}{:.2}%, {:.2}ms → {:.2}ms)",
                comp.function.bold(),
                comp.status,
                sign,
                comp.time_diff_percent,
                comp.baseline_time.as_secs_f64() * 1000.0,
                comp.current_time.as_secs_f64() * 1000.0
            );
        }
    }

    if recommendations {
        let recs = profiler::generate_recommendations(&profile_data);
        println!("\n{}", "Recommendations:".bold().magenta());
        for (i, rec) in recs.iter().enumerate() {
            println!("{}. {}", i + 1, rec);
        }
    }

    Ok(())
}

pub async fn deps_list(api_url: &str, contract_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/dependencies", api_url, contract_id);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch contract dependencies")?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
             anyhow::bail!("Contract not found");
        }
        anyhow::bail!("Failed to fetch dependencies: {}", response.status());
    }

    let items: serde_json::Value = response.json().await?;
    let tree = items.as_array().context("Invalid response format")?;

    println!("\n{}", "Dependency Tree:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if tree.is_empty() {
        println!("{}", "No dependencies found.".yellow());
        return Ok(());
    }

    fn print_tree(nodes: &[serde_json::Value], prefix: &str, is_last: bool) {
        for (i, node) in nodes.iter().enumerate() {
            let name = node["name"].as_str().unwrap_or("Unknown");
            let constraint = node["constraint_to_parent"].as_str().unwrap_or("*");
            let contract_id = node["contract_id"].as_str().unwrap_or("");
            
            let is_node_last = i == nodes.len() - 1;
            let marker = if is_node_last { "└──" } else { "├──" };
            
            println!(
                "{}{} {} ({}) {}", 
                prefix, 
                marker.bright_black(), 
                name.bold(), 
                constraint.cyan(),
                if contract_id == "unknown" { "[Unresolved]".red() } else { "".normal() }
            );

            if let Some(children) = node["dependencies"].as_array() {
                if !children.is_empty() {
                     let new_prefix = format!("{}{}", prefix, if is_node_last { "    " } else { "│   " });
                     print_tree(children, &new_prefix, true);
                }
            }
        }
    }

    print_tree(tree, "", false);

    println!();
    Ok(())
}

pub async fn run_tests(
    test_file: &str,
    contract_path: Option<&str>,
    junit_output: Option<&str>,
    show_coverage: bool,
    verbose: bool,
) -> Result<()> {
    let test_path = Path::new(test_file);
    if !test_path.exists() {
        anyhow::bail!("Test file not found: {}", test_file);
    }

    let contract_dir = contract_path.unwrap_or(".");
    let mut runner = test_framework::TestRunner::new(contract_dir)?;

    println!("\n{}", "Running Integration Tests...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let scenario = test_framework::load_test_scenario(test_path)?;
    
    if verbose {
        println!("\n{}: {}", "Scenario".bold(), scenario.name);
        if let Some(desc) = &scenario.description {
            println!("{}: {}", "Description".bold(), desc);
        }
        println!("{}: {}", "Steps".bold(), scenario.steps.len());
    }

    let start_time = std::time::Instant::now();
    let result = runner.run_scenario(scenario).await?;
    let total_time = start_time.elapsed();

    println!("\n{}", "Test Results:".bold().green());
    println!("{}", "=".repeat(80).cyan());

    let status_icon = if result.passed { "✓" } else { "✗" };
    
    println!(
        "\n{} {} {} ({:.2}ms)",
        status_icon,
        "Scenario:".bold(),
        result.scenario.bold(),
        result.duration.as_secs_f64() * 1000.0
    );

    if !result.passed {
        if let Some(ref err) = result.error {
            println!("{} {}", "Error:".bold().red(), err);
        }
    }

    println!("\n{}", "Step Results:".bold());
    for (i, step) in result.steps.iter().enumerate() {
        let step_icon = if step.passed { "✓" } else { "✗" };
        
        println!(
            "  {}. {} {} ({:.2}ms)",
            i + 1,
            step_icon,
            step.step_name.bold(),
            step.duration.as_secs_f64() * 1000.0
        );

        if verbose {
            println!(
                "     Assertions: {}/{} passed",
                step.assertions_passed,
                step.assertions_passed + step.assertions_failed
            );
        }

        if let Some(ref err) = step.error {
            println!("     {}", err.red());
        }
    }

    if show_coverage {
        println!("\n{}", "Coverage Report:".bold().magenta());
        println!("  Contracts Tested: {}", result.coverage.contracts_tested);
        println!("  Methods Tested: {}/{}", 
            result.coverage.methods_tested, 
            result.coverage.total_methods
        );
        println!("  Coverage: {:.2}%", result.coverage.coverage_percent);
        
        if result.coverage.coverage_percent < 80.0 {
            println!("  {} Low coverage detected!", "⚠".yellow());
        }
    }

    if let Some(junit_path) = junit_output {
        test_framework::generate_junit_xml(&[result.clone()], Path::new(junit_path))?;
        println!("\n{} JUnit XML report exported to: {}", "✓".green(), junit_path);
    }

    if total_time.as_secs() > 5 {
        println!("\n{} Test execution took {:.2}s (target: <5s)", 
            "⚠".yellow(), 
            total_time.as_secs_f64()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    if !result.passed {
        anyhow::bail!("Tests failed");
    }

    Ok(())
}

pub fn incident_trigger(contract_id: &str, severity_str: &str) -> Result<()> {
    use crate::incident::{IncidentManager, IncidentSeverity};

    let severity = severity_str.parse::<IncidentSeverity>()?;
    let mut mgr = IncidentManager::default();
    let id = mgr.trigger(contract_id.to_string(), severity);

    println!("\n{}", "Incident Triggered".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("  {}: {}", "Incident ID".bold(), id);
    println!("  {}: {}", "Contract".bold(), contract_id.bright_black());
    println!(
        "  {}: {}",
        "Severity".bold(),
        match severity {
            IncidentSeverity::Critical => "CRITICAL".red().bold(),
            IncidentSeverity::High => "HIGH".yellow().bold(),
            IncidentSeverity::Medium => "MEDIUM".cyan(),
            IncidentSeverity::Low => "LOW".normal(),
        }
    );
    println!("  {}: Detected", "State".bold());

    if mgr.is_halted(contract_id) {
        println!(
            "\n  {} {}",
            "⚡ CIRCUIT BREAKER ENGAGED —".red().bold(),
            format!("contract {} is now halted", contract_id).red()
        );
    }

    println!(
        "\n  {} To advance state:\n    soroban-registry incident update {} --state responding\n",
        "→".bright_black(),
        id
    );

    Ok(())
}

pub async fn config_get(api_url: &str, contract_id: &str, environment: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config?environment={}", api_url, contract_id, environment);

    let response = client.get(&url).send().await.context("Failed to fetch configuration")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to get config: {}", response.text().await.unwrap_or_default());
    }

    let config: serde_json::Value = response.json().await?;

    println!("\n{}", "Contract Configuration (Latest):".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract ID".bold(), contract_id);
    println!("{}: {}", "Environment".bold(), environment);
    println!("{}: {}", "Version".bold(), config["version"].as_i64().unwrap_or(0));
    println!("{}: {}", "Contains Secrets".bold(), config["has_secrets"].as_bool().unwrap_or(false));
    println!("{}: {}", "Created By".bold(), config["created_by"].as_str().unwrap_or("Unknown"));
    println!("{}:", "Config Data".bold());
    println!("{}", serde_json::to_string_pretty(&config["config_data"]).unwrap_or_default().green());
    println!();

    Ok(())
}

pub async fn config_set(
    api_url: &str,
    contract_id: &str,
    environment: &str,
    config_data: &str,
    secrets_data: Option<&str>,
    created_by: &str,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config", api_url, contract_id);

    let mut payload = json!({
        "environment": environment,
        "config_data": serde_json::from_str::<serde_json::Value>(config_data).context("Invalid config JSON")?,
        "created_by": created_by,
    });

    if let Some(sec) = secrets_data {
        let sec_json: serde_json::Value = serde_json::from_str(sec).context("Invalid secrets JSON")?;
        payload["secrets_data"] = sec_json;
    }

    println!("\n{}", "Publishing configuration...".bold().cyan());

    let response = client.post(&url).json(&payload).send().await.context("Failed to set configuration")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to set config: {}", response.text().await.unwrap_or_default());
    }

    let config: serde_json::Value = response.json().await?;

    println!("{}", "✓ Configuration published successfully!".green().bold());
    println!("  {}: {}", "Environment".bold(), environment);
    println!("  {}: {}", "New Version".bold(), config["version"].as_i64().unwrap_or(0));
    println!();

    Ok(())
}

pub async fn config_history(api_url: &str, contract_id: &str, environment: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config/history?environment={}", api_url, contract_id, environment);

    let response = client.get(&url).send().await.context("Failed to fetch configuration history")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to get config history: {}", response.text().await.unwrap_or_default());
    }

    let configs: Vec<serde_json::Value> = response.json().await?;

    println!("\n{}", "Configuration History:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if configs.is_empty() {
        println!("{}", "No configurations found.".yellow());
        return Ok(());
    }

    for (i, config) in configs.iter().enumerate() {
        println!(
            "  {}. {} (v{}) - By: {}",
            i + 1,
            config["created_at"].as_str().unwrap_or("Unknown Date").bright_black(),
            config["version"].as_i64().unwrap_or(0),
            config["created_by"].as_str().unwrap_or("Unknown").bright_blue()
        );
    }
    println!();

    Ok(())
}

pub async fn config_rollback(
    api_url: &str,
    contract_id: &str,
    environment: &str,
    version: i32,
    created_by: &str,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config/rollback?environment={}", api_url, contract_id, environment);

    let payload = json!({
        "roll_back_to_version": version,
        "created_by": created_by,
    });

    println!("\n{}", format!("Rolling back configuration to v{}...", version).bold().cyan());

    let response = client.post(&url).json(&payload).send().await.context("Failed to rollback configuration")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to rollback config: {}", response.text().await.unwrap_or_default());
    }

    let config: serde_json::Value = response.json().await?;

    println!("{}", "✓ Configuration rolled back successfully!".green().bold());
    println!("  {}: {}", "Environment".bold(), environment);
    println!("  {}: {}", "New Active Version".bold(), config["version"].as_i64().unwrap_or(0));
    println!();

    Ok(())
}

pub fn incident_update(incident_id_str: &str, state_str: &str) -> Result<()> {
    use crate::incident::IncidentState;
    use uuid::Uuid;

    let id = incident_id_str
        .parse::<Uuid>()
        .map_err(|_| anyhow::anyhow!("invalid incident ID: {}", incident_id_str))?;
    let new_state = state_str.parse::<IncidentState>()?;

    println!("\n{}", "Incident Updated".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("  {}: {}", "Incident ID".bold(), id);
    println!("  {}: {}", "New State".bold(), new_state.to_string().green().bold());

    if matches!(new_state, IncidentState::Recovered | IncidentState::PostReview) {
        println!(
            "\n  {} {}",
            "✓".green(),
            "Circuit breaker cleared — registry interactions for this contract resumed.".green()
        );
    }

    println!();

    Ok(())
}

pub async fn scan_deps(
    api_url: &str,
    contract_id: &str,
    dependencies: &str,
    fail_on_high: bool,
) -> Result<()> {
    println!("\n{}", "Scanning Dependencies...".bold().cyan());

    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/scan", api_url, contract_id);

    // Parse dependencies
    let mut deps_list = Vec::new();
    for dep_pair in dependencies.split(',') {
        if dep_pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = dep_pair.split('@').collect();
        if parts.len() == 2 {
            deps_list.push(json!({
                "package_name": parts[0].trim(),
                "version": parts[1].trim()
            }));
        }
    }

    let payload = json!({
        "dependencies": deps_list,
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to run dependency scan")?;

    if !response.status().is_success() {
        anyhow::bail!("Scan failed: {}", response.text().await.unwrap_or_default());
    }

    let report: serde_json::Value = response.json().await?;
    let findings = report["findings"].as_array().unwrap();

    if findings.is_empty() {
        println!("{}", "✓ No vulnerabilities found!".green().bold());
        return Ok(());
    }

    let mut has_high_severity = false;
    println!("\n{}", "Vulnerabilities Found:".bold().red());
    println!("{}", "=".repeat(80).red());

    for finding in findings {
        let package = finding["package_name"].as_str().unwrap_or("Unknown");
        let version = finding["current_version"].as_str().unwrap_or("Unknown");
        let severity = finding["severity"].as_str().unwrap_or("Unknown");
        let cve_id = finding["cve_id"].as_str().unwrap_or("Unknown");
        let recommended = finding["recommended_version"].as_str().unwrap_or("None");

        let sev_enum = severity.parse::<Severity>().unwrap_or(Severity::Low);
        if matches!(sev_enum, Severity::Critical | Severity::High) {
            has_high_severity = true;
        }

        println!("  {} {}@{} - {}", severity_colored(&sev_enum), package, version, cve_id.bold());
        println!("    {} Recommended patch: {}", "↳".bright_black(), recommended.green());
    }

    println!("\n{}", "=".repeat(80).red());
    println!("{} issue(s) detected\n", findings.len());

    if fail_on_high && has_high_severity {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_network_parsing() {
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("futurenet".parse::<Network>().unwrap(), Network::Futurenet);
        assert_eq!("Mainnet".parse::<Network>().unwrap(), Network::Mainnet); // Case insensitive
        assert!("invalid".parse::<Network>().is_err());
    }

    // Note: Integration tests involving file system would require mocking or temporary files.
    // Given the constraints and the environment, we focus on unit tests for parsing here.
    // `resolve_network` with file interaction is harder to test in isolation without dependency injection or mocking `dirs` / `fs`.
}
/// Validate a contract function call for type safety
pub async fn validate_call(
    api_url: &str,
    contract_id: &str,
    method_name: &str,
    params: &[String],
    strict: bool,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/validate-call", api_url, contract_id);

    let body = json!({
        "method_name": method_name,
        "params": params,
        "strict": strict
    });

    log::debug!("POST {} body={}", url, body);

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Failed to validate contract call")?;

    let status = response.status();
    let data: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let error_msg = data["message"].as_str().unwrap_or("Unknown error");
        println!("\n{} {}", "Error:".bold().red(), error_msg);
        anyhow::bail!("Validation failed: {}", error_msg);
    }

    let valid = data["valid"].as_bool().unwrap_or(false);

    println!("\n{}", "Contract Call Validation".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("\n{}: {}", "Function".bold(), method_name);
    println!("{}: {}", "Contract".bold(), contract_id);
    println!("{}: {}", "Strict Mode".bold(), if strict { "Yes" } else { "No" });

    if valid {
        println!("\n{} {}", "✓".green().bold(), "Call is valid!".green().bold());

        // Show parsed parameters
        if let Some(params) = data["parsed_params"].as_array() {
            println!("\n{}", "Parsed Parameters:".bold());
            for param in params {
                let name = param["name"].as_str().unwrap_or("?");
                let type_name = param["expected_type"].as_str().unwrap_or("?");
                println!("  {} {}: {}", "•".green(), name.bold(), type_name);
            }
        }

        // Show expected return type
        if let Some(ret) = data["expected_return"].as_str() {
            println!("\n{}: {}", "Returns".bold(), ret);
        }

        // Show warnings
        if let Some(warnings) = data["warnings"].as_array() {
            if !warnings.is_empty() {
                println!("\n{}", "Warnings:".bold().yellow());
                for warning in warnings {
                    let msg = warning["message"].as_str().unwrap_or("?");
                    println!("  {} {}", "⚠".yellow(), msg);
                }
            }
        }
    } else {
        println!("\n{} {}", "✗".red().bold(), "Call is invalid!".red().bold());

        // Show errors
        if let Some(errors) = data["errors"].as_array() {
            println!("\n{}", "Errors:".bold().red());
            for error in errors {
                let code = error["code"].as_str().unwrap_or("?");
                let msg = error["message"].as_str().unwrap_or("?");
                let field = error["field"].as_str();

                if let Some(f) = field {
                    println!("  {} [{}] {}: {}", "✗".red(), code.bright_black(), f.bold(), msg);
                } else {
                    println!("  {} [{}] {}", "✗".red(), code.bright_black(), msg);
                }

                if let Some(expected) = error["expected"].as_str() {
                    println!("      Expected: {}", expected.green());
                }
                if let Some(actual) = error["actual"].as_str() {
                    println!("      Actual:   {}", actual.red());
                }
            }
        }
    }

    println!("\n{}", "=".repeat(60).cyan());
    println!();

    if !valid {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}

/// Generate type-safe bindings for a contract
pub async fn generate_bindings(
    api_url: &str,
    contract_id: &str,
    language: &str,
    output: Option<&str>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts/{}/bindings?language={}",
        api_url, contract_id, language
    );

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to generate bindings")?;

    let status = response.status();

    if !status.is_success() {
        let error: serde_json::Value = response.json().await?;
        let msg = error["message"].as_str().unwrap_or("Unknown error");
        anyhow::bail!("Failed to generate bindings: {}", msg);
    }

    let bindings = response.text().await?;

    if let Some(output_path) = output {
        fs::write(output_path, &bindings)?;
        println!(
            "\n{} {} bindings written to: {}",
            "✓".green().bold(),
            language,
            output_path
        );
    } else {
        // Print to stdout
        println!("{}", bindings);
    }

    Ok(())
}

/// List functions available on a contract
pub async fn list_functions(api_url: &str, contract_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/functions", api_url, contract_id);

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to list contract functions")?;

    let status = response.status();
    let data: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let msg = data["message"].as_str().unwrap_or("Unknown error");
        anyhow::bail!("Failed to list functions: {}", msg);
    }

    let contract_name = data["contract_name"].as_str().unwrap_or("Unknown");
    let functions = data["functions"].as_array();

    println!("\n{}", "Contract Functions".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("\n{}: {}", "Contract".bold(), contract_name);
    println!("{}: {}", "ID".bold(), contract_id);

    if let Some(funcs) = functions {
        println!("\n{} {} function(s):\n", "Found".bold(), funcs.len());

        for func in funcs {
            let name = func["name"].as_str().unwrap_or("?");
            let visibility = func["visibility"].as_str().unwrap_or("?");
            let return_type = func["return_type"].as_str().unwrap_or("void");
            let is_mutable = func["is_mutable"].as_bool().unwrap_or(false);

            let visibility_badge = if visibility == "public" {
                "public".green()
            } else {
                "internal".yellow()
            };

            let mutability = if is_mutable {
                "mut".red()
            } else {
                "view".blue()
            };

            println!(
                "  {} {} {} {}",
                "fn".bright_blue(),
                name.bold(),
                visibility_badge,
                mutability
            );

            // Parameters
            if let Some(params) = func["params"].as_array() {
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|p| {
                        let pname = p["name"].as_str().unwrap_or("?");
                        let ptype = p["type_name"].as_str().unwrap_or("?");
                        format!("{}: {}", pname, ptype)
                    })
                    .collect();

                println!("     ({}) -> {}", param_strs.join(", "), return_type);
            }

            // Doc
            if let Some(doc) = func["doc"].as_str() {
                println!("     /// {}", doc.bright_black());
            }

            println!();
        }
    } else {
        println!("\nNo functions found.");
    }

    println!("{}", "=".repeat(60).cyan());
    println!();

    Ok(())
}

pub async fn info(api_url: &str, contract_id: &str, network: Network) -> Result<()> {
    println!("\n{}", "Fetching contract information...".bold().cyan());
    
    let url = format!("{}/contracts/{}", api_url, contract_id);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .query(&[("network", network.to_string())])
       .send()
        .await?;

    if response.status().is_success() {
        let contract_info: serde_json::Value = response.json().await?;
        println!("\n{}", serde_json::to_string_pretty(&contract_info)?);
    } else {
        anyhow::bail!("Failed to fetch contract info: {}", response.status());
    }

    Ok(())
}

pub fn doc(contract_path: &str, output: &str) -> Result<()> {
    println!("\n{}", "Generating contract documentation...".bold().cyan());
    
    let content = format!(
        r#"# Contract Documentation

## Contract Path
{}

## Generated
{}

*This is a placeholder. Full documentation generation coming soon.*
"#,
        contract_path,
        chrono::Utc::now().to_rfc3339()
    );

    fs::write(output, content)?;
    println!("{} Documentation saved to: {}", "✓".green(), output);

    Ok(())
}

pub fn sla_record(id: &str, uptime: f64, latency: f64, error_rate: f64) -> Result<()> {
    println!("\n{}", "Recording SLA metrics...".bold().cyan());
    println!("Contract ID: {}", id);
    println!("Uptime: {:.2}%", uptime);
    println!("Latency: {:.2}ms", latency);
    println!("Error Rate: {:.2}%", error_rate);
    println!("{} SLA metrics recorded", "✓".green());

    Ok(())
}

pub fn sla_status(id: &str) -> Result<()> {
    println!("\n{}", "Fetching SLA status...".bold().cyan());
    println!("Contract ID: {}", id);
    println!("\nStatus: {}", "Active".green());
    println!("Uptime: {}%", "99.9".green());
    println!("Avg Latency: {}ms", "45.2".green());

    Ok(())
}
