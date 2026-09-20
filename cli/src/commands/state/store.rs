//! The on-disk store behind `soroban-registry state`, and the helpers that
//! read and write it.

use crate::support::network::Network;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LocalStateHistoryEntry {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) action: String,
    pub(crate) key: Option<String>,
    pub(crate) previous: Option<serde_json::Value>,
    pub(crate) value: Option<serde_json::Value>,
    pub(crate) note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LocalStateSnapshot {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) created_at: String,
    pub(crate) entry_count: usize,
    pub(crate) state: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LocalContractStateStore {
    pub(crate) contract_id: String,
    pub(crate) network: String,
    pub(crate) values: BTreeMap<String, serde_json::Value>,
    pub(crate) snapshots: Vec<LocalStateSnapshot>,
    pub(crate) history: Vec<LocalStateHistoryEntry>,
}

impl LocalContractStateStore {
    fn new(contract_id: &str, network: Network) -> Self {
        Self {
            contract_id: contract_id.to_string(),
            network: network.to_string(),
            values: BTreeMap::new(),
            snapshots: Vec::new(),
            history: Vec::new(),
        }
    }
}

fn state_root_dir() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("SOROBAN_REGISTRY_STATE_DIR") {
        let path = PathBuf::from(custom);
        fs::create_dir_all(&path).with_context(|| {
            format!(
                "Failed to create custom state directory from SOROBAN_REGISTRY_STATE_DIR: {}",
                path.display()
            )
        })?;
        return Ok(path);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".soroban-registry").join("state"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".soroban-registry").join("state"));
    }
    if let Some(temp) = std::env::var_os("TMP").map(PathBuf::from) {
        candidates.push(temp.join("soroban-registry-state"));
    }

    for candidate in candidates {
        if fs::create_dir_all(&candidate).is_ok() {
            return Ok(candidate);
        }
    }

    anyhow::bail!("Unable to create a writable state directory")
}

fn sanitize_for_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn state_file_path(contract_id: &str, network: Network) -> Result<PathBuf> {
    let root = state_root_dir()?;
    let network_dir = root.join(network.to_string());
    fs::create_dir_all(&network_dir).with_context(|| {
        format!(
            "Failed to create state directory: {}",
            network_dir.display()
        )
    })?;
    let file_name = format!("{}.json", sanitize_for_filename(contract_id));
    Ok(network_dir.join(file_name))
}

pub(crate) fn load_local_state(
    contract_id: &str,
    network: Network,
) -> Result<LocalContractStateStore> {
    let path = state_file_path(contract_id, network)?;
    if !path.exists() {
        return Ok(LocalContractStateStore::new(contract_id, network));
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read state file: {}", path.display()))?;
    let mut store: LocalContractStateStore = serde_json::from_str(&content)
        .with_context(|| format!("Invalid state file format: {}", path.display()))?;

    if store.contract_id.is_empty() {
        store.contract_id = contract_id.to_string();
    }
    if store.network.is_empty() {
        store.network = network.to_string();
    }

    Ok(store)
}

pub(crate) fn save_local_state(store: &LocalContractStateStore, network: Network) -> Result<()> {
    let path = state_file_path(&store.contract_id, network)?;
    let data = serde_json::to_string_pretty(store).context("Failed to serialize state")?;
    fs::write(&path, data)
        .with_context(|| format!("Failed to write state file: {}", path.display()))
}

pub(crate) fn parse_state_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

pub(crate) fn require_mutable_network(network: Network) -> Result<()> {
    if matches!(network, Network::Mainnet) {
        anyhow::bail!("State mutation is disabled on mainnet. Use testnet or futurenet.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_state_value_uses_json_when_possible() {
        let parsed = parse_state_value("{\"x\":1}");
        assert_eq!(parsed["x"], 1);
    }

    #[test]
    fn parse_state_value_falls_back_to_string() {
        let parsed = parse_state_value("not-json");
        assert_eq!(parsed, serde_json::Value::String("not-json".to_string()));
    }

    #[test]
    fn mainnet_mutation_is_blocked() {
        let result = require_mutable_network(Network::Mainnet);
        assert!(result.is_err());
    }

    #[test]
    fn non_mainnet_mutation_is_allowed() {
        assert!(require_mutable_network(Network::Testnet).is_ok());
        assert!(require_mutable_network(Network::Futurenet).is_ok());
    }
}
