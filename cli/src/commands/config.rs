//! `soroban-registry config` — per-environment contract configuration.

use crate::support::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;

pub async fn config_get(api_url: &str, contract_id: &str, environment: &str) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!(
        "{}/api/contracts/{}/config?environment={}",
        api_url, contract_id, environment
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch configuration")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to get config: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let config: serde_json::Value = response.json().await?;

    println!("\n{}", "Contract Configuration (Latest):".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract ID".bold(), contract_id);
    println!("{}: {}", "Environment".bold(), environment);
    println!(
        "{}: {}",
        "Version".bold(),
        crate::support::conversions::as_i64(&config["version"], "version")?
    );
    println!(
        "{}: {}",
        "Contains Secrets".bold(),
        crate::support::conversions::as_bool(&config["has_secrets"], "has_secrets")?
    );
    println!(
        "{}: {}",
        "Created By".bold(),
        crate::support::conversions::as_str(&config["created_by"], "created_by")?
    );
    println!("{}:", "Config Data".bold());
    println!(
        "{}",
        serde_json::to_string_pretty(&config["config_data"])
            .unwrap_or_default()
            .green()
    );
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
    let client = crate::support::net::client();
    let url = format!("{}/api/contracts/{}/config", api_url, contract_id);

    let mut payload = json!({
        "environment": environment,
        "config_data": serde_json::from_str::<serde_json::Value>(config_data).context("Invalid config JSON")?,
        "created_by": created_by,
    });

    if let Some(sec) = secrets_data {
        let sec_json: serde_json::Value =
            serde_json::from_str(sec).context("Invalid secrets JSON")?;
        payload["secrets_data"] = sec_json;
    }

    println!("\n{}", "Publishing configuration...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to set configuration")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to set config: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let config: serde_json::Value = response.json().await?;

    println!(
        "{}",
        "[OK] Configuration published successfully!".green().bold()
    );
    println!("  {}: {}", "Environment".bold(), environment);
    println!(
        "  {}: {}",
        "New Version".bold(),
        crate::support::conversions::as_i64(&config["version"], "version")?
    );
    println!();

    Ok(())
}

pub async fn config_history(api_url: &str, contract_id: &str, environment: &str) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!(
        "{}/api/contracts/{}/config/history?environment={}",
        api_url, contract_id, environment
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch configuration history")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to get config history: {}",
            response.text().await.unwrap_or_default()
        );
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
            crate::support::conversions::as_str(&config["created_at"], "created_at")?
                .bright_black(),
            crate::support::conversions::as_i64(&config["version"], "version")?,
            crate::support::conversions::as_str(&config["created_by"], "created_by")?.bright_blue()
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
    let client = crate::support::net::client();
    let url = format!(
        "{}/api/contracts/{}/config/rollback?environment={}",
        api_url, contract_id, environment
    );

    let payload = json!({
        "roll_back_to_version": version,
        "created_by": created_by,
    });

    println!(
        "\n{}",
        format!("Rolling back configuration to v{}...", version)
            .bold()
            .cyan()
    );

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to rollback configuration")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to rollback config: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let config: serde_json::Value = response.json().await?;

    println!(
        "{}",
        "[OK] Configuration rolled back successfully!"
            .green()
            .bold()
    );
    println!("  {}: {}", "Environment".bold(), environment);
    println!(
        "  {}: {}",
        "New Active Version".bold(),
        crate::support::conversions::as_i64(&config["version"], "version")?
    );
    println!();

    Ok(())
}
