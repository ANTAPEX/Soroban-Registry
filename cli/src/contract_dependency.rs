//! contract_dependency.rs — `soroban-registry contract dependency <ADDRESS>` (#836)
//!
//! Analyze a contract's dependencies: contracts it depends on, contracts that
//! depend on it, and a dependency tree with a configurable `--depth`.

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;

/// `soroban-registry contract dependency <ADDRESS> [--depth N] [--json]`
pub async fn run(api_url: &str, address: &str, depth: u32, json: bool) -> Result<()> {
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/dependencies?depth={}",
        api_url.trim_end_matches('/'),
        address,
        depth
    );
    log::debug!("GET {}", url);

    let resp = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to reach the registry API. Is the registry running?")?;
    let status = resp.status();
    let value: Value = resp.json().await.unwrap_or(Value::Null);
    if status.as_u16() == 404 {
        anyhow::bail!("no dependency data found for {}", address);
    }
    if !status.is_success() {
        anyhow::bail!("contract dependency failed ({}): {}", status, value);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("{} {}", "Dependencies for".bold(), address.cyan());

    let depends_on = value.get("dependsOn").and_then(Value::as_array).cloned().unwrap_or_default();
    println!("\n  {} ({})", "Depends on:".bold(), depends_on.len());
    for d in &depends_on {
        let addr = d.get("address").and_then(Value::as_str).unwrap_or("?");
        let name = d.get("name").and_then(Value::as_str).unwrap_or("");
        println!("    → {} {}", addr.cyan(), name.dimmed());
    }

    let dependents = value.get("dependents").and_then(Value::as_array).cloned().unwrap_or_default();
    println!("\n  {} ({})", "Depended on by:".bold(), dependents.len());
    for d in &dependents {
        let addr = d.get("address").and_then(Value::as_str).unwrap_or("?");
        let name = d.get("name").and_then(Value::as_str).unwrap_or("");
        println!("    ← {} {}", addr.cyan(), name.dimmed());
    }

    if let Some(tree) = value.get("tree") {
        println!("\n  {} (depth {})", "Dependency tree:".bold(), depth);
        print_tree(tree, 0);
    }
    Ok(())
}

fn print_tree(node: &Value, indent: usize) {
    let pad = "  ".repeat(indent + 2);
    let addr = node.get("address").and_then(Value::as_str).unwrap_or("?");
    println!("{}{}", pad, addr);
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            print_tree(child, indent + 1);
        }
    }
}
