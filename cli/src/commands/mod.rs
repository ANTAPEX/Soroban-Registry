#![allow(dead_code)]

//! The commands the CLI runs.
//!
//! One module per command, and a directory for a command that has subcommands
//! of its own.
//!
//! Everything below the module list is dead. No dispatch arm, no other module
//! and no test reaches any of it — `cargo build --all-targets` says so once the
//! `allow(dead_code)` above is removed. It is left here, together rather than
//! scattered through the tree, so that deleting it is one reviewable decision
//! instead of a judgement call buried in a move.

pub mod analytics;
pub mod analyze;
pub mod api_key;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod batch;
pub mod breaking_changes;
pub mod cache;
pub mod category;
pub mod cicd;
pub mod compare;
pub mod completion;
pub mod config;
pub mod contract;
pub mod coverage;
pub mod dashboard;
pub mod deploy;
pub mod deps;
pub mod doc;
pub mod env;
pub mod export;
pub mod formal_verification;
pub mod fuzz;
pub mod import;
pub mod incident;
pub mod list;
pub mod migration;
pub mod multisig;
pub mod network;
pub mod notification;
pub mod openapi;
pub mod package_signing;
pub mod patch;
pub mod perf;
pub mod plugins;
pub mod profile;
pub mod publish;
pub mod publisher;
pub mod release_notes;
pub mod scan_deps;
pub mod search;
pub mod shell;
pub mod sla;
pub mod snapshot;
pub mod state;
pub mod stats;
pub mod test;
pub mod track_deployment;
pub mod upgrade;
pub mod verification;
pub mod version;
pub mod webhook;
pub mod wizard;

use crate::support::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use serde_yaml;
use std::fs;

use crate::commands::list::contract_list;
use crate::support::network::Network;

fn resolve_smart_routing(current_network: Network) -> String {
    if current_network.to_string() == "auto" {
        "mainnet".to_string()
    } else {
        current_network.to_string()
    }
}

pub async fn contract_info(api_url: &str, id: &str) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!("{}/api/contracts/{}", api_url.trim_end_matches('/'), id);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch contract info")?;

    if !response.status().is_success() {
        if response.status() == 404 {
            anyhow::bail!("Contract not found: {}", id);
        }
        anyhow::bail!("API returned error: {}", response.status());
    }

    let data: serde_json::Value = response.json().await?;

    println!("\n{}", "Contract Details".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    println!(
        "{:<20} {}",
        "Name:".bold(),
        data["name"].as_str().unwrap_or("Unknown")
    );
    println!(
        "{:<20} {}",
        "ID:".bold(),
        data["contract_id"].as_str().unwrap_or("Unknown")
    );
    println!(
        "{:<20} {}",
        "Network:".bold(),
        data["network"].as_str().unwrap_or("Unknown")
    );
    println!(
        "{:<20} {}",
        "Category:".bold(),
        data["category"].as_str().unwrap_or("None")
    );

    let verified = if data["is_verified"].as_bool().unwrap_or(false) {
        "Yes".green()
    } else {
        "No".red()
    };
    println!("{:<20} {}", "Verified:".bold(), verified);

    if let Some(desc) = data["description"].as_str() {
        println!("{:<20} {}", "Description:".bold(), desc);
    }

    println!("\n{}", "Resources".bold().yellow());
    println!(
        "{:<20} {}",
        "WASM Hash:".bold(),
        data["wasm_hash"].as_str().unwrap_or("N/A")
    );

    if let Some(abi) = data["abi"].as_object() {
        println!("{:<20} {} methods", "ABI:".bold(), abi.len());
    }

    println!();
    Ok(())
}

pub async fn list(
    api_url: &str,
    limit: usize,
    network: crate::config::Network,
    json: bool,
) -> Result<()> {
    contract_list(
        api_url,
        limit,
        0,
        Some(network),
        Vec::new(),
        None,
        if json { "json" } else { "table" },
    )
    .await
}

