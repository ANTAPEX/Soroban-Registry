/// Core state inspection logic
use crate::client::StellarRpcClient;
use crate::decoder::decode_scval;
use crate::types::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// State inspector for fetching and analyzing contract state
pub struct StateInspector {
    client: StellarRpcClient,
}

impl StateInspector {
    /// Create a new inspector with the given RPC endpoint
    pub fn new(rpc_endpoint: &str) -> Self {
        Self {
            client: StellarRpcClient::new(rpc_endpoint),
        }
    }

    /// Create inspector connected to Testnet
    pub fn testnet() -> Self {
        Self {
            client: StellarRpcClient::testnet(),
        }
    }

    /// Create inspector connected to Mainnet
    pub fn mainnet() -> Self {
        Self {
            client: StellarRpcClient::mainnet(),
        }
    }

    /// Fetch all state keys and values for a contract at a ledger height
    pub async fn inspect(
        &self,
        contract_id: &str,
        ledger: Option<u32>,
        _key_filter: Option<&str>,
    ) -> Result<ContractState> {
        let key = format!("contract:{}", contract_id);

        let response = self.client.get_ledger_entries(vec![key], ledger).await?;

        let current_ledger = response.latest_ledger;
        let timestamp = format_timestamp(response.latest_ledger_close_time);

        let mut entries = Vec::new();

        if let Some(ledger_entries) = response.ledger_entries {
            for entry in ledger_entries {
                let decoded_key =
                    decode_scval(&entry.key).unwrap_or(DecodedValue::Bytes(entry.key.clone()));
                let decoded_value =
                    decode_scval(&entry.xdr).unwrap_or(DecodedValue::Bytes(entry.xdr.clone()));

                // FIX: clone key_raw before moving entry.key into the struct,
                // so determine_entry_type can still borrow it afterwards
                let key_raw = entry.key.clone();
                let entry_type = determine_entry_type(&key_raw);

                entries.push(StateEntry {
                    key: decoded_key,
                    key_raw,
                    value: decoded_value,
                    value_raw: entry.xdr,
                    entry_type,
                    ttl: None,
                });
            }
        }

        Ok(ContractState {
            contract_id: contract_id.to_string(),
            ledger: current_ledger,
            timestamp,
            entries,
        })
    }

    /// Get state history between two ledger heights
    pub async fn history(
        &self,
        contract_id: &str,
        start_ledger: u32,
        end_ledger: u32,
        _key_filter: Option<&str>,
    ) -> Result<Vec<StateSnapshot>> {
        let mut snapshots = Vec::new();

        let events = self
            .client
            .get_contract_events(contract_id, start_ledger, Some(end_ledger))
            .await?;

        for event in events {
            let timestamp = format_timestamp(0);
            snapshots.push(StateSnapshot {
                ledger: event.ledger,
                timestamp,
                entries: Vec::new(),
            });
        }

        Ok(snapshots)
    }
}

/// Determine entry type from key prefix
fn determine_entry_type(key: &str) -> EntryType {
    if key.contains("temp") {
        EntryType::Temporary
    } else if key.contains("instance") {
        EntryType::Instance
    } else {
        EntryType::Persistent
    }
}

/// Format Unix timestamp to readable string
fn format_timestamp(ts: u64) -> String {
    if ts == 0 {
        "unknown".to_string()
    } else {
        match DateTime::<Utc>::from_timestamp(ts as i64, 0) {
            Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            None => "unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_creation() {
        let _inspector = StateInspector::testnet();
    }

    #[test]
    fn test_entry_type_detection() {
        assert_eq!(determine_entry_type("temp:key"), EntryType::Temporary);
        assert_eq!(determine_entry_type("instance:key"), EntryType::Instance);
        assert_eq!(determine_entry_type("key"), EntryType::Persistent);
    }

    #[test]
    fn test_format_timestamp() {
        let formatted = format_timestamp(1705329781);
        assert!(formatted.contains("2024"));
    }
}
