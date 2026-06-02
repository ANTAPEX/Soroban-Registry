/// Stellar RPC client for state inspection
use crate::types::{ContractEvent, LedgerEntriesResponse};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::time::Duration;

pub const TESTNET_RPC: &str = "https://soroban-testnet.stellar.org";
pub const MAINNET_RPC: &str = "https://mainnet.stellar.validationcloud.io/v1/soroban/rpc";

/// Client for communicating with Stellar RPC
#[derive(Debug, Clone)]
pub struct StellarRpcClient {
    pub endpoint: String,
    client: reqwest::Client,
}

impl StellarRpcClient {
    /// Create a new RPC client with the given endpoint
    pub fn new(endpoint: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.to_string(),
            client,
        }
    }

    /// Create a client connected to Testnet
    pub fn testnet() -> Self {
        Self::new(TESTNET_RPC)
    }

    /// Create a client connected to Mainnet
    pub fn mainnet() -> Self {
        Self::new(MAINNET_RPC)
    }

    /// Get ledger entries (contract state) at a specific ledger
    pub async fn get_ledger_entries(
        &self,
        keys: Vec<String>,
        ledger: Option<u32>,
    ) -> Result<LedgerEntriesResponse> {
        let mut params = json!({ "keys": keys });

        if let Some(l) = ledger {
            params["ledger"] = json!(l);
        }

        // FIX: jsonrpc_call returns serde_json::Value — deserialize explicitly
        // into LedgerEntriesResponse instead of relying on the generic return type
        let value = self.jsonrpc_call("getLedgerEntries", params).await?;
        serde_json::from_value::<LedgerEntriesResponse>(value)
            .map_err(|e| anyhow!("Failed to parse LedgerEntriesResponse: {}", e))
    }

    /// Get the latest ledger height
    pub async fn get_latest_ledger(&self) -> Result<u32> {
        let response = self.jsonrpc_call("getLatestLedger", json!({})).await?;
        response
            .get("sequence")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .ok_or_else(|| anyhow!("Invalid getLatestLedger response"))
    }

    /// Get transaction history for a contract
    pub async fn get_contract_events(
        &self,
        contract_id: &str,
        start_ledger: u32,
        end_ledger: Option<u32>,
    ) -> Result<Vec<ContractEvent>> {
        let mut params = json!({
            "filters": [{
                "type": "contract",
                "contractIds": [contract_id]
            }],
            "startLedger": start_ledger,
            "limit": 100
        });

        if let Some(e) = end_ledger {
            params["endLedger"] = json!(e);
        }

        let response = self.jsonrpc_call("getEvents", params).await?;

        let events = response
            .get("events")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| serde_json::from_value::<ContractEvent>(e.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(events)
    }

    /// Simulate a contract function call
    pub async fn simulate_transaction(&self, tx_envelope: &str) -> Result<serde_json::Value> {
        self.jsonrpc_call("simulateTransaction", json!({ "transaction": tx_envelope }))
            .await
    }

    /// Internal JSON-RPC 2.0 call with retry logic
    async fn jsonrpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": if params.is_null() { json!({}) } else { params }
        });

        let mut retries = 0u32;
        let max_retries = 3u32;

        loop {
            match self
                .client
                .post(&self.endpoint)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
            {
                Ok(response) => {
                    let result = response.json::<serde_json::Value>().await?;

                    if let Some(error) = result.get("error") {
                        return Err(anyhow!("RPC Error: {}", error));
                    }

                    return result
                        .get("result")
                        .cloned()
                        .ok_or_else(|| anyhow!("No result in RPC response"));
                }
                Err(_e) if retries < max_retries => {
                    retries += 1;
                    let backoff = Duration::from_millis(100 * 2_u64.pow(retries - 1));
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => {
                    return Err(anyhow!(
                        "RPC call failed after {} retries: {}",
                        max_retries,
                        e
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = StellarRpcClient::testnet();
        assert!(client.endpoint.contains("testnet"));
    }

    #[test]
    fn test_mainnet_endpoint() {
        let client = StellarRpcClient::mainnet();
        assert!(client.endpoint.contains("mainnet"));
    }
}
