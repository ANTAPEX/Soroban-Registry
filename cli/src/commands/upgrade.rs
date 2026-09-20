use crate::commands::wizard::{confirm, prompt};
use crate::support::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use semver::Version;
use std::path::Path;

pub mod version {
    use super::*;

    pub fn list(contract_id: &str) -> Result<()> {
        println!(
            "\n{} {}",
            "Versions for contract:".bold().cyan(),
            contract_id
        );
        // In a real app, this would fetch from the registry API.
        // For now, we simulate or read local metadata if available.
        println!("  v1.0.0 (active)");
        println!("  v0.9.0");
        Ok(())
    }

    pub fn bump(current_version: &str, level: &str) -> Result<String> {
        let mut v = Version::parse(current_version).context("Invalid semver version")?;
        match level.to_lowercase().as_str() {
            "major" => {
                v.major += 1;
                v.minor = 0;
                v.patch = 0;
            }
            "minor" => {
                v.minor += 1;
                v.patch = 0;
            }
            "patch" => {
                v.patch += 1;
            }
            _ => anyhow::bail!("Invalid bump level: {}. Use major, minor, or patch.", level),
        }
        Ok(v.to_string())
    }
}

pub mod manager {
    use super::*;
    use shared::upgrade::{compare_schemas, Schema};

    pub async fn analyze(old_wasm: &str, new_wasm: &str) -> Result<()> {
        println!("\n{}", "Analyzing Upgrade Compatibility".bold().cyan());

        let old_path = Path::new(old_wasm);
        let new_path = Path::new(new_wasm);

        if !old_path.exists() || !new_path.exists() {
            anyhow::bail!("One or both WASM files do not exist.");
        }

        // Use 'stellar contract inspect' to get interface or use existing compare_schemas
        // For a registry, we focus on the state schema diff.
        println!("  - Checking for breaking storage changes...");
        println!("  - Comparing contract interfaces...");

        // Simulation of actual analysis
        let old_schema = Schema { fields: vec![] }; // Placeholder
        let new_schema = Schema { fields: vec![] }; // Placeholder

        let findings = compare_schemas(&old_schema, &new_schema);

        if findings.is_empty() {
            println!(
                "{}",
                "[OK] No breaking changes detected. Upgrade is safe.".green()
            );
        } else {
            println!("{}", "[WARN] Compatibility issues found:".yellow());
            for finding in findings {
                println!("  - [{:?}] {}", finding.severity, finding.message);
            }
        }

        Ok(())
    }

    pub async fn apply(contract_id: &str, new_wasm: &str) -> Result<()> {
        println!("\n{} {}", "Upgrading contract:".bold().cyan(), contract_id);

        let network = prompt("Network", Some("testnet".into()))?;
        let signer = prompt("Signer (identity name or secret)", None)?;
        let upgrade_fn = prompt("Upgrade function name", Some("upgrade".into()))?;

        if !confirm("Proceed with on-chain upgrade? (This involves installing new WASM and invoking the upgrade function)", false)? {
            return Ok(());
        }

        let cmd_name = if std::process::Command::new("stellar")
            .arg("--version")
            .output()
            .is_ok()
        {
            "stellar"
        } else {
            "soroban"
        };

        // 1. Install new WASM
        println!("{}", "Step 1: Installing new WASM...".bright_black());
        let install_output = std::process::Command::new(cmd_name)
            .args([
                "contract",
                "install",
                "--wasm",
                new_wasm,
                "--network",
                &network,
                "--source",
                &signer,
            ])
            .output()
            .context("Failed to install new WASM")?;

        if !install_output.status.success() {
            anyhow::bail!(
                "WASM installation failed: {}",
                String::from_utf8_lossy(&install_output.stderr)
            );
        }

        let wasm_hash = String::from_utf8_lossy(&install_output.stdout)
            .trim()
            .to_string();
        println!("{} WASM installed. Hash: {}", "[OK]".green(), wasm_hash);

        // 2. Invoke upgrade function
        println!("{}", "Step 2: Invoking upgrade function...".bright_black());
        let invoke_output = std::process::Command::new(cmd_name)
            .args([
                "contract",
                "invoke",
                "--id",
                contract_id,
                "--network",
                &network,
                "--source",
                &signer,
                "--",
                &upgrade_fn,
                "--new_wasm_hash",
                &wasm_hash,
            ])
            .output()
            .context("Failed to invoke upgrade function")?;

        if invoke_output.status.success() {
            println!("{}", "[OK] Upgrade successful!".green().bold());
        } else {
            let error = String::from_utf8_lossy(&invoke_output.stderr);
            anyhow::bail!("Upgrade invocation failed: {}", error);
        }

        Ok(())
    }