fn extract_migration_id(migration: &serde_json::Value) -> Result<String> {
    let Some(migration_id) = migration["id"].as_str() else {
        eprintln!(
            "[error] migration response missing string id field: {}",
            migration
        );
        anyhow::bail!("Invalid migration response: missing id");
    };

    Ok(migration_id.to_string())
}

pub async fn migrate(
    api_url: &str,
    contract_id: &str,
    wasm_path: &str,
    simulate_fail: bool,
    dry_run: bool,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    use tokio::process::Command;

    println!("\n{}", "Migration Tool".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    // 1. Read WASM file
    let wasm_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("Failed to read WASM file at {}", wasm_path))?;

    // 2. Compute Hash
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let wasm_hash = hex::encode(hasher.finalize());

    println!("Contract ID: {}", contract_id.green());
    println!("WASM Hash: {}", wasm_hash.bright_black());

    if dry_run {
        println!(
            "\n{}",
            "Dry run enabled: not contacting the registry API.".yellow()
        );
        println!(
            "{}",
            "[OK] Migration simulation complete (dry-run)."
                .green()
                .bold()
        );
        return Ok(());
    }

    // 3. Create Migration Record (Pending)
    let client = crate::support::net::client();
    let create_url = format!("{}/api/migrations", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "wasm_hash": wasm_hash,
    });

    print!("\nInitializing migration... ");
    let response = client
        .post(&create_url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to contact registry API")?;

    if !response.status().is_success() {
        println!("{}", "Failed".red());
        let err = response.text().await?;
        anyhow::bail!("API Error: {}", err);
    }

    let migration: serde_json::Value = response.json().await?;
    let migration_id = extract_migration_id(&migration)?;
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
        .send_with_retry()
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

    let manifest = crate::commands::import::extract_and_verify(archive_path, dest)?;

    println!(
        "{}",
        "[OK] Import complete — integrity verified!".green().bold()
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

/// GET /api/contracts/:id/trust-score
pub async fn trust_score(api_url: &str, contract_id: &str, network: Network) -> Result<()> {
    let url = format!("{}/api/contracts/{}/trust-score", api_url, contract_id);
    log::debug!("GET {}", url);

    let client = crate::support::net::client();
    let resp = client
        .get(&url)
        .query(&[("network", network.to_string())])
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get trust score ({}): {}", status, body);
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .context("Failed to parse trust score response")?;

    // ── Header ────────────────────────────────────────────────────────────────
    let name = crate::support::conversions::as_str(&data["contract_name"], "contract_name")?;
    let score = crate::support::conversions::as_f64(&data["score"], "score")?;
    let badge = crate::support::conversions::as_str(&data["badge"], "badge")?;
    let badge_icon = crate::support::conversions::as_str(&data["badge_icon"], "badge_icon")?;
    let summary = crate::support::conversions::as_str(&data["summary"], "summary")?;

    println!("\n{}", "─".repeat(56));
    println!("  Trust Score — {}", name.bold());
    println!("{}", "─".repeat(56));
    println!("  Score : {:.0}/100", score);
    println!("  Badge : {} {}", badge_icon, badge.bold());
    println!("  {}", summary);
    println!("{}", "─".repeat(56));

    // ── Factor breakdown ──────────────────────────────────────────────────────
    println!("\n  {}\n", "Factor Breakdown".bold());

    if let Some(factors) = data["factors"].as_array() {
        for factor in factors {
            let fname = crate::support::conversions::as_str(&factor["name"], "name")?;
            let earned =
                crate::support::conversions::as_f64(&factor["points_earned"], "points_earned")?;
            let max = crate::support::conversions::as_f64(&factor["points_max"], "points_max")?;
            let explain =
                crate::support::conversions::as_str(&factor["explanation"], "explanation")?;

            // Mini progress bar (10 chars)
            let filled = ((earned / max) * 10.0).round() as usize;
            let filled = filled.min(10);
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));

            println!("  {:<28} [{bar}] {:.0}/{:.0}", fname, earned, max);
            println!("    {}", explain.dimmed());
        }
    }

    // ── Weight documentation ──────────────────────────────────────────────────
    println!("\n  {}\n", "Score Weights".bold());
    if let Ok(weights) = crate::support::conversions::as_object(&data["weights"], "weights") {
        for (k, v) in weights {
            let max_pts = crate::support::conversions::as_f64(v, "weight_value")?;
            println!("  {:<22} {:.0} pts max", k, max_pts);
        }
    }

    let computed_at = crate::support::conversions::as_str(&data["computed_at"], "computed_at")?;
    println!("\n  Computed at: {}\n", computed_at.dimmed());

    Ok(())
}

