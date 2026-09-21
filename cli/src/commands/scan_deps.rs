//! `soroban-registry scan-deps` — scan a contract's dependencies for advisories.

use crate::commands::patch::Severity;
use crate::support::net::RequestBuilderExt;
use crate::support::severity::severity_colored;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;

pub async fn scan_deps(
    api_url: &str,
    contract_id: &str,
    dependencies: &str,
    fail_on_high: bool,
) -> Result<()> {
    println!("\n{}", "Scanning Dependencies...".bold().cyan());

    let client = crate::support::net::client();
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
        .send_with_retry()
        .await
        .context("Failed to run dependency scan")?;

    if !response.status().is_success() {
        anyhow::bail!("Scan failed: {}", response.text().await.unwrap_or_default());
    }

    let report: serde_json::Value = response.json().await?;
    let findings = crate::support::conversions::as_array(&report["findings"], "findings")?;

    if findings.is_empty() {
        println!("{}", "[OK] No vulnerabilities found!".green().bold());
        return Ok(());
    }

    let mut has_high_severity = false;
    println!("\n{}", "Vulnerabilities Found:".bold().red());
    println!("{}", "=".repeat(80).red());

    for finding in findings {
        let package =
            crate::support::conversions::as_str(&finding["package_name"], "package_name")?;
        let version =
            crate::support::conversions::as_str(&finding["current_version"], "current_version")?;
        let severity = crate::support::conversions::as_str(&finding["severity"], "severity")?;
        let cve_id = crate::support::conversions::as_str(&finding["cve_id"], "cve_id")?;
        let recommended = crate::support::conversions::as_str(
            &finding["recommended_version"],
            "recommended_version",
        )?;

        let sev_enum = severity
            .parse::<Severity>()
            .context("Invalid severity string")?;
        if matches!(sev_enum, Severity::Critical | Severity::High) {
            has_high_severity = true;
        }

        println!(
            "  {} {}@{} - {}",
            severity_colored(&sev_enum),
            package,
            version,
            cve_id.bold()
        );
        println!(
            "    {} Recommended patch: {}",
            "↳".bright_black(),
            recommended.green()
        );
    }

    println!("\n{}", "=".repeat(80).red());
    println!("{} issue(s) detected\n", findings.len());

    if fail_on_high && has_high_severity {
        std::process::exit(1);
    }

    Ok(())
}
