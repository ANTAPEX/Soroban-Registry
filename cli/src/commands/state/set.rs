//! `soroban-registry state set`.

use crate::commands::state::remote::try_remote_state_set;
use crate::commands::state::store::load_local_state;
use crate::commands::state::store::parse_state_value;
use crate::commands::state::store::require_mutable_network;
use crate::commands::state::store::save_local_state;
use crate::commands::state::store::LocalStateHistoryEntry;
use crate::support::network::Network;
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub async fn state_set(
    api_url: &str,
    contract_id: &str,
    key: &str,
    raw_value: &str,
    network: Network,
    json_output: bool,
) -> Result<()> {
    require_mutable_network(network)?;
    let new_value = parse_state_value(raw_value);

    let remote_applied = try_remote_state_set(api_url, contract_id, key, &new_value)
        .await
        .unwrap_or(false);

    let mut store = load_local_state(contract_id, network)?;
    let previous = store.values.insert(key.to_string(), new_value.clone());
    store.history.push(LocalStateHistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "set".to_string(),
        key: Some(key.to_string()),
        previous,
        value: Some(new_value.clone()),
        note: if remote_applied {
            Some("remote + local".to_string())
        } else {
            Some("local".to_string())
        },
    });
    save_local_state(&store, network)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "key": key,
                "value": new_value,
                "remote_applied": remote_applied,
                "status": "updated"
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State Updated".bold().green());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Key".bold(), key.bright_magenta());
    println!("{}: {}", "Remote Applied".bold(), remote_applied);
    println!(
        "{}:\n{}",
        "New Value".bold(),
        serde_json::to_string_pretty(&new_value).unwrap_or_else(|_| new_value.to_string())
    );
    println!();
    Ok(())
}
