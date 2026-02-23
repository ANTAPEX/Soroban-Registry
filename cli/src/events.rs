#![allow(dead_code)]

use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    pub id: String,
    pub contract_id: String,
    pub topic: String,
    pub data: Option<serde_json::Value>,
    pub ledger_sequence: i64,
    pub transaction_hash: Option<String>,
    pub timestamp: String,
    pub network: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    pub contract_id: String,
    pub total_events: i64,
    pub unique_topics: i64,
    pub first_event: Option<String>,
    pub last_event: Option<String>,
    pub events_by_topic: serde_json::Value,
}

pub async fn query_events(
    api_url: &str,
    contract_id: &str,
    topic: Option<&str>,
    filter: Option<&str>,
    limit: i64,
    offset: i64,
    export_path: Option<&str>,
    stats_only: bool,
) -> Result<()> {
    println!("\n{}", "Contract Events".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let client = reqwest::Client::new();

    if stats_only {
        let url = format!("{}/api/contracts/{}/events/stats", api_url, contract_id);

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch event stats: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await?;
            anyhow::bail!("API error: {}", error);
        }

        let stats: EventStats = response.json().await?;

        println!("\n{}", "Event Statistics".bold());
        println!(
            "  {}: {}",
            "Contract ID".bold(),
            stats.contract_id.bright_black()
        );
        println!("  {}: {}", "Total Events".bold(), stats.total_events);
        println!("  {}: {}", "Unique Topics".bold(), stats.unique_topics);

        if let Some(first) = &stats.first_event {
            println!("  {}: {}", "First Event".bold(), first);
        }
        if let Some(last) = &stats.last_event {
            println!("  {}: {}", "Last Event".bold(), last);
        }

        if let Some(obj) = stats.events_by_topic.as_object() {
            if !obj.is_empty() {
                println!("\n{}", "Events by Topic".bold());
                for (topic, count) in obj.iter() {
                    let count_val = count.as_i64().unwrap_or(0);
                    println!(
                        "  {} {}",
                        topic.bright_magenta(),
                        format!("({})", count_val).bright_black()
                    );
                }
            }
        }

        println!("\n{}", "=".repeat(80).cyan());
        return Ok(());
    }

    let mut url = format!(
        "{}/api/contracts/{}/events?limit={}&offset={}",
        api_url, contract_id, limit, offset
    );

    if let Some(t) = topic {
        url.push_str(&format!("&topic={}", t));
    }

    if let Some(f) = filter {
        url.push_str(&format!("&data_pattern={}", f));
    }

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch events: {}", e))?;

    if !response.status().is_success() {
        let error = response.text().await?;
        anyhow::bail!("API error: {}", error);
    }

    let events: Vec<ContractEvent> = response.json().await?;

    if let Some(path) = export_path {
        let mut csv = String::from(
            "id,contract_id,topic,data,ledger_sequence,transaction_hash,timestamp,network\n",
        );

        for event in &events {
            let data_str = event
                .data
                .as_ref()
                .map(|d| serde_json::to_string(d).unwrap_or_default())
                .unwrap_or_default();

            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                event.id,
                event.contract_id,
                event.topic,
                data_str.replace("\"", "\"\""),
                event.ledger_sequence,
                event.transaction_hash.as_deref().unwrap_or(""),
                event.timestamp,
                event.network
            ));
        }

        std::fs::write(path, csv)?;
        println!(
            "{} Exported {} events to {}",
            "✓".green(),
            events.len(),
            path
        );
        return Ok(());
    }

    println!("\n{}", format!("Found {} event(s)", events.len()).bold());

    for event in &events {
        println!("\n{} {}", "●".cyan(), event.topic.bold().yellow());
        println!(
            "  {}: {}",
            "Ledger".bold(),
            event.ledger_sequence.to_string().bright_black()
        );
        println!(
            "  {}: {}",
            "Timestamp".bold(),
            event.timestamp.bright_black()
        );

        if let Some(tx_hash) = &event.transaction_hash {
            println!("  {}: {}...", "Tx".bold(), &tx_hash[..16].bright_black());
        }

        if let Some(data) = &event.data {
            let data_str = serde_json::to_string_pretty(data).unwrap_or_default();
            let lines: Vec<&str> = data_str.lines().take(5).collect();
            println!("  {}:", "Data".bold());
            for line in lines {
                println!("    {}", line.bright_black());
            }
            if data_str.lines().count() > 5 {
                println!("    {}", "...".bright_black());
            }
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}
