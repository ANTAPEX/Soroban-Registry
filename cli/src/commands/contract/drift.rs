//! contract_drift.rs — `soroban-registry contract drift` (Issue #1191)
//!
//! Three-way WASM drift detection surfacing:
//! 1. Local lockfile hash (`soroban-registry.lock.json`)
//! 2. Registry catalog hash (`Contract.wasm_hash`)
//! 3. Stellar live on-chain hash (`ContractCodeEntry`)
//!
//! Commands:
//! - `soroban-registry contract drift --id <ID> [--lockfile <PATH>] [--network <NET>] [--json]`
//! - `soroban-registry contract drift --all [--status <STATUS>] [--network <NET>] [--json]`

use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use shared::{evaluate_three_way_drift, DriftStatus, ThreeWayDriftState};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MiniLockfile {
    #[serde(default)]
    pub contracts: BTreeMap<String, MiniLockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MiniLockEntry {
    pub contract_id: String,
    #[serde(default)]
    pub hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiDriftListResponse {
    pub contracts: Vec<shared::ContractDriftRecord>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

pub struct DriftCliOptions<'a> {
    pub api_url: &'a str,
    pub id: Option<&'a str>,
    pub all: bool,
    pub status: Option<&'a str>,
    pub lockfile: Option<&'a str>,
    pub network: Option<&'a str>,
    pub json: bool,
}

fn load_lockfile_hash(lockfile_path: Option<&str>, contract_id: &str) -> Option<String> {
    let candidate_path = lockfile_path.unwrap_or("soroban-registry.lock.json");
    let path = Path::new(candidate_path);
    if !path.exists() {
        return None;
    }

    let contents = fs::read_to_string(path).ok()?;
    let parsed: MiniLockfile = serde_json::from_str(&contents).ok()?;
    parsed
        .contracts
        .get(contract_id)
        .and_then(|entry| entry.hash.clone())
}

pub async fn run(opts: DriftCliOptions<'_>) -> Result<bool> {
    let client = reqwest::Client::new();
    let network = opts.network.unwrap_or("testnet");

    if opts.all {
        return run_all_contracts(&client, &opts, network).await;
    }

    let Some(id) = opts.id else {
        anyhow::bail!("Must specify either --id <CONTRACT_ID> or --all");
    };

    run_single_contract(&client, &opts, id, network).await
}

async fn run_single_contract(
    client: &reqwest::Client,
    opts: &DriftCliOptions<'_>,
    contract_id: &str,
    network: &str,
) -> Result<bool> {
    let url = format!(
        "{}/api/contracts/{}/drift?network={}",
        opts.api_url.trim_end_matches('/'),
        contract_id,
        network
    );

    let res = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to registry API at {}", url))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        anyhow::bail!("Registry API error: {}", err_text);
    }

    let mut state: ThreeWayDriftState = res
        .json()
        .await
        .with_context(|| "Failed to parse drift state from API")?;

    let lockfile_hash = load_lockfile_hash(opts.lockfile, contract_id);
    if let Some(ref lock_hash) = lockfile_hash {
        state.lockfile_hash = Some(lock_hash.clone());

        // Re-evaluate three-way drift incorporating lockfile
        let onchain_result = match &state.onchain_hash {
            Some(chain_hash) => Ok(Some(chain_hash.clone())),
            None => {
                if state.status == DriftStatus::NotOnChain {
                    Ok(None)
                } else {
                    Err("onchain unavailable")
                }
            }
        };

        let eval = evaluate_three_way_drift(
            &state.registry_hash,
            onchain_result.as_ref().map(|o| o.clone()).map_err(|e| *e),
            Some(lock_hash.as_str()),
            None,
            state.observed_at_ledger,
            Utc::now(),
        );

        state.status = eval.status;
        state.diverged_leg = eval.diverged_leg;
    }

    let has_drift = state.status == DriftStatus::Drift;

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&state)?);
    } else {
        print_single_contract_report(&state);
    }

    Ok(has_drift)
}

