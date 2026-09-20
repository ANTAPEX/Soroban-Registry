//! `soroban-registry stats` — registry-wide statistics.

use crate::support::net::RequestBuilderExt;
use crate::support::output_format;
use crate::support::stats_format::format_stats_table;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

/// Get comprehensive registry statistics
/// Command: soroban-registry stats [options]
pub async fn stats(
    api_url: &str,
    timeframe: &str,
    format: &str,
    output: Option<&str>,
) -> Result<()> {
    let fmt = output_format::validate_format(format).unwrap_or(output_format::OutputFormat::Table);

    let client = crate::support::net::client();
    let url = format!("{}/api/stats?timeframe={}", api_url, timeframe);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch registry statistics")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch stats: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let stats: serde_json::Value = response.json().await?;

    // Format output
    let output_str = match fmt {
        output_format::OutputFormat::Json => serde_json::to_string_pretty(&stats)?,
        output_format::OutputFormat::Yaml => serde_yaml::to_string(&stats)?,
        output_format::OutputFormat::Table => format_stats_table(&stats),
        output_format::OutputFormat::Csv => {
            let flat = serde_json::json!([stats]);
            output_format::render_csv(&flat)?
        }
    };

    if let Some(path) = output {
        fs::write(path, &output_str)?;
        println!("{} Stats written to {}", "[OK]".green(), path);
    } else {
        println!("{}", output_str);
    }

    Ok(())
}
