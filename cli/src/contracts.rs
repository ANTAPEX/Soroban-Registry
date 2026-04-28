use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use std::cmp::Ordering;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractListItem {
    pub id: String,
    pub name: String,
    pub contract_id: String,
    pub network: String,
    pub category: Option<String>,
    pub is_verified: bool,
    pub health_score: i32,
    pub created_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub enum SortBy {
    Name,
    CreatedAt,
    HealthScore,
    Network,
}

impl std::str::FromStr for SortBy {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "name" => Ok(SortBy::Name),
            "created_at" | "created-at" => Ok(SortBy::CreatedAt),
            "health_score" | "health-score" => Ok(SortBy::HealthScore),
            "network" => Ok(SortBy::Network),
            _ => Err(format!(
                "Invalid sort-by value: {}. Supported: name, created_at, health_score, network",
                s
            )),
        }
    }
}

impl std::str::FromStr for SortOrder {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "asc" | "ascending" => Ok(SortOrder::Asc),
            "desc" | "descending" => Ok(SortOrder::Desc),
            _ => Err(format!(
                "Invalid sort-order value: {}. Supported: asc, desc",
                s
            )),
        }
    }
}

pub async fn list_contracts(
    api_url: &str,
    network: Option<&str>,
    category: Option<&str>,
    limit: usize,
    offset: usize,
    sort_by: Option<&str>,
    sort_order: Option<&str>,
    output_format: OutputFormat,
) -> Result<()> {
    // Build query parameters - ensure limit is reasonable
    let limit = limit.min(100); // API probably has a max limit
    let mut params = vec![format!("limit={}", limit), format!("offset={}", offset)];

    if let Some(net) = network {
        params.push(format!("network={}", net));
    }

    if let Some(cat) = category {
        params.push(format!("category={}", cat));
    }

    // Add sorting parameters (API will handle them server-side)
    if let Some(sort) = sort_by {
        params.push(format!("sort_by={}", sort));
    }
    if let Some(order) = sort_order {
        params.push(format!("sort_order={}", order));
    }

    let query_string = params.join("&");
    let url = format!("{}/api/contracts?{}", api_url, query_string);

    log::debug!("Fetching contracts from: {}", url);

    let client = crate::net::client();
    let response = client
        .get(&url)
        .send_with_retry().await
        .context("Failed to fetch contracts from API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API request failed with status {}: {}", status, body);
    }

    let response_body: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse contracts response")?;

    // Extract contracts from response
    let contracts_array = response_body
        .get("items")
        .or_else(|| response_body.get("contracts"))
        .and_then(|v| v.as_array())
        .context("No contracts found in response")?;

    // Parse contracts
    let mut contracts = contracts_array
        .iter()
        .filter_map(|item| {
            let id = item
                .get("id")
                .or_else(|| item.get("contract_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let contract_id = item
                .get("contract_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let network = item
                .get("network")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let category = item
                .get("category")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let is_verified = item
                .get("is_verified")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let health_score = item
                .get("health_score")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let created_at = item
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let tags = item
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|tag| tag.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            Some(ContractListItem {
                id,
                name,
                contract_id,
                network,
                category,
                is_verified,
                health_score,
                created_at,
                tags,
            })
        })
        .collect::<Vec<_>>();

    // Apply client-side sorting if not handled by API
    let sort_by_field = sort_by
        .map(|s| s.parse::<SortBy>().map_err(|e| anyhow::anyhow!(e)))
        .transpose()?
        .unwrap_or(SortBy::CreatedAt);
    let sort_order_field = sort_order
        .map(|s| s.parse::<SortOrder>().map_err(|e| anyhow::anyhow!(e)))
        .transpose()?
        .unwrap_or(SortOrder::Desc);

    sort_contracts(&mut contracts, sort_by_field, sort_order_field);

    // Output results
    match output_format {
        OutputFormat::Table => print_table(&contracts),
        OutputFormat::Json => print_json(&contracts),
        OutputFormat::Csv => print_csv(&contracts),
    }

    Ok(())
}

fn sort_contracts(contracts: &mut [ContractListItem], sort_by: SortBy, sort_order: SortOrder) {
    contracts.sort_by(|a, b| {
        let cmp = match sort_by {
            SortBy::Name => a.name.cmp(&b.name),
            SortBy::CreatedAt => a.created_at.cmp(&b.created_at),
            SortBy::HealthScore => a.health_score.cmp(&b.health_score),
            SortBy::Network => a.network.cmp(&b.network),
        };

        if sort_order == SortOrder::Asc {
            cmp
        } else {
            cmp.reverse()
        }
    });
}

fn print_table(contracts: &[ContractListItem]) {
    if contracts.is_empty() {
        println!("{}", "No contracts found.".yellow());
        return;
    }

    // Header
    println!(
        "{:<36} {:<30} {:<15} {:<10} {:<15} {:<12}",
        "ID".bold(),
        "Name".bold(),
        "Network".bold(),
        "Verified".bold(),
        "Health".bold(),
        "Category".bold()
    );
    println!("{}", "─".repeat(120).cyan());

    // Rows
    for contract in contracts {
        let verified = if contract.is_verified {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };

        let health_color = match contract.health_score {
            85..=100 => contract.health_score.to_string().green(),
            60..=84 => contract.health_score.to_string().yellow(),
            _ => contract.health_score.to_string().red(),
        };

        let id = if contract.id.len() > 36 {
            format!("{}...", &contract.id[..33])
        } else {
            contract.id.clone()
        };

        let category = contract.category.as_deref().unwrap_or("—").to_string();

        println!(
            "{:<36} {:<30} {:<15} {:<10} {:<15} {:<12}",
            id,
            &contract.name[..contract.name.len().min(29)],
            contract.network,
            verified,
            health_color,
            &category[..category.len().min(11)]
        );
    }

    println!(
        "\n{}: {} contract(s) found",
        "Total".bold(),
        contracts.len().to_string().cyan()
    );
}

fn print_json(contracts: &[ContractListItem]) {
    let json_output = serde_json::to_string_pretty(&json!({
        "contracts": contracts,
        "count": contracts.len()
    }))
    .unwrap_or_else(|_| "{}".to_string());

    println!("{}", json_output);
}

fn print_csv(contracts: &[ContractListItem]) {
    // Header
    println!("id,name,contract_id,network,category,is_verified,health_score,created_at,tags");

    // Rows
    for contract in contracts {
        let tags = contract.tags.join("|");
        let category = contract.category.as_deref().unwrap_or("");

        println!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},{},\"{}\",\"{}\"",
            contract.id,
            contract.name,
            contract.contract_id,
            contract.network,
            category,
            contract.is_verified,
            contract.health_score,
            contract.created_at,
            tags
        );
    }
}

