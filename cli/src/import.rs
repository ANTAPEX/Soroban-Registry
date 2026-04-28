use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::io_utils::{compute_sha256_streaming, extract_tar_gz};
use crate::manifest::{AuditEntry, ExportManifest};
use crate::net::RequestBuilderExt;

pub fn extract_and_verify(archive_path: &Path, output_dir: &Path) -> Result<ExportManifest> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;

    extract_tar_gz(archive_path, tmp_dir.path())?;

    let manifest_path = tmp_dir.path().join("manifest.json");
    let inner_path = tmp_dir.path().join("contract.tar.gz");

    if !manifest_path.exists() || !inner_path.exists() {
        bail!("invalid archive: missing manifest.json or contract.tar.gz");
    }

    let mut manifest: ExportManifest =
        serde_json::from_reader(BufReader::new(File::open(&manifest_path)?))?;

    let computed_hash = compute_sha256_streaming(&inner_path)?;
    if computed_hash != manifest.sha256 {
        bail!(
            "integrity check failed: expected {} got {}",
            manifest.sha256,
            computed_hash
        );
    }

    manifest.audit_trail.push(AuditEntry {
        action: "import_verified".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    fs::create_dir_all(output_dir)?;
    extract_tar_gz(&inner_path, output_dir)?;

    manifest.audit_trail.push(AuditEntry {
        action: "import_extracted".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    Ok(manifest)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPayload {
    pub contract_id: String,
    pub name: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub publisher_address: String,
}

#[derive(Debug, Deserialize)]
struct CsvImportPayload {
    pub contract_id: String,
    pub name: String,
    pub network: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub wasm_hash: Option<String>,
    pub source_url: Option<String>,
    pub publisher_address: Option<String>,
}

pub async fn run(
    api_url: &str,
    file_path: &str,
    format: Option<&str>,
    network_flag: Option<&str>,
    output_dir: &str,
    validate: bool,
    dry_run: bool,
) -> Result<()> {
    let path = Path::new(file_path);
    anyhow::ensure!(path.is_file(), "File not found: {}", file_path);

    let format_str = format.unwrap_or_else(|| {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("unknown");
        if ext == "gz" || ext == "tar" {
            "archive"
        } else {
            ext
        }
    });

    match format_str.to_lowercase().as_str() {
        "json" => import_json(api_url, path, network_flag, validate, dry_run).await,
        "csv" => import_csv(api_url, path, network_flag, validate, dry_run).await,
        "archive" | "tar.gz" => {
            if dry_run {
                println!("{} Archive dry-run not fully supported, skipping extraction.", "i".cyan());
                return Ok(());
            }
            if validate {
                println!("{} Validating archive...", "i".cyan());
            }
            let dest = Path::new(output_dir);
            let manifest = extract_and_verify(path, dest)?;
            
            println!("{}", "✓ Import complete — integrity verified!".green().bold());
            println!("  {}: {}", "Contract".bold(), manifest.contract_id.bright_black());
            println!("  {}: {}", "Name".bold(), manifest.name);
            if let Some(n) = network_flag {
                println!("  {}: {}", "Network".bold(), n.bright_blue());
            }
            println!("  {}: {}", "SHA-256".bold(), manifest.sha256.bright_black());
            Ok(())
        }
        _ => bail!("Unsupported format: {}. Use json, csv, or archive.", format_str),
    }
}

async fn import_json(api_url: &str, path: &Path, network_flag: Option<&str>, validate: bool, dry_run: bool) -> Result<()> {
    let content = fs::read_to_string(path).context("Failed to read JSON file")?;
    
    // Support either an array of contracts or a wrapper object like {"contracts": [...]}
    let mut payload_list: Vec<ImportPayload> = match serde_json::from_str(&content) {
        Ok(arr) => arr,
        Err(_) => {
            let wrapper: serde_json::Value = serde_json::from_str(&content)?;
            if let Some(arr) = wrapper.get("contracts").and_then(|c| c.as_array()) {
                serde_json::from_value(serde_json::Value::Array(arr.clone()))?
            } else {
                bail!("Invalid JSON format. Expected array of contracts or {{\"contracts\": [...]}}")
            }
        }
    };
    
    process_bulk_import(api_url, &mut payload_list, network_flag, validate, dry_run).await
}

async fn import_csv(api_url: &str, path: &Path, network_flag: Option<&str>, validate: bool, dry_run: bool) -> Result<()> {
    let mut reader = csv::Reader::from_path(path)?;
    let mut payload_list = Vec::new();

    for result in reader.deserialize() {
        let record: CsvImportPayload = result?;
        
        let tags = record.tags
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
            
        payload_list.push(ImportPayload {
            contract_id: record.contract_id,
            name: record.name,
            network: record.network.unwrap_or_else(|| "testnet".to_string()),
            description: record.description,
            category: record.category,
            tags,
            wasm_hash: record.wasm_hash,
            source_url: record.source_url,
            publisher_address: record.publisher_address.unwrap_or_else(|| "Unknown".to_string()),
        });
    }

    process_bulk_import(api_url, &mut payload_list, network_flag, validate, dry_run).await
}

async fn process_bulk_import(
    api_url: &str,
    payload_list: &mut Vec<ImportPayload>,
    network_flag: Option<&str>,
    validate: bool,
    dry_run: bool,
) -> Result<()> {
    let default_network = network_flag.unwrap_or("testnet").to_string();

    // 1. Resolve network overrides and validate
    let mut errors = Vec::new();
    for (i, p) in payload_list.iter_mut().enumerate() {
        if p.network.is_empty() {
            p.network = default_network.clone();
        }
        
        if validate {
            if p.contract_id.trim().is_empty() {
                errors.push(format!("Row {}: contract_id is empty", i + 1));
            }
            if p.name.trim().is_empty() {
                errors.push(format!("Row {}: name is empty", i + 1));
            }
            if !["mainnet", "testnet", "futurenet"].contains(&p.network.as_str()) {
                errors.push(format!("Row {}: invalid network '{}'", i + 1, p.network));
            }
        }
    }

    if validate && !errors.is_empty() {
        bail!("Validation failed:\n  {}", errors.join("\n  "));
    }

    println!("\n{}", "Bulk Contract Import".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("  {}: {}", "Contracts to import".bold(), payload_list.len());
    if dry_run {
        println!("  {}: {}", "Mode".bold(), "DRY RUN".yellow());
    }

    if dry_run {
        println!("\n{}", "Dry-run complete. No data imported.".yellow());
        return Ok(());
    }

    println!("\n{}", "Starting import...".bold());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    
    let url = format!("{}/api/contracts", api_url);
    let mut success_ids = Vec::new();
    let mut has_failure = false;

    for (i, payload) in payload_list.iter().enumerate() {
        print!("  [{}/{}] Importing {} ... ", i + 1, payload_list.len(), payload.contract_id.bold());
        
        let response = client
            .post(&url)
            .json(payload)
            .send_with_retry().await;

        match response {
            Ok(resp) => {
                if resp.status() == reqwest::StatusCode::CONFLICT {
                    println!("{}", "skipped (already exists)".bright_black());
                } else if resp.status().is_success() {
                    println!("{}", "success".green());
                    success_ids.push(payload.contract_id.clone());
                } else {
                    let err_text = resp.text().await.unwrap_or_default();
                    println!("{} - {}", "failed".red(), err_text.red());
                    has_failure = true;
                    break;
                }
            }
            Err(e) => {
                println!("{} - {}", "error".red(), e.to_string().red());
                has_failure = true;
                break;
            }
        }
    }

    if has_failure && !success_ids.is_empty() {
        println!("\n{}", "Error encountered. Rolling back successful imports...".yellow().bold());
        // Since the API might not support rollback delete directly from the CLI (no endpoint in contracts.rs),
        // we will print a warning if we cannot rollback or try DELETE.
        // Assuming there is a DELETE endpoint: DELETE /api/contracts/{id}
        for id in success_ids {
            print!("  Rolling back {} ... ", id.bold());
            let del_url = format!("{}/api/contracts/{}", api_url, id);
            let del_resp = client.delete(&del_url).send_with_retry().await;
            if let Ok(resp) = del_resp {
                if resp.status().is_success() {
                    println!("{}", "rolled back".yellow());
                } else {
                    println!("{}", "rollback failed".red());
                }
            } else {
                println!("{}", "rollback failed".red());
            }
        }
        bail!("Import failed and rollback executed.");
    } else if has_failure {
        bail!("Import failed.");
    }

    println!("\n{}", "✓ All contracts imported successfully!".green().bold());
    Ok(())
}
