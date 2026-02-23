/// Dry run engine for simulating contract calls and collecting gas metrics
use crate::client::StellarRpcClient;
use crate::types::*;
use anyhow::Result;
use std::time::Instant;

const SAFETY_MARGIN_BPS: u64 = 1500;

pub struct DryRunner {
    client: StellarRpcClient,
}

impl DryRunner {
    pub fn new(rpc_endpoint: &str) -> Self {
        Self {
            client: StellarRpcClient::new(rpc_endpoint),
        }
    }

    pub fn testnet() -> Self {
        Self {
            client: StellarRpcClient::testnet(),
        }
    }

    pub fn mainnet() -> Self {
        Self {
            client: StellarRpcClient::mainnet(),
        }
    }

    /// Simulate a contract function call and return resulting state delta
    pub async fn simulate(
        &self,
        contract_id: &str,
        function: &str,
        args: Vec<String>,
        _ledger: Option<u32>,
    ) -> Result<DryRunResult> {
        let sim_result = self.simulate_with_metrics(contract_id, function, args).await?;

        Ok(DryRunResult {
            success: sim_result.success,
            return_value: None,
            state_changes: vec![],
            events: sim_result.events,
            cpu_instructions: sim_result.gas_metrics.cpu_instructions,
            memory_bytes: sim_result.gas_metrics.memory_bytes,
            error: sim_result.error,
        })
    }

    /// Full simulation that returns detailed gas metrics for estimation
    pub async fn simulate_with_metrics(
        &self,
        contract_id: &str,
        function: &str,
        args: Vec<String>,
    ) -> Result<SimulationResult> {
        let tx_envelope = self.build_invoke_tx(contract_id, function, &args)?;
        let start = Instant::now();

        let response = self.client.simulate_transaction(&tx_envelope).await?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        Self::parse_simulation_response(&response, execution_time_ms)
    }

    /// Build a minimal Soroban invokeHostFunction transaction envelope (XDR, base64-encoded).
    ///
    /// The Stellar RPC `simulateTransaction` endpoint only needs a *syntactically*
    /// valid transaction XDR; it does not verify signatures. We build the smallest
    /// valid envelope that carries the contract invocation and use a zero-filled
    /// source account so no real key is required.
    fn build_invoke_tx(
        &self,
        contract_id: &str,
        function: &str,
        args: &[String],
    ) -> Result<String> {
        use base64::Engine;
        use serde_json::json;

        let invoke = json!({
            "type": "invokeHostFunction",
            "contract": contract_id,
            "function": function,
            "args": args,
        });

        let envelope = json!({
            "tx": {
                "sourceAccount": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                "fee": 100_000,
                "seqNum": 0,
                "operations": [invoke],
            }
        });

        let json_str = serde_json::to_string(&envelope)?;
        Ok(base64::engine::general_purpose::STANDARD.encode(json_str.as_bytes()))
    }