pub async fn run_details(
    api_url: &str,
    address: &str,
    network: &str,
    json_output: bool,
) -> Result<()> {
    let url = format!("{}/api/contracts/{}?network={}", api_url, address, network);
    log::debug!("Fetching contract details from: {}", url);

    let client = crate::net::client();
    let response = client
        .get(&url)
        .send_with_retry().await
        .context("Failed to fetch contract details from API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API request failed with status {}: {}", status, body);
    }

    let contract: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse contract response")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&contract)?);
        return Ok(());
    }

    let name = contract.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let desc = contract.get("description").and_then(|v| v.as_str()).unwrap_or("No description provided");
    let publisher = contract.get("publisher_id").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let category = contract.get("category").and_then(|v| v.as_str()).unwrap_or("—");
    let verified = contract.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false);
    
    let deployments = contract.get("deployment_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let interactions = contract.get("interaction_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let tags = contract.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(", ")
    }).unwrap_or_default();

    println!("\n{}", "Contract Details".bold().underline());
    println!("{:<15} {}", "Name:".bold(), name.cyan());
    println!("{:<15} {}", "Address:".bold(), address.cyan());
    println!("{:<15} {}", "Network:".bold(), network.cyan());
    println!("{:<15} {}", "Publisher:".bold(), publisher);
    println!("{:<15} {}", "Category:".bold(), category);
    println!("{:<15} {}", "Tags:".bold(), tags);
    
    let verified_str = if verified { "Yes".green() } else { "No".red() };
    println!("{:<15} {}", "Verified:".bold(), verified_str);
    
    println!("\n{}", "Description".bold().underline());
    println!("{}", desc);
    
    println!("\n{}", "Statistics".bold().underline());
    println!("{:<15} {}", "Deployments:".bold(), deployments.to_string().yellow());
    println!("{:<15} {}", "Interactions:".bold(), interactions.to_string().yellow());
    
    println!();
    
    Ok(())
}
