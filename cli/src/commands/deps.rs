//! `soroban-registry deps` — a contract's dependencies.

use crate::support::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;

pub async fn deps_list(api_url: &str, contract_id: &str) -> Result<()> {
    let client = crate::support::net::client();
    let url = format!("{}/api/contracts/{}/dependencies", api_url, contract_id);

    let response = client
        .get(&url)
        .send_with_retry()
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

    fn print_tree(nodes: &[serde_json::Value], prefix: &str, is_last: bool) -> Result<()> {
        for (i, node) in nodes.iter().enumerate() {
            let name = node["name"].as_str().unwrap_or("Unknown");
            let constraint = node["constraint_to_parent"].as_str().unwrap_or("*");
            let contract_id = node["contract_id"].as_str().unwrap_or("");

            let name = crate::support::conversions::as_str(&node["name"], "name")?;
            let constraint = crate::support::conversions::as_str(
                &node["constraint_to_parent"],
                "constraint_to_parent",
            )?;
            let contract_id =
                crate::support::conversions::as_str(&node["contract_id"], "contract_id")?;

            let is_node_last = i == nodes.len() - 1;
            let marker = if is_node_last {
                "└──"
            } else {
                "├──"
            };

            println!(
                "{}{} {} ({}) {}",
                prefix,
                marker.bright_black(),
                name.bold(),
                constraint.cyan(),
                if contract_id == "unknown" {
                    "[Unresolved]".red()
                } else {
                    "".normal()
                }
            );

            if let Some(children) = node["dependencies"].as_array() {
                if !children.is_empty() {
                    let new_prefix =
                        format!("{}{}", prefix, if is_node_last { "    " } else { "│   " });
                    let _ = print_tree(children, &new_prefix, true);
                    let new_prefix =
                        format!("{}{}", prefix, if is_node_last { "    " } else { "│   " });
                    print_tree(children, &new_prefix, true)?;
                }
            }
        }
        Ok(())
    }

    print_tree(tree, "", false)?;

    println!("\n{}", "=".repeat(80).cyan());
    println!();
    Ok(())
}