    fn parse_simulation_response(
        response: &serde_json::Value,
        execution_time_ms: u64,
    ) -> Result<SimulationResult> {
        let error = response.get("error").and_then(|e| e.as_str()).map(String::from);
        if let Some(ref err_msg) = error {
            return Ok(SimulationResult {
                success: false,
                gas_metrics: GasMetrics {
                    execution_time_ms,
                    ..Default::default()
                },
                return_value: None,
                events: vec![],
                state_changes: vec![],
                error: Some(err_msg.clone()),
                latest_ledger: Self::extract_ledger(response),
            });
        }

        let min_resource_fee = response
            .get("minResourceFee")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let cost = response.get("cost").unwrap_or(&serde_json::Value::Null);
        let cpu_instructions = cost
            .get("cpuInsns")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let memory_bytes = cost
            .get("memBytes")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let transaction_data = response
            .get("transactionData")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let (read_bytes, write_bytes, ledger_reads, ledger_writes) =
            Self::extract_resource_info(response);

        let refundable_fee = response
            .get("refundableFee")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let events = response
            .get("events")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|e| e.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let return_value = response
            .get("results")
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .and_then(|r| r.get("xdr"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let state_changes = response
            .get("stateChanges")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| serde_json::to_string(c).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(SimulationResult {
            success: true,
            gas_metrics: GasMetrics {
                cpu_instructions,
                memory_bytes,
                min_resource_fee,
                read_bytes,
                write_bytes,
                ledger_reads,
                ledger_writes,
                transaction_size: transaction_data.len() as u64,
                refundable_fee,
                execution_time_ms,
            },
            return_value,
            events,
            state_changes,
            error: None,
            latest_ledger: Self::extract_ledger(response),
        })
    }

    fn extract_ledger(response: &serde_json::Value) -> u32 {
        response
            .get("latestLedger")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32
    }

    fn extract_resource_info(response: &serde_json::Value) -> (u64, u64, u32, u32) {
        let resources = response
            .get("transactionData")
            .and_then(|_| response.get("minResourceFee"));

        if resources.is_none() {
            let footprint = response.get("footprint").unwrap_or(&serde_json::Value::Null);
            let reads = footprint
                .get("readOnly")
                .and_then(|v| v.as_array())
                .map(|a| a.len() as u32)
                .unwrap_or(0);
            let writes = footprint
                .get("readWrite")
                .and_then(|v| v.as_array())
                .map(|a| a.len() as u32)
                .unwrap_or(0);
            return (0, 0, reads, writes);
        }

        (0, 0, 0, 0)
    }

    /// Apply a safety margin to the estimated gas (default 15%)
    pub fn apply_safety_margin(estimated_gas: u64) -> u64 {
        estimated_gas + (estimated_gas * SAFETY_MARGIN_BPS / 10_000)
    }

    /// Compute the fee breakdown from raw gas metrics
    pub fn compute_fee_breakdown(metrics: &GasMetrics) -> GasFeeBreakdown {
        let compute_fee = metrics.min_resource_fee;
        let storage_fee = (metrics.write_bytes as i64) * 50;
        let bandwidth_fee = (metrics.transaction_size as i64) * 10;
        let refundable_fee = metrics.refundable_fee;
        let total_fee = compute_fee + storage_fee + bandwidth_fee + refundable_fee;

        GasFeeBreakdown {
            compute_fee,
            storage_fee,
            bandwidth_fee,
            refundable_fee,
            total_fee,
        }
    }

    /// Generate optimization suggestions based on gas metrics
    pub fn suggest_optimizations(metrics: &GasMetrics) -> Vec<GasOptimizationTip> {
        let mut tips = Vec::new();

        if metrics.cpu_instructions > 10_000_000 {
            tips.push(GasOptimizationTip {
                category: "compute".to_string(),
                description: "High CPU usage detected. Consider simplifying logic, \
                    caching intermediate results, or breaking into smaller transactions."
                    .to_string(),
                estimated_savings_percent: 15.0,
                priority: "high".to_string(),
            });
        }

        if metrics.write_bytes > 4096 {
            tips.push(GasOptimizationTip {
                category: "storage".to_string(),
                description: "Large storage writes detected. Use compact data structures, \
                    prefer Temporary storage for ephemeral data, and batch updates."
                    .to_string(),
                estimated_savings_percent: 20.0,
                priority: "high".to_string(),
            });
        }

        if metrics.read_bytes > 8192 {
            tips.push(GasOptimizationTip {
                category: "storage".to_string(),
                description: "High read volume. Consolidate related data into fewer \
                    ledger entries and use Instance storage for frequently accessed state."
                    .to_string(),
                estimated_savings_percent: 10.0,
                priority: "medium".to_string(),
            });
        }

        if metrics.ledger_reads > 5 {
            tips.push(GasOptimizationTip {
                category: "io".to_string(),
                description: "Multiple ledger reads detected. Minimize cross-contract \
                    calls and batch reads where possible."
                    .to_string(),
                estimated_savings_percent: 12.0,
                priority: "medium".to_string(),
            });
        }

        if metrics.transaction_size > 2048 {
            tips.push(GasOptimizationTip {
                category: "bandwidth".to_string(),
                description: "Large transaction size. Reduce parameter sizes and \
                    consider splitting into multiple smaller transactions."
                    .to_string(),
                estimated_savings_percent: 8.0,
                priority: "low".to_string(),
            });
        }

        if metrics.memory_bytes > 5_000_000 {
            tips.push(GasOptimizationTip {
                category: "memory".to_string(),
                description: "High memory usage. Avoid large allocations in loops \
                    and prefer iterating over items without collecting into vectors."
                    .to_string(),
                estimated_savings_percent: 10.0,
                priority: "medium".to_string(),
            });
        }

        tips
    }
}

/// Fee breakdown from simulation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GasFeeBreakdown {
    pub compute_fee: i64,
    pub storage_fee: i64,
    pub bandwidth_fee: i64,
    pub refundable_fee: i64,
    pub total_fee: i64,
}

/// Actionable optimization tip
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GasOptimizationTip {
    pub category: String,
    pub description: String,
    pub estimated_savings_percent: f64,
    pub priority: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_runner_creation() {
        let _runner = DryRunner::testnet();
    }

    #[test]
    fn test_safety_margin() {
        assert_eq!(DryRunner::apply_safety_margin(10_000), 11_500);
        assert_eq!(DryRunner::apply_safety_margin(0), 0);
        assert_eq!(DryRunner::apply_safety_margin(100), 115);
    }

    #[test]
    fn test_fee_breakdown() {
        let metrics = GasMetrics {
            min_resource_fee: 1000,
            write_bytes: 100,
            transaction_size: 200,
            refundable_fee: 500,
            ..Default::default()
        };
        let breakdown = DryRunner::compute_fee_breakdown(&metrics);
        assert_eq!(breakdown.compute_fee, 1000);
        assert_eq!(breakdown.storage_fee, 5000);
        assert_eq!(breakdown.bandwidth_fee, 2000);
        assert_eq!(breakdown.refundable_fee, 500);
        assert_eq!(breakdown.total_fee, 8500);
    }

    #[test]
    fn test_optimization_suggestions_empty_for_low_usage() {
        let metrics = GasMetrics::default();
        let tips = DryRunner::suggest_optimizations(&metrics);
        assert!(tips.is_empty());
    }

    #[test]
    fn test_optimization_suggestions_high_cpu() {
        let metrics = GasMetrics {
            cpu_instructions: 20_000_000,
            ..Default::default()
        };
        let tips = DryRunner::suggest_optimizations(&metrics);
        assert!(!tips.is_empty());
        assert_eq!(tips[0].category, "compute");
        assert_eq!(tips[0].priority, "high");
    }

    #[test]
    fn test_parse_error_response() {
        let response = serde_json::json!({
            "error": "Contract not found",
            "latestLedger": 12345
        });
        let result = DryRunner::parse_simulation_response(&response, 50).unwrap();
        assert!(!result.success);
        assert_eq!(result.error, Some("Contract not found".to_string()));
        assert_eq!(result.latest_ledger, 12345);
        assert_eq!(result.gas_metrics.execution_time_ms, 50);
    }

    #[test]
    fn test_parse_success_response() {
        let response = serde_json::json!({
            "cost": {
                "cpuInsns": "5000000",
                "memBytes": "1048576"
            },
            "minResourceFee": "12345",
            "refundableFee": "500",
            "transactionData": "AAAA",
            "latestLedger": 99999,
            "results": [{"xdr": "AQAAAA=="}],
            "events": ["event1", "event2"]
        });
        let result = DryRunner::parse_simulation_response(&response, 100).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_metrics.cpu_instructions, 5_000_000);
        assert_eq!(result.gas_metrics.memory_bytes, 1_048_576);
        assert_eq!(result.gas_metrics.min_resource_fee, 12345);
        assert_eq!(result.gas_metrics.refundable_fee, 500);
        assert_eq!(result.gas_metrics.execution_time_ms, 100);
        assert_eq!(result.latest_ledger, 99999);
        assert_eq!(result.return_value, Some("AQAAAA==".to_string()));
        assert_eq!(result.events.len(), 2);
    }
}
