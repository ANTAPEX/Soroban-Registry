//! `soroban-registry contract stats` — statistics for one contract.

use crate::support::net::RequestBuilderExt;
use crate::support::stats_format::format_stats_csv;
use crate::support::stats_format::format_stats_table;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

#[allow(clippy::too_many_arguments)]
pub async fn contract_stats(
    api_url: &str,
    network: Option<&str>,
    category: Option<&str>,
    top_n: usize,
    format: &str,
    output: Option<&str>,
    compare: Option<&str>,
) -> Result<()> {
    let client = crate::support::net::client();
    let mut url = reqwest::Url::parse(&format!("{}/api/stats", api_url.trim_end_matches('/')))
        .context("Invalid registry API URL")?;
    {
        let mut query = url.query_pairs_mut();
        if let Some(network) = network {
            query.append_pair("network", network);
        }
        if let Some(category) = category {
            query.append_pair("category", category);
        }
        if let Some(compare) = compare {
            query.append_pair("compare", compare);
        }
    }

    let response = client
        .get(url)
        .send_with_retry()
        .await
        .context("Failed to fetch contract statistics")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch contract stats: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let mut stats: serde_json::Value = response.json().await?;
    if let Some(top) = stats
        .get_mut("top_contracts")
        .and_then(serde_json::Value::as_array_mut)
    {
        top.truncate(top_n);
    }

    let output_str = match format {
        "json" => serde_json::to_string_pretty(&stats)?,
        "csv" => format_stats_csv(&stats)?,
        "table" => format_stats_table(&stats),
        _ => anyhow::bail!("Invalid format: {}. Use table, json, or csv", format),
    };

    if let Some(path) = output {
        fs::write(path, &output_str)?;
        println!("{} Contract stats written to {}", "OK".green(), path);
    } else {
        println!("{}", output_str);
    }

    Ok(())
}
