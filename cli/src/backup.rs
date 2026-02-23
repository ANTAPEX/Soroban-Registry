#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateBackupRequest {
    include_state: bool,
}

#[derive(Debug, Deserialize)]
struct ContractBackup {
    id: String,
    contract_id: String,
    backup_date: String,
    wasm_hash: String,
    storage_size_bytes: i64,
    verified: bool,
    primary_region: String,
    backup_regions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RestoreBackupRequest {
    backup_date: String,
}

#[derive(Debug, Deserialize)]
struct BackupRestoration {
    id: String,
    restore_duration_ms: i32,
    success: bool,
    restored_at: String,
}

pub async fn create_backup(api_url: &str, contract_id: &str, include_state: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let backup: ContractBackup = client
        .post(format!("{}/api/contracts/{}/backups", api_url, contract_id))
        .json(&CreateBackupRequest { include_state })
        .send()
        .await?
        .json()
        .await?;

    println!("✅ Backup created successfully");
    println!("   Date: {}", backup.backup_date);
    println!("   Size: {} bytes", backup.storage_size_bytes);
    println!("   Regions: {}", backup.backup_regions.join(", "));
    Ok(())
}

pub async fn list_backups(api_url: &str, contract_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let backups: Vec<ContractBackup> = client
        .get(format!("{}/api/contracts/{}/backups", api_url, contract_id))
        .send()
        .await?
        .json()
        .await?;

    println!("📦 Contract Backups (last 30 days)");
    println!("═══════════════════════════════════════════════════════");
    for backup in backups {
        let status = if backup.verified { "✓" } else { "○" };
        println!(
            "{} {} - {} bytes - {}",
            status, backup.backup_date, backup.storage_size_bytes, backup.primary_region
        );
    }
    Ok(())
}

pub async fn restore_backup(api_url: &str, contract_id: &str, backup_date: &str) -> Result<()> {
    let client = reqwest::Client::new();

    println!("🔄 Restoring backup from {}...", backup_date);

    let restoration: BackupRestoration = client
        .post(format!(
            "{}/api/contracts/{}/backups/restore",
            api_url, contract_id
        ))
        .json(&RestoreBackupRequest {
            backup_date: backup_date.to_string(),
        })
        .send()
        .await?
        .json()
        .await?;

    if restoration.success {
        println!("✅ Restoration completed successfully");
        println!("   Duration: {}ms", restoration.restore_duration_ms);
        println!("   Restored at: {}", restoration.restored_at);
    } else {
        println!("❌ Restoration failed");
    }
    Ok(())
}

pub async fn verify_backup(api_url: &str, contract_id: &str, backup_date: &str) -> Result<()> {
    let client = reqwest::Client::new();
    client
        .post(format!(
            "{}/api/contracts/{}/backups/{}/verify",
            api_url, contract_id, backup_date
        ))
        .send()
        .await?;

    println!("✅ Backup verified: {}", backup_date);
    Ok(())
}

pub async fn backup_stats(api_url: &str, contract_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let stats: serde_json::Value = client
        .get(format!(
            "{}/api/contracts/{}/backups/stats",
            api_url, contract_id
        ))
        .send()
        .await?
        .json()
        .await?;

    println!("📊 Backup Statistics");
    println!("═══════════════════════════════════════════════════════");
    println!("Total backups: {}", stats["total_backups"]);
    println!("Verified: {}", stats["verified_backups"]);
    println!("Total size: {} bytes", stats["total_size_bytes"]);
    if let Some(latest) = stats["latest_backup"].as_str() {
        println!("Latest backup: {}", latest);
    }
    Ok(())
}
