//! `soroban-registry list` — list contracts in the registry.

use crate::support::contract_view::contract_json;
use crate::support::filters::normalize_filter_values;
use crate::support::network::normalize_network_filters;
use crate::support::network::shared_network;
use crate::support::network::shared_networks;
use crate::support::truncate::Truncate;
use anyhow::{Context, Result};
use colored::Colorize;
use registry_client::ContractSearchParams;

pub async fn contract_list(
    api_url: &str,
    limit: usize,
    offset: usize,
    network: Option<crate::config::Network>,
    networks: Vec<String>,
    category: Option<String>,
    format: &str,
) -> Result<()> {
    // Normalize before sending so comma-separated `--networks`/`--category` and
    // the singular `--network` flag produce the same request shape as `search`,
    // and unknown networks fail here rather than being silently dropped server-side.
    let networks = normalize_network_filters(&networks)?;
    let categories = normalize_filter_values(&category.into_iter().collect::<Vec<_>>());

    let mut params = ContractSearchParams {
        limit: Some(limit as i64),
        offset: Some(offset as i64),
        ..Default::default()
    };
    if !networks.is_empty() {
        params.networks = Some(shared_networks(&networks)?);
    } else if let Some(net) = network {
        params.network = shared_network(&net.to_string());
    }
    if !categories.is_empty() {
        params.categories = Some(categories.clone());
    }

    let page = crate::support::registry::client(api_url)
        .await?
        .list_contracts(&params)
        .await
        .context("Failed to list contracts")?;

    let items = &page.items;

    if format == "json" {
        let contracts: Vec<serde_json::Value> = items.iter().map(contract_json).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contracts": contracts,
                "count": contracts.len()
            }))?
        );
        return Ok(());
    }

    if format == "csv" {
        println!("contract_id,name,network,is_verified,category");
        for item in items {
            println!(
                "{},{},{},{},{}",
                item.contract_id,
                item.name,
                item.network,
                item.is_verified,
                item.category.as_deref().unwrap_or("")
            );
        }
        return Ok(());
    }

    // Default: Table
    println!("\n{}", "Contract Registry".bold().cyan());
    println!("{}", "=".repeat(100).cyan());

    if items.is_empty() {
        println!("{}", "No contracts found matching the criteria.".yellow());
        return Ok(());
    }

    println!(
        "{:<45} {:<25} {:<10} {:<10}",
        "CONTRACT ID".bold(),
        "NAME".bold(),
        "NETWORK".bold(),
        "VERIFIED".bold()
    );
    println!("{}", "-".repeat(100));

    for item in items {
        let verified = if item.is_verified {
            "Yes".green()
        } else {
            "No".red()
        };

        println!(
            "{:<45} {:<25} {:<10} {:<10}",
            item.contract_id,
            item.name.truncate_str(23),
            item.network.to_string(),
            verified
        );
    }

    println!("{}", "-".repeat(100));
    println!(
        "Showing {}-{} of {} contracts",
        offset + 1,
        offset + items.len(),
        page.total
    );
    println!();

    Ok(())
}
