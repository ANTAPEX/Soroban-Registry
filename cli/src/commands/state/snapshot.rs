//! `soroban-registry state snapshot` — create and list snapshots.

use crate::commands::state::store::load_local_state;
use crate::commands::state::store::save_local_state;
use crate::commands::state::store::LocalStateHistoryEntry;
use crate::commands::state::store::LocalStateSnapshot;
use crate::support::network::Network;
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub fn state_snapshot_create(
    contract_id: &str,
    network: Network,
    label: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let mut store = load_local_state(contract_id, network)?;
    let snapshot = LocalStateSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        label: label.map(|s| s.to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        entry_count: store.values.len(),
        state: store.values.clone(),
    };

    store.history.push(LocalStateHistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "snapshot".to_string(),
        key: None,
        previous: None,
        value: None,
        note: snapshot.label.clone(),
    });
    store.snapshots.push(snapshot.clone());
    save_local_state(&store, network)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "snapshot_id": snapshot.id,
                "label": snapshot.label,
                "created_at": snapshot.created_at,
                "entry_count": snapshot.entry_count
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State Snapshot Created".bold().green());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Snapshot ID".bold(), snapshot.id.bright_magenta());
    println!(
        "{}: {}",
        "Label".bold(),
        snapshot.label.unwrap_or_else(|| "-".to_string())
    );
    println!("{}: {}", "Entries".bold(), snapshot.entry_count);
    println!();
    Ok(())
}

pub fn state_snapshot_list(
    contract_id: &str,
    network: Network,
    limit: usize,
    json_output: bool,
) -> Result<()> {
    let store = load_local_state(contract_id, network)?;
    let snapshots: Vec<&LocalStateSnapshot> = store.snapshots.iter().rev().take(limit).collect();

    if json_output {
        let payload: Vec<serde_json::Value> = snapshots
            .iter()
            .map(|snapshot| {
                json!({
                    "id": snapshot.id,
                    "label": snapshot.label,
                    "created_at": snapshot.created_at,
                    "entry_count": snapshot.entry_count
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "items": payload
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State Snapshots".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    if snapshots.is_empty() {
        println!("{}", "No snapshots found.".yellow());
        println!();
        return Ok(());
    }

    for (index, snapshot) in snapshots.iter().enumerate() {
        println!(
            "  {}. {} [{}] entries={} label={}",
            index + 1,
            snapshot.id.bright_magenta(),
            snapshot.created_at.bright_black(),
            snapshot.entry_count,
            snapshot.label.clone().unwrap_or_else(|| "-".to_string())
        );
    }
    println!();
    Ok(())
}