async fn run_all_contracts(
    client: &reqwest::Client,
    opts: &DriftCliOptions<'_>,
    network: &str,
) -> Result<bool> {
    let mut url = format!(
        "{}/api/contracts/drift?network={}&limit=100",
        opts.api_url.trim_end_matches('/'),
        network
    );

    if let Some(status) = opts.status {
        url.push_str(&format!("&status={}", status));
    }

    let res = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to registry API at {}", url))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        anyhow::bail!("Registry API error: {}", err_text);
    }

    let response: ApiDriftListResponse = res
        .json()
        .await
        .with_context(|| "Failed to parse drift contracts list from API")?;

    let drifted_count = response
        .contracts
        .iter()
        .filter(|c| c.status == DriftStatus::Drift)
        .count();

    let has_drift = drifted_count > 0;

    if opts.json {
        let output = serde_json::json!({
            "contracts": response.contracts,
            "total": response.total,
            "drifted_count": drifted_count,
            "network": network,
            "has_drift": has_drift,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_all_contracts_report(&response.contracts, response.total, network);
    }

    Ok(has_drift)
}

fn print_single_contract_report(state: &ThreeWayDriftState) {
    println!();
    println!("{}", "══════════════════════════════════════════════════════════════".bold());
    println!("  {}", "Three-Way WASM Drift Analysis".bold().cyan());
    println!("{}", "══════════════════════════════════════════════════════════════".bold());
    println!("  Contract ID:        {}", state.contract_id.bold());

    let status_str = match state.status {
        DriftStatus::Match => "MATCH (verified consistent)".bold().green(),
        DriftStatus::Drift => "DRIFT DETECTED (hashes diverge!)".bold().red(),
        DriftStatus::NotOnChain => "NOT ON CHAIN (not deployed)".bold().yellow(),
        DriftStatus::Unknown => "UNKNOWN (RPC check unavailable)".bold().yellow(),
    };
    println!("  Status:             {}", status_str);

    if let Some(ref leg) = state.diverged_leg {
        println!("  Diverged Leg:       {}", leg.bold().red());
    }

    println!();
    println!("  {}", "Three Legs of Truth:".bold().underline());
    if let Some(ref lock) = state.lockfile_hash {
        println!("    1. Lockfile Hash: {}", lock.dimmed());
    } else {
        println!("    1. Lockfile Hash: {}", "none (soroban-registry.lock.json not found)".dimmed());
    }
    println!("    2. Registry Hash: {}", state.registry_hash.dimmed());
    if let Some(ref chain) = state.onchain_hash {
        println!("    3. On-Chain Hash: {}", chain.dimmed());
    } else {
        println!("    3. On-Chain Hash: {}", "none / unreachable".dimmed());
    }

    println!();
    println!("  First Detected At:  {}", state.first_detected_at.to_rfc3339());
    println!("  Last Checked At:   {}", state.last_checked_at.to_rfc3339());
    if let Some(ledger) = state.observed_at_ledger {
        println!("  Observed Ledger:    {}", ledger);
    }
    println!("{}", "══════════════════════════════════════════════════════════════".bold());
    println!();
}

fn print_all_contracts_report(
    contracts: &[shared::ContractDriftRecord],
    total: i64,
    network: &str,
) {
    println!();
    println!("{}", "══════════════════════════════════════════════════════════════".bold());
    println!("  {} ({}, total: {})", "Registry Drift Status Report".bold().cyan(), network, total);
    println!("{}", "══════════════════════════════════════════════════════════════".bold());

    if contracts.is_empty() {
        println!("  {}", "No contracts found matching filter criteria.".green());
    } else {
        for c in contracts {
            let status_badge = match c.status {
                DriftStatus::Match => "[ MATCH ]".green().bold(),
                DriftStatus::Drift => "[ DRIFT ]".red().bold(),
                DriftStatus::NotOnChain => "[ NOT ON CHAIN ]".yellow().bold(),
                DriftStatus::Unknown => "[ UNKNOWN ]".yellow().bold(),
            };
            println!(
                "  {} {} | leg: {:?} | ledger: {:?}",
                status_badge,
                c.contract_id.bold(),
                c.diverged_leg.as_deref().unwrap_or("-"),
                c.observed_at_ledger.unwrap_or(0),
            );
        }
    }
    println!("{}", "══════════════════════════════════════════════════════════════".bold());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_lockfile_hash_missing_file() {
        let hash = load_lockfile_hash(Some("non_existent_lockfile.json"), "C123");
        assert_eq!(hash, None);
    }

    #[test]
    fn test_load_lockfile_hash_present() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("soroban-registry.lock.json");

        let content = serde_json::json!({
            "version": 1,
            "generated_at": "2026-09-16T12:00:00Z",
            "registry_url": "https://api.soroban-registry.org",
            "contracts": {
                "CB11111111111111111111111111111111111111111111111111111111": {
                    "contract_id": "CB11111111111111111111111111111111111111111111111111111111",
                    "hash": "9f2caaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }
            }
        });

        std::fs::write(&file_path, content.to_string()).unwrap();

        let loaded = load_lockfile_hash(
            file_path.to_str(),
            "CB11111111111111111111111111111111111111111111111111111111",
        );
        assert_eq!(
            loaded.as_deref(),
            Some("9f2caaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }
}