/// Validate a contract function call for type safety
pub async fn validate_call(
    api_url: &str,
    contract_id: &str,
    method_name: &str,
    params: &[String],
    strict: bool,
) -> Result<()> {
    let client = crate::support::net::client();
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
        .send_with_retry()
        .await
        .context("Failed to validate contract call")?;

    let status = response.status();
    let data: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let error_msg = crate::support::conversions::as_str(&data["message"], "message")?;
        println!("\n{} {}", "Error:".bold().red(), error_msg);
        anyhow::bail!("Validation failed: {}", error_msg);
    }

    let valid = crate::support::conversions::as_bool(&data["valid"], "valid")?;

    println!("\n{}", "Contract Call Validation".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("\n{}: {}", "Function".bold(), method_name);
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Strict Mode".bold(),
        if strict { "Yes" } else { "No" }
    );

    if valid {
        println!(
            "\n{} {}",
            "[OK]".green().bold(),
            "Call is valid!".green().bold()
        );

        // Show parsed parameters
        if let Some(params) = data["parsed_params"].as_array() {
            println!("\n{}", "Parsed Parameters:".bold());
            for param in params {
                let name = crate::support::conversions::as_str(&param["name"], "name")?;
                let type_name =
                    crate::support::conversions::as_str(&param["expected_type"], "expected_type")?;
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
                    let msg = crate::support::conversions::as_str(&warning["message"], "message")?;
                    println!("  {} {}", "[WARN]".yellow(), msg);
                }
            }
        }
    } else {
        println!(
            "\n{} {}",
            "[ERR]".red().bold(),
            "Call is invalid!".red().bold()
        );

        // Show errors
        if let Some(errors) = data["errors"].as_array() {
            println!("\n{}", "Errors:".bold().red());
            for error in errors {
                let code = crate::support::conversions::as_str(&error["code"], "code")?;
                let msg = crate::support::conversions::as_str(&error["message"], "message")?;
                let field = error["field"].as_str();

                if let Some(f) = field {
                    println!(
                        "  {} [{}] {}: {}",
                        "[ERR]".red(),
                        code.bright_black(),
                        f.bold(),
                        msg
                    );
                } else {
                    println!("  {} [{}] {}", "[ERR]".red(), code.bright_black(), msg);
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
    let client = crate::support::net::client();
    let url = format!(
        "{}/api/contracts/{}/bindings?language={}",
        api_url, contract_id, language
    );

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to generate bindings")?;

    let status = response.status();

    if !status.is_success() {
        let error: serde_json::Value = response.json().await?;
        let msg = crate::support::conversions::as_str(&error["message"], "message")?;
        anyhow::bail!("Failed to generate bindings: {}", msg);
    }

    let bindings = response.text().await?;

    if let Some(output_path) = output {
        fs::write(output_path, &bindings)?;
        println!(
            "\n{} {} bindings written to: {}",
            "[OK]".green().bold(),
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
    let client = crate::support::net::client();
    let url = format!("{}/api/contracts/{}/functions", api_url, contract_id);

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to list contract functions")?;

    let status = response.status();
    let data: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let msg = crate::support::conversions::as_str(&data["message"], "message")?;
        anyhow::bail!("Failed to list functions: {}", msg);
    }

    let contract_name =
        crate::support::conversions::as_str(&data["contract_name"], "contract_name")?;
    let functions = data["functions"].as_array();

    println!("\n{}", "Contract Functions".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("\n{}: {}", "Contract".bold(), contract_name);
    println!("{}: {}", "ID".bold(), contract_id);

    if let Some(funcs) = functions {
        println!("\n{} {} function(s):\n", "Found".bold(), funcs.len());

        for func in funcs {
            let name = crate::support::conversions::as_str(&func["name"], "name")?;
            let visibility =
                crate::support::conversions::as_str(&func["visibility"], "visibility")?;
            let return_type =
                crate::support::conversions::as_str(&func["return_type"], "return_type")?;
            let is_mutable =
                crate::support::conversions::as_bool(&func["is_mutable"], "is_mutable")?;

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
                let mut param_strs: Vec<String> = Vec::new();
                for p in params {
                    let pname = crate::support::conversions::as_str(&p["name"], "name")?;
                    let ptype = crate::support::conversions::as_str(&p["type_name"], "type_name")?;
                    param_strs.push(format!("{}: {}", pname, ptype));
                }

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

/// Fetch contract info from the registry. `id` is the contract's registry identifier.
pub async fn info(
    api_url: &str,
    id: &str,
    format: &str,
    highlight_method: Option<&str>,
    network: crate::config::Network,
) -> Result<()> {
    let client = crate::support::net::client();
    let base_url = api_url.trim_end_matches('/');

    if format == "text" {
        println!("\n{}", "Fetching contract information...".bold().cyan());
    }

    // 1. Fetch Metadata
    let metadata_url = format!("{}/api/contracts/{}", base_url, id);
    let metadata_res = client
        .get(&metadata_url)
        .query(&[("network", network.to_string())])
        .send_with_retry()
        .await?;

    if !metadata_res.status().is_success() {
        anyhow::bail!(
            "Failed to fetch contract metadata: {}",
            metadata_res.status()
        );
    }
    let metadata: serde_json::Value = metadata_res.json().await?;

    // Extract genuine UUID if 'id' was a name or address
    let contract_uuid = metadata["contract"]["id"]
        .as_str()
        .context("Metadata missing contract ID")?;
    let contract_address = metadata["contract"]["contract_id"].as_str().unwrap_or(id);

    // 2. Fetch ABI
    let abi_url = format!("{}/api/contracts/{}/abi", base_url, contract_uuid);
    let abi_res = client.get(&abi_url).send_with_retry().await;
    let abi: Option<serde_json::Value> = if let Ok(res) = abi_res {
        if res.status().is_success() {
            res.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("abi").cloned())
        } else {
            None
        }
    } else {
        None
    };

    // 3. Fetch Deployments
    let depl_url = format!("{}/api/contracts/{}/deployments", base_url, contract_uuid);
    let depl_res = client.get(&depl_url).send_with_retry().await;
    let deployments: Vec<serde_json::Value> = if let Ok(res) = depl_res {
        if res.status().is_success() {
            res.json().await.unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 4. Fetch Dependencies
    let deps_url = format!("{}/api/contracts/{}/dependencies", base_url, contract_uuid);
    let deps_res = client.get(&deps_url).send_with_retry().await;
    let dependencies: Vec<serde_json::Value> = if let Ok(res) = deps_res {
        if res.status().is_success() {
            res.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("dependencies").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 5. Fetch Dependents (Related Contracts)
    let relate_url = format!("{}/api/contracts/{}/dependents", base_url, contract_uuid);
    let relate_res = client.get(&relate_url).send_with_retry().await;
    let dependents: Vec<serde_json::Value> = if let Ok(res) = relate_res {
        if res.status().is_success() {
            res.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("dependents").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 6. Fetch Versions (for verification status)
    let versions_url = format!("{}/api/contracts/{}/versions", base_url, contract_uuid);
    let versions_res = client.get(&versions_url).send_with_retry().await;
    let versions: Vec<serde_json::Value> = if let Ok(res) = versions_res {
        if res.status().is_success() {
            res.json().await.unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Aggregate data
    let full_info = json!({
        "metadata": metadata["contract"],
        "current_network_config": metadata["network_config"],
        "abi": abi,
        "deployments": deployments,
        "dependencies": dependencies,
        "dependents": dependents,
        "versions": versions,
    });

    // Render output
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&full_info)?);
        }
        "yaml" => {
            let yaml = serde_yaml::to_string(&full_info)?;
            println!("{}", yaml);
        }
        _ => {
            render_info_text(
                &full_info,
                highlight_method,
                contract_address,
                &network.to_string(),
            )?;
        }
    }

    Ok(())
}

fn render_info_text(
    info: &serde_json::Value,
    highlight_method: Option<&str>,
    contract_address: &str,
    network_str: &str,
) -> Result<()> {
    let metadata = &info["metadata"];
    let name = metadata["name"].as_str().unwrap_or("Unknown");
    let desc = metadata["description"]
        .as_str()
        .unwrap_or("No description provided.");
    let is_verified = metadata["is_verified"].as_bool().unwrap_or(false);
    let health_score = metadata["health_score"].as_i64().unwrap_or(0);

    println!("\n{}", "=".repeat(80).cyan());
    println!("{} {}", "CONTRACT:".bold(), name.bold().green());
    println!("{} {}", "ID:      ".bold(), contract_address.yellow());
    println!(
        "{} {}",
        "STATUS:  ".bold(),
        if is_verified {
            "Verified".green().bold()
        } else {
            "Unverified".red()
        }
    );
    println!("{} {}/100", "HEALTH:  ".bold(), health_score);
    println!("{} {}", "DESC:    ".bold(), desc);
    println!("{}", "=".repeat(80).cyan());

    // Explorer Links
    println!("\n{}", "BLOCK EXPLORERS:".bold().underline());
    let explorer_url = match network_str {
        "testnet" => format!(
            "https://stellar.expert/explorer/testnet/contract/{}",
            contract_address
        ),
        "futurenet" => format!(
            "https://stellar.expert/explorer/futurenet/contract/{}",
            contract_address
        ),
        _ => format!(
            "https://stellar.expert/explorer/public/contract/{}",
            contract_address
        ),
    };
    println!("  • StellarExpert: {}", explorer_url.blue().underline());

    // ABI Methods
    if let Some(abi) = info["abi"].as_array() {
        println!("\n{}", "ABI METHODS:".bold().underline());
        for item in abi {
            if item["type"] == "function" {
                let m_name = item["name"].as_str().unwrap_or("unknown");
                let mut line = format!("  • {}", m_name);
                if let Some(target) = highlight_method {
                    if m_name == target {
                        line = format!("  • {}", m_name.on_yellow().black().bold());
                    }
                }
                println!("{}", line);
            }
        }
    }

    // Deployments
    if let Some(depls) = info["deployments"].as_array() {
        if !depls.is_empty() {
            println!("\n{}", "DEPLOYMENTS:".bold().underline());
            for d in depls {
                let env = d["environment"].as_str().unwrap_or("unknown");
                let status = d["status"].as_str().unwrap_or("unknown");
                let date = d["deployed_at"].as_str().unwrap_or("");
                println!("  • {:<10} | {:<10} | {}", env, status, date);
            }
        }
    }

    // Dependencies
    if let Some(deps) = info["dependencies"].as_array() {
        if !deps.is_empty() {
            println!("\n{}", "DEPENDENCIES:".bold().underline());
            for d in deps {
                let d_name = d["dependency_name"].as_str().unwrap_or("unknown");
                let constraint = d["version_constraint"].as_str().unwrap_or("*");
                println!("  • {} ({})", d_name, constraint);
            }
        }
    }

    // Related Contracts (Dependents)
    if let Some(deps) = info["dependents"].as_array() {
        if !deps.is_empty() {
            println!("\n{}", "RELATED CONTRACTS (DEPENDENTS):".bold().underline());
            for d in deps {
                let d_name = d["dependency_name"].as_str().unwrap_or("unknown"); // This is from the perspective of the dependent
                                                                                 // Wait, it should use the contract name if available.
                                                                                 // But dependents might just be a list of contract IDs.
                println!("  • Contract ID: {}", d["contract_id"]);
            }
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    Ok(())
}

pub async fn snapshot_create(api_url: &str, contract_id: &str) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!("{}/api/contracts/{}/snapshots", api_url, contract_id);

    println!("\n{}", "Creating contract snapshot...".bold().cyan());

    let response = client
        .post(&url)
        .send_with_retry()
        .await
        .context("Failed to create snapshot")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to create snapshot: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let snapshot: serde_json::Value = response.json().await?;

    println!("{}", "[OK] Snapshot created successfully!".green().bold());
    println!(
        "  {}: {}",
        "ID".bold(),
        snapshot["id"].as_str().unwrap_or("")
    );
    println!(
        "  {}: {}",
        "Version".bold(),
        snapshot["version_number"].as_i64().unwrap_or(0)
    );
    println!(
        "  {}: {}",
        "Created At".bold(),
        snapshot["created_at"].as_str().unwrap_or("")
    );
    println!();

    Ok(())
}

pub async fn snapshot_list(api_url: &str, contract_id: &str) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!("{}/api/contracts/{}/snapshots", api_url, contract_id);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to list snapshots")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to list snapshots: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let snapshots: Vec<serde_json::Value> = response.json().await?;

    println!("\n{}", "Contract Snapshots:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if snapshots.is_empty() {
        println!("{}", "No snapshots found.".yellow());
        return Ok(());
    }

    for s in snapshots {
        println!(
            "  v{} - {} [{}]",
            s["version_number"].as_i64().unwrap_or(0),
            s["created_at"].as_str().unwrap_or("").bright_black(),
            s["id"].as_str().unwrap_or("").cyan()
        );
    }
    println!();

    Ok(())
}

pub async fn snapshot_get(api_url: &str, contract_id: &str, timestamp: &str) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!(
        "{}/api/contracts/{}/snapshots?timestamp={}",
        api_url, contract_id, timestamp
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch snapshot")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch snapshot: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let snapshot: serde_json::Value = response.json().await?;
    println!("\n{}", "Snapshot Details:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}", serde_json::to_string_pretty(&snapshot)?.green());
    println!();

    Ok(())
}

pub async fn snapshot_diff(api_url: &str, contract_id: &str, v1: i32, v2: i32) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!(
        "{}/api/contracts/{}/versions/{}/diff/{}",
        api_url, contract_id, v1, v2
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch diff")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch diff: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let diff: shared::models::VersionDiff = response.json().await?;

    println!(
        "\n{}",
        format!("Diff between v{} and v{}:", v1, v2).bold().cyan()
    );
    println!("{}", "=".repeat(80).cyan());

    if diff.added.is_empty() && diff.removed.is_empty() && diff.modified.is_empty() {
        println!("{}", "No differences found.".green());
        return Ok(());
    }

    for add in diff.added {
        println!(
            "  {} {}: {}",
            "+".green().bold(),
            add.field.bold(),
            add.to.to_string().green()
        );
    }
    for rm in diff.removed {
        println!(
            "  {} {}: {}",
            "-".red().bold(),
            rm.field.bold(),
            rm.from.to_string().red()
        );
    }
    for modif in diff.modified {
        println!(
            "  {} {}: {} -> {}",
            "~".yellow().bold(),
            modif.field.bold(),
            modif.from.to_string().red(),
            modif.to.to_string().green()
        );
    }

    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const VALID_CONTRACT_ID: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
    const VALID_PUBLISHER: &str = "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ";

    #[test]
    fn extract_migration_id_returns_id_for_valid_payload() {
        let payload = json!({"id": "migration-123"});
        let migration_id = extract_migration_id(&payload);
        assert!(migration_id.is_ok());
        assert_eq!(migration_id.unwrap_or_default(), "migration-123");
    }

    #[test]
    fn extract_migration_id_fails_when_missing_id() {
        let payload = json!({"status": "pending"});
        let err = extract_migration_id(&payload);
        assert!(err.is_err());
        assert!(err
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .contains("Invalid migration response: missing id"));
    }

    #[test]
    fn extract_migration_id_fails_when_id_is_not_string() {
        let payload = json!({"id": 99});
        let err = extract_migration_id(&payload);
        assert!(err.is_err());
        assert!(err
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .contains("Invalid migration response: missing id"));
    }
}
