//! `soroban-registry breaking-changes` — compare two contract versions.

use crate::support::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;

pub async fn breaking_changes(api_url: &str, old_id: &str, new_id: &str, json: bool) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!(
        "{}/api/contracts/breaking-changes?old_id={}&new_id={}",
        api_url, old_id, new_id
    );

    let response = client
        .get(&url)
        .send_with_retry()
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

    let breaking = crate::support::conversions::as_bool(&report["breaking"], "breaking")?;
    let breaking_count =
        crate::support::conversions::as_u64(&report["breaking_count"], "breaking_count")?;
    let non_breaking_count =
        crate::support::conversions::as_u64(&report["non_breaking_count"], "non_breaking_count")?;

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
            let severity = crate::support::conversions::as_str(&change["severity"], "severity")?;
            let message = crate::support::conversions::as_str(&change["message"], "message")?;
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