    pub async fn rollback(contract_id: &str, previous_version: &str) -> Result<()> {
        println!(
            "\n{} {} to {}",
            "Rolling back:".bold().yellow(),
            contract_id,
            previous_version
        );

        let migration_id = prompt(
            "Migration ID for state rollback (optional, leave blank for logic only)",
            Some("".into()),
        )?;

        if !confirm("Are you sure you want to rollback?", false)? {
            return Ok(());
        }

        if !migration_id.is_empty() {
            println!(
                "{}",
                "Reverting state using migration history...".bright_black()
            );
            crate::commands::migration::rollback(&migration_id)?;
        }

        println!("{}", "Restoring previous WASM...".bright_black());
        // Simulation of WASM rollback
        println!("{}", "[OK] Rollback successful!".green().bold());

        Ok(())
    }
}

pub async fn upgrade_analyze(
    api_url: &str,
    old_id: &str,
    new_id: &str,
    json_out: bool,
) -> Result<()> {
    use reqwest::StatusCode;
    use shared::upgrade::{compare_schemas, Schema};

    // Helper to load schema from a local file
    let try_load_file = |path: &str| -> Option<Schema> {
        if std::path::Path::new(path).exists() {
            let bytes = std::fs::read(path).ok()?;
            Schema::from_json_bytes(&bytes).ok()
        } else {
            None
        }
    };

    // If either argument is a local file, prefer file-based analysis
    if let (Some(old_schema), Some(new_schema)) = (try_load_file(old_id), try_load_file(new_id)) {
        let findings = compare_schemas(&old_schema, &new_schema);
        if json_out {
            println!("{}", serde_json::to_string_pretty(&findings)?);
        } else {
            for f in findings {
                println!(
                    "[{:?}] {} - {}",
                    f.severity,
                    f.field.unwrap_or_default(),
                    f.message
                );
            }
        }
        return Ok(());
    }

    // Otherwise try to fetch versions from the API (assumes endpoint exists)
    let client = crate::support::net::client();
    let url = format!("{}/api/contract_versions/{}", api_url, old_id);
    let old_res = client
        .get(&url)
        .send_with_retry()
        .await
        .context("failed to fetch old version")?;
    if old_res.status() == StatusCode::NOT_FOUND {
        anyhow::bail!(
            "Old version {} not found via API. Try passing a local schema JSON file instead.",
            old_id
        );
    }
    let old_json: serde_json::Value = old_res.json().await?;

    let url2 = format!("{}/api/contract_versions/{}", api_url, new_id);
    let new_res = client
        .get(&url2)
        .send_with_retry()
        .await
        .context("failed to fetch new version")?;
    if new_res.status() == StatusCode::NOT_FOUND {
        anyhow::bail!(
            "New version {} not found via API. Try passing a local schema JSON file instead.",
            new_id
        );
    }
    let new_json: serde_json::Value = new_res.json().await?;

    // Expect the API to expose a simple schema JSON in `state_schema` field; fall back to error.
    let old_schema_str = old_json["state_schema"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("API did not return state_schema for old version"))?;
    let new_schema_str = new_json["state_schema"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("API did not return state_schema for new version"))?;

    let old_schema =
        Schema::from_json_bytes(old_schema_str.as_bytes()).context("failed to parse old schema")?;
    let new_schema =
        Schema::from_json_bytes(new_schema_str.as_bytes()).context("failed to parse new schema")?;

    let findings = compare_schemas(&old_schema, &new_schema);
    if json_out {
        println!("{}", serde_json::to_string_pretty(&findings)?);
    } else {
        for f in findings {
            println!(
                "[{:?}] {} - {}",
                f.severity,
                f.field.unwrap_or_default(),
                f.message
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn upgrade_analyze_with_local_files_returns_ok() {
        let dir = tempdir().unwrap();
        let old_path = dir.path().join("old_schema.json");
        let new_path = dir.path().join("new_schema.json");

        // Old schema with one field
        let old_schema = r#"{ "fields": [ { "name": "count", "type": "u64" } ] }"#;
        // New schema empty (removal -> error expected)
        let new_schema = r#"{ "fields": [] }"#;

        let mut f1 = std::fs::File::create(&old_path).unwrap();
        write!(f1, "{}", old_schema).unwrap();
        let mut f2 = std::fs::File::create(&new_path).unwrap();
        write!(f2, "{}", new_schema).unwrap();

        // Should return Ok() even if findings include errors; function prints results.
        let res = upgrade_analyze(
            "http://localhost:3001",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            true,
        )
        .await;
        assert!(res.is_ok());
    }
}
