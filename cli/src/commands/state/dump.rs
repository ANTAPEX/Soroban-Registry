//! `soroban-registry state dump`.

use crate::commands::state::store::load_local_state;
use crate::support::network::Network;
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub fn state_dump(contract_id: &str, network: Network, json_output: bool) -> Result<()> {
    let store = load_local_state(contract_id, network)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "count": store.values.len(),
                "values": store.values,
                "snapshots": store.snapshots.len(),
                "history_entries": store.history.len()
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "Contract State Dump".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Entries".bold(), store.values.len());
    println!("{}: {}", "Snapshots".bold(), store.snapshots.len());
    println!("{}: {}", "History Entries".bold(), store.history.len());
    println!();

    if store.values.is_empty() {
        println!("{}", "No state entries found.".yellow());
        println!();
        return Ok(());
    }

    for (key, value) in store.values {
        let pretty = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
        println!("{}: {}", key.bold(), pretty);
    }
    println!();
    Ok(())
}
