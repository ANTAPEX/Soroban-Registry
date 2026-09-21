//! `soroban-registry state history`.

use crate::commands::state::store::load_local_state;
use crate::commands::state::store::LocalStateHistoryEntry;
use crate::support::network::Network;
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub fn state_history(
    contract_id: &str,
    network: Network,
    key_filter: Option<&str>,
    limit: usize,
    json_output: bool,
) -> Result<()> {
    let store = load_local_state(contract_id, network)?;
    let entries: Vec<&LocalStateHistoryEntry> = store
        .history
        .iter()
        .rev()
        .filter(|entry| {
            if let Some(filter) = key_filter {
                return entry.key.as_deref() == Some(filter);
            }
            true
        })
        .take(limit)
        .collect();

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "items": entries
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State History".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    if entries.is_empty() {
        println!("{}", "No history entries found.".yellow());
        println!();
        return Ok(());
    }

    for (index, entry) in entries.iter().enumerate() {
        let key = entry.key.as_deref().unwrap_or("-");
        println!(
            "  {}. [{}] {} key={} note={}",
            index + 1,
            entry.timestamp.bright_black(),
            entry.action.bold(),
            key.bright_magenta(),
            entry.note.clone().unwrap_or_else(|| "-".to_string())
        );
    }
    println!();
    Ok(())
}
