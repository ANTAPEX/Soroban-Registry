use crate::support::contract_view::contract_json;
use crate::support::filters::normalize_filter_values;
use crate::support::net::RequestBuilderExt;
use crate::support::network::normalize_network_filters;
use crate::support::network::shared_network;
use crate::support::network::shared_networks;
use crate::support::network::Network;
use anyhow::{Context, Result};
use colored::Colorize;
use registry_client::ContractSearchParams;
use shared::models::Contract;
use std::time::Instant;

pub async fn run(
    query: &str,
    verified_only: bool,
    networks: Option<&String>,
    category: Option<&String>,
    sort: Option<&String>,
    limit: usize,
    offset: usize,
    output_json: bool,
    api_url: &str,
) -> anyhow::Result<()> {
    let start = Instant::now();

    let url = format!("{}/api/contracts", api_url);
    let client = crate::support::net::client();
    let mut all_contracts: Vec<Contract> = client.get(&url).send_with_retry().await?.json().await?;

    let q = query.to_lowercase();
    all_contracts.retain(|c| {
        c.name.to_lowercase().contains(&q)
            || c.description
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(&q)
    });

    if let Some(nets) = networks {
        let network_list: Vec<&str> = nets.split(',').map(|s| s.trim()).collect();
        all_contracts.retain(|c| {
            let net_str = format!("{:?}", c.network).to_lowercase();
            network_list.iter().any(|n| n.to_lowercase() == net_str)
        });
    }

    if let Some(category_filter) = category {
        let category_list: Vec<&str> = category_filter.split(',').map(|s| s.trim()).collect();
        all_contracts.retain(|c| {
            let cat = c.category.as_deref().unwrap_or("");
            category_list.iter().any(|&c| cat.eq_ignore_ascii_case(c))
        });
    }

    if verified_only {
        all_contracts.retain(|c| c.is_verified);
    }

    let sort_mode = sort.map(|s| s.as_str()).unwrap_or("relevance");
    match sort_mode {
        "updated" => all_contracts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        "created" => all_contracts.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        "name" => all_contracts.sort_by(|a, b| a.name.cmp(&b.name)),
        _ => {
            all_contracts.sort_by(|a, b| {
                b.relevance_score
                    .unwrap_or(0.0)
                    .partial_cmp(&a.relevance_score.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    if offset < all_contracts.len() {
        all_contracts = all_contracts.split_off(offset);
    } else {
        all_contracts.clear();
    }
    all_contracts.truncate(limit);
    let elapsed = start.elapsed();

    if output_json {
        let contracts: Vec<serde_json::Value> = all_contracts
            .iter()
            .map(|c| {
                let tag_names: Vec<String> = c.tags.iter().map(|t| t.name.clone()).collect();
                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "contract_id": c.contract_id,
                    "network": c.network,
                    "category": c.category.as_deref().unwrap_or(""),
                    "is_verified": c.is_verified,
                    "health_score": c.health_score,
                    "created_at": c.created_at.to_rfc3339(),
                    "tags": tag_names,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contracts": contracts,
                "count": contracts.len()
            }))?
        );
        return Ok(());
    }

    if all_contracts.is_empty() {
        println!("{}", "No contracts found matching your query.".yellow());
        return Ok(());
    }

    println!(
        "{} {} result(s) in {:.0}ms\n",
        "Found".green().bold(),
        all_contracts.len().to_string().green().bold(),
        elapsed.as_millis()
    );

    let mut filters: Vec<String> = Vec::new();
    if let Some(nets) = networks {
        filters.push(format!("networks: {}", nets));
    }
    if let Some(cat) = category {
        filters.push(format!("category: {}", cat));
    }
    if verified_only {
        filters.push("verified".to_string());
    }
    if !filters.is_empty() {
        println!("  {}\n", filters.join(" | ").bright_blue());
    }

    for contract in &all_contracts {
        let highlighted_name = highlight_match(&contract.name, query);
        let desc = contract.description.as_deref().unwrap_or("No description");
        let highlighted_desc = highlight_match(desc, query);

        let verified_badge = if contract.is_verified {
            " [OK] verified".green().to_string()
        } else {
            String::new()
        };

        println!(" {}{}", highlighted_name.bold(), verified_badge);
        println!("   {}", highlighted_desc);
        println!(
            "   {} {:?} | {} {}",
            "Network:".dimmed(),
            contract.network,
            "Category:".dimmed(),
            contract.category.as_deref().unwrap_or("unknown")
        );
        println!(
            "   {} {}",
            "Updated:".dimmed(),
            contract.updated_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!();
    }

    Ok(())
}

fn highlight_match(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut result = String::new();
    let mut last = 0;
    while let Some(pos) = lower_text[last..].find(&lower_query) {
        let abs = last + pos;
        result.push_str(&text[last..abs]);
        result.push_str(&text[abs..abs + query.len()].yellow().bold().to_string());
        last = abs + query.len();
    }
    result.push_str(&text[last..]);
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_multiple_networks_comma_separated() {
        let input = "testnet,mainnet,futurenet";
        let networks: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
        assert_eq!(networks.len(), 3);
        assert!(networks.contains(&"testnet"));
        assert!(networks.contains(&"mainnet"));
        assert!(networks.contains(&"futurenet"));
    }

    #[test]
    fn parse_single_network() {
        let input = "testnet";
        let networks: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0], "testnet");
    }

    #[test]
    fn parse_networks_with_spaces() {
        let input = "testnet, mainnet , futurenet";
        let networks: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
        assert_eq!(networks.len(), 3);
        assert_eq!(networks[0], "testnet");
        assert_eq!(networks[1], "mainnet");
        assert_eq!(networks[2], "futurenet");
    }
}

/// Map the CLI's `--sort` values onto the API's `sort_by`.
///
/// `name` has no server-side equivalent, so it is applied locally to the page
/// that comes back rather than being sent and silently ignored.
fn parse_sort_by(value: &str) -> Option<shared::models::SortBy> {
    match value.trim().to_lowercase().as_str() {
        "created" | "created_at" => Some(shared::models::SortBy::CreatedAt),
        "updated" | "updated_at" => Some(shared::models::SortBy::UpdatedAt),
        "relevance" => Some(shared::models::SortBy::Relevance),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn search(
    api_url: &str,
    query: &str,
    network: Network,
    verified_only: bool,
    networks: Vec<String>,
    category: Option<&str>,
    sort: Option<&str>,
    limit: usize,
    offset: usize,
    json: bool,
    explain: bool,
) -> Result<()> {
    let t0 = std::time::Instant::now();

    // Normalize before sending so comma-separated and repeated flags produce an
    // identical request, and unknown networks fail here rather than being
    // silently dropped server-side.
    let networks = normalize_network_filters(&networks)?;
    let categories =
        normalize_filter_values(&category.map(str::to_string).into_iter().collect::<Vec<_>>());

    let mut params = ContractSearchParams {
        query: Some(query.to_string()),
        limit: Some(limit as i64),
        offset: Some(offset as i64),
        ..Default::default()
    };
    if !networks.is_empty() {
        params.networks = Some(shared_networks(&networks)?);
    } else {
        params.network = shared_network(&network.to_string());
    }
    if verified_only {
        params.verified_only = Some(true);
    }
    if !categories.is_empty() {
        params.categories = Some(categories.clone());
    }
    if let Some(sort) = sort {
        params.sort_by = parse_sort_by(sort);
    }

    let mut page = crate::support::registry::client(api_url)
        .await?
        .list_contracts(&params)
        .await
        .context("Failed to search contracts")?;

    if sort.is_some_and(|value| value.trim().eq_ignore_ascii_case("name")) {
        page.items.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let items = &page.items;

    // Issue #1187: Search ranking transparency and explainability mode
    if explain {
        let results: Vec<crate::support::ranking::ExplainSearchResult> = items
            .iter()
            .map(|contract| {
                let (score, factors) =
                    crate::support::ranking::calculate_ranking_factors(query, contract);
                crate::support::ranking::ExplainSearchResult {
                    contract_id: contract.contract_id.clone(),
                    score,
                    factors,
                }
            })
            .collect();

        if json {
            let response = crate::support::ranking::ExplainSearchResponse { results };
            println!("{}", serde_json::to_string_pretty(&response)?);
            return Ok(());
        }

        println!(
            "\n{}",
            "Search Ranking Transparency & Explainability (--explain):"
                .bold()
                .cyan()
        );
        println!("{}", "=".repeat(80).cyan());
        println!(
            "Formula: Score = Text Relevance + Verification Bonus + Popularity + Recency + Deprecation Penalty\n"
        );

        if items.is_empty() {
            println!("{}", "No contracts found to explain.".yellow());
            return Ok(());
        }

        for (idx, contract) in items.iter().enumerate() {
            let (score, factors) =
                crate::support::ranking::calculate_ranking_factors(query, contract);
            println!(
                "{}. {} ({})",
                (idx + 1).to_string().bold(),
                contract.name.bold().green(),
                contract.contract_id.dimmed()
            );
            println!(
                "   Total Score: {}",
                format!("{:.2}", score).bold().yellow()
            );
            println!(
                "   ├── Text Relevance:      {:+5.2}  (query term match density)",
                factors.text_relevance
            );
            println!(
                "   ├── Verification Bonus:  {:+5.2}  ({})",
                factors.verification_bonus,
                if contract.is_verified {
                    "verified contract"
                } else {
                    "unverified"
                }
            );
            println!(
                "   ├── Popularity Signal:   {:+5.2}  (usage count: {})",
                factors.popularity, contract.usage_count
            );
            let days_old = (chrono::Utc::now() - contract.updated_at).num_days().max(0);
            println!(
                "   ├── Recency Decay:       {:+5.2}  (updated {}d ago)",
                factors.recency, days_old
            );
            println!(
                "   └── Deprecation Penalty: {:+5.2}  ({})",
                factors.deprecation_penalty,
                if factors.deprecation_penalty < 0.0 {
                    "deprecated"
                } else {
                    "active"
                }
            );
            println!();
        }

        println!(
            "{} {} result(s) with ranking factor breakdown in {:.0}ms\n",
            "Explained".green().bold(),
            items.len(),
            t0.elapsed().as_millis()
        );
        return Ok(());
    }

    if json {
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

    println!("\n{}", "Search Results:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let mut active_filters: Vec<String> = Vec::new();
    if !networks.is_empty() {
        active_filters.push(format!("network: {}", networks.join(", ")));
    }
    if !categories.is_empty() {
        active_filters.push(format!("category: {}", categories.join(", ")));
    }
    if verified_only {
        active_filters.push("verified only".to_string());
    }
    if !active_filters.is_empty() {
        println!(
            "  {} {}\n",
            "Active filters:".bold(),
            active_filters.join(" | ").bright_blue()
        );
    }

    if items.is_empty() {
        println!("{}", "No contracts found matching your filters.".yellow());
        println!("\n{}", "Suggestions:".bold());
        println!("  • Try a broader search query");
        if !categories.is_empty() {
            println!("  • Remove the --category filter to see all contract types");
        }
        if !networks.is_empty() {
            println!("  • Try adding more networks: --network mainnet,testnet,futurenet");
        }
        if verified_only {
            println!("  • Remove --verified-only to include unverified contracts");
        }
        println!("  • Use 'list' command to browse all contracts\n");
        return Ok(());
    }

    // Compute visible column widths from raw data (before applying ANSI codes).
    let name_w = items
        .iter()
        .map(|contract| contract.name.chars().count())
        .max()
        .unwrap_or(0)
        .max("Name".len());
    let net_w = items
        .iter()
        .map(|contract| contract.network.to_string().chars().count())
        .max()
        .unwrap_or(0)
        .max("Network".len());
    let cat_w = items
        .iter()
        .filter_map(|contract| {
            contract
                .category
                .as_deref()
                .filter(|value| !value.is_empty())
        })
        .map(|value| value.chars().count())
        .max()
        .unwrap_or(0)
        .max("Category".len());
    // "Unverified" is the longest possible verified cell value.
    let ver_w = "Unverified".chars().count();
    let link_prefix = format!("{}/contracts/", api_url);
    let link_w = items
        .iter()
        .map(|contract| link_prefix.len() + contract.contract_id.len())
        .max()
        .unwrap_or(0)
        .max("Links".len())
        .min(60);

    let mut rows: Vec<Vec<String>> = Vec::new();
    for contract in items {
        let link = format!("{}/contracts/{}", api_url, contract.contract_id);
        let verified = if contract.is_verified {
            "Verified".green().to_string()
        } else {
            "Unverified".yellow().to_string()
        };

        rows.push(vec![
            // Highlight where the query matched, as the pre-client renderer did.
            crate::support::table_format::highlight_match(&contract.name, query),
            contract.network.to_string(),
            crate::support::table_format::highlight_match(
                contract.category.as_deref().unwrap_or_default(),
                query,
            ),
            verified,
            link,
        ]);
    }

    println!(
        "{:<name_w$}  {:<net_w$}  {:<cat_w$}  {:<ver_w$}  {:<link_w$}",
        "Name".bold(),
        "Network".bold(),
        "Category".bold(),
        "Verified".bold(),
        "Links".bold(),
    );

    for (row, contract) in rows.iter().zip(items) {
        // Highlighted cells carry ANSI codes, so pad them by visible width
        // rather than by byte length.
        let name_pad = " ".repeat(name_w.saturating_sub(contract.name.chars().count()));
        let category = contract.category.as_deref().unwrap_or_default();
        let cat_pad = " ".repeat(cat_w.saturating_sub(category.chars().count()));
        let verified_visible = if contract.is_verified {
            "Verified".chars().count()
        } else {
            "Unverified".chars().count()
        };
        let verified_pad = " ".repeat(ver_w.saturating_sub(verified_visible));

        println!(
            "{}{}  {:<net_w$}  {}{}  {}{}  {:<link_w$}",
            row[0], name_pad, row[1], row[2], cat_pad, row[3], verified_pad, row[4],
        );
    }

    println!(
        "\n{} {} result(s) of {} in {:.0}ms",
        "Found".green().bold(),
        items.len(),
        page.total,
        t0.elapsed().as_millis()
    );
    println!();

    Ok(())
}
