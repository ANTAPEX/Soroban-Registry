//! `soroban-registry state get`.

use crate::commands::state::remote::try_remote_state_get;
use crate::commands::state::store::load_local_state;
use crate::support::network::Network;
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub async fn state_get(
    api_url: &str,
    contract_id: &str,
    key: &str,
    network: Network,
    json_output: bool,
) -> Result<()> {
    let remote_value = try_remote_state_get(api_url, contract_id, key).await?;
    let (value, source) = if let Some(value) = remote_value {
        (value, "remote")
    } else {
        let store = load_local_state(contract_id, network)?;
        let value = store
            .values
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("State key not found: {}", key))?;
        (value, "local")
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "key": key,
                "value": value,
                "source": source
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "Contract State Value".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Key".bold(), key.bright_magenta());
    println!("{}: {}", "Source".bold(), source);
    println!(
        "{}:\n{}",
        "Value".bold(),
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
    );
    println!();
    Ok(())
}
