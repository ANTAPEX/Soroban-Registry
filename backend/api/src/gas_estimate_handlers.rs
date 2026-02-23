use axum::{
    extract::{Path, State},
    Json,
};
use shared::models::{
    GasAccuracy, GasCostBreakdown, GasEstimateRequest, GasEstimateResponse,
    GasEstimationHistory, OptimizationSuggestion,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

const STROOPS_PER_XLM: f64 = 10_000_000.0;
const SAFETY_MARGIN_BPS: i64 = 1500; // 15%

/// POST /api/contracts/:id/gas-estimate
///
/// Performs a dry-run simulation of the given contract method and returns
/// estimated gas, fee breakdown, accuracy metrics, and optimization tips.
pub async fn gas_estimate(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<GasEstimateRequest>,
) -> ApiResult<Json<GasEstimateResponse>> {
    let contract = sqlx::query_scalar::<_, String>(
        "SELECT contract_id FROM contracts WHERE id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::db_error(e.to_string()))?
    .ok_or_else(|| ApiError::not_found("ContractNotFound", "Contract not found"))?;

    let network = req.network.as_deref().unwrap_or("testnet");

    let rpc_endpoint = match network {
        "mainnet" => "https://mainnet.stellar.validationcloud.io/v1/soroban/rpc",
        "futurenet" => "https://rpc-futurenet.stellar.org",
        _ => "https://soroban-testnet.stellar.org",
    };

    let params: Vec<String> = req
        .params
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| serde_json::to_string(p).ok())
                .collect()
        })
        .unwrap_or_default();

    let start = std::time::Instant::now();

    let sim_result = simulate_contract_call(rpc_endpoint, &contract, &req.method, &params).await;
    let execution_time_ms = start.elapsed().as_millis() as u64;

    let (cpu_instructions, memory_bytes, min_resource_fee, refundable_fee, read_bytes, write_bytes, tx_size) =
        match &sim_result {
            Ok(metrics) => (
                metrics.cpu_instructions,
                metrics.memory_bytes,
                metrics.min_resource_fee,
                metrics.refundable_fee,
                metrics.read_bytes,
                metrics.write_bytes,
                metrics.transaction_size,
            ),
            Err(_) => {
                let fallback = get_historical_gas(&state, contract_id, &req.method).await;
                (
                    fallback.0 as u64,
                    fallback.1 as u64,
                    fallback.2,
                    0i64,
                    0u64,
                    0u64,
                    0u64,
                )
            }
        };

    let compute_fee = min_resource_fee;
    let storage_fee = (write_bytes as i64) * 50;
    let bandwidth_fee = (tx_size as i64) * 10;
    let total_fee = compute_fee + storage_fee + bandwidth_fee + refundable_fee;
    let total_with_margin = total_fee + (total_fee * SAFETY_MARGIN_BPS / 10_000);
    let estimated_gas = cpu_instructions + memory_bytes;
    let estimated_gas_with_margin =
        estimated_gas + (estimated_gas * SAFETY_MARGIN_BPS as u64 / 10_000);

    let accuracy = get_accuracy_stats(&state, contract_id, &req.method, network).await;

    let suggestions = generate_optimization_suggestions(
        cpu_instructions,
        memory_bytes,
        write_bytes,
        read_bytes,
        tx_size,
    );

    let _ = record_estimation(
        &state,
        contract_id,
        &req.method,
        estimated_gas_with_margin as i64,
        total_with_margin,
        network,
        cpu_instructions,
        memory_bytes,
        read_bytes,
        write_bytes,
    )
    .await;

    Ok(Json(GasEstimateResponse {
        contract_id: contract,
        method: req.method,
        estimated_gas: estimated_gas_with_margin,
        estimated_fee_stroops: total_with_margin,
        estimated_fee_xlm: total_with_margin as f64 / STROOPS_PER_XLM,
        cpu_instructions,
        memory_bytes,
        execution_time_ms,
        cost_breakdown: GasCostBreakdown {
            compute_fee,
            storage_fee,
            bandwidth_fee,
            refundable_fee,
            total_fee: total_with_margin,
        },
        optimization_suggestions: suggestions,
        accuracy,
    }))
}

/// POST /api/contracts/:id/gas-estimate/record-actual
///
/// Records the actual gas used after a real transaction, enabling accuracy tracking.
pub async fn record_actual_gas(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<RecordActualGasRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_as::<_, GasEstimationHistory>(
        "SELECT * FROM gas_estimation_history
         WHERE contract_id = $1 AND method_name = $2 AND actual_gas IS NULL
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract_id)
    .bind(&req.method_name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::db_error(e.to_string()))?;

    if let Some(record) = row {
        let deviation = if record.estimated_gas != 0 {
            ((req.actual_gas - record.estimated_gas) as f64 / record.estimated_gas as f64) * 100.0
        } else {
            0.0
        };

        sqlx::query(
            "UPDATE gas_estimation_history
             SET actual_gas = $1, actual_fee = $2, deviation_percent = $3
             WHERE id = $4",
        )
        .bind(req.actual_gas)
        .bind(req.actual_fee)
        .bind(deviation)
        .bind(record.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db_error(e.to_string()))?;

        // Update the cost_estimates table running average
        sqlx::query(
            "INSERT INTO cost_estimates (contract_id, method_name, avg_gas_cost, avg_storage_bytes, sample_count)
             VALUES ($1, $2, $3, 0, 1)
             ON CONFLICT (contract_id, method_name)
             DO UPDATE SET
                 avg_gas_cost = (cost_estimates.avg_gas_cost * cost_estimates.sample_count + $3)
                     / (cost_estimates.sample_count + 1),
                 sample_count = cost_estimates.sample_count + 1,
                 last_updated = NOW()",
        )
        .bind(contract_id)
        .bind(&req.method_name)
        .bind(req.actual_gas)
        .execute(&state.db)
        .await
        .ok();

        Ok(Json(serde_json::json!({
            "recorded": true,
            "deviation_percent": deviation,
            "within_10_percent": deviation.abs() <= 10.0,
        })))
    } else {
        Ok(Json(serde_json::json!({
            "recorded": false,
            "message": "No pending estimation found for this method",
        })))
    }
}

/// GET /api/contracts/:id/gas-estimate/accuracy
///
/// Returns historical accuracy statistics for gas estimates on this contract.
pub async fn gas_accuracy(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<MethodAccuracy>>> {
    let rows = sqlx::query_as::<_, MethodAccuracyRow>(
        "SELECT method_name, network,
                COUNT(*) AS sample_count,
                AVG(deviation_percent) AS mean_deviation,
                COUNT(*) FILTER (WHERE ABS(COALESCE(deviation_percent, 0)) <= 10.0)::FLOAT
                    / NULLIF(COUNT(*) FILTER (WHERE deviation_percent IS NOT NULL), 0)
                    AS within_10_pct
         FROM gas_estimation_history
         WHERE contract_id = $1 AND actual_gas IS NOT NULL
         GROUP BY method_name, network
         ORDER BY method_name",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db_error(e.to_string()))?;

    let result: Vec<MethodAccuracy> = rows
        .into_iter()
        .map(|r| MethodAccuracy {
            method_name: r.method_name,
            network: r.network,
            sample_count: r.sample_count,
            mean_deviation_percent: r.mean_deviation.unwrap_or(0.0),
            within_10_percent_ratio: r.within_10_pct.unwrap_or(0.0),
        })
        .collect();

    Ok(Json(result))
}

// ─── Internal helpers ────────────────────────────────────────────────────────

struct SimMetrics {
    cpu_instructions: u64,
    memory_bytes: u64,
    min_resource_fee: i64,
    refundable_fee: i64,
    read_bytes: u64,
    write_bytes: u64,
    transaction_size: u64,
}

async fn simulate_contract_call(
    rpc_endpoint: &str,
    contract_id: &str,
    function: &str,
    args: &[String],
) -> Result<SimMetrics, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    use base64::Engine;
    let invoke = serde_json::json!({
        "type": "invokeHostFunction",
        "contract": contract_id,
        "function": function,
        "args": args,
    });
    let envelope = serde_json::json!({
        "tx": {
            "sourceAccount": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            "fee": 100_000,
            "seqNum": 0,
            "operations": [invoke],
        }
    });
    let tx_xdr = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_string(&envelope).unwrap().as_bytes());

    let rpc_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": { "transaction": tx_xdr }
    });

    let response = client
        .post(rpc_endpoint)
        .header("Content-Type", "application/json")
        .json(&rpc_body)
        .send()
        .await
        .map_err(|e| format!("RPC request failed: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse RPC response: {}", e))?;

    let result = body
        .get("result")
        .ok_or_else(|| "No result in RPC response".to_string())?;

    if let Some(error) = result.get("error") {
        return Err(format!("Simulation error: {}", error));
    }

    let cost = result.get("cost").unwrap_or(&serde_json::Value::Null);

    Ok(SimMetrics {
        cpu_instructions: cost
            .get("cpuInsns")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        memory_bytes: cost
            .get("memBytes")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        min_resource_fee: result
            .get("minResourceFee")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        refundable_fee: result
            .get("refundableFee")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        read_bytes: 0,
        write_bytes: 0,
        transaction_size: result
            .get("transactionData")
            .and_then(|v| v.as_str())
            .map(|s| s.len() as u64)
            .unwrap_or(0),
    })
}

async fn get_historical_gas(state: &AppState, contract_id: Uuid, method: &str) -> (i64, i64, i64) {
    let row = sqlx::query_as::<_, (i64, i64)>(
        "SELECT avg_gas_cost, avg_storage_bytes FROM cost_estimates
         WHERE contract_id = $1 AND method_name = $2",
    )
    .bind(contract_id)
    .bind(method)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    match row {
        Some((gas, storage)) => (gas, storage, gas),
        None => (100_000, 0, 100_000),
    }
}

async fn get_accuracy_stats(
    state: &AppState,
    contract_id: Uuid,
    method: &str,
    network: &str,
) -> Option<GasAccuracy> {
    let row = sqlx::query_as::<_, AccuracyRow>(
        "SELECT
            COUNT(*) AS sample_count,
            AVG(deviation_percent) AS mean_deviation,
            PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY ABS(COALESCE(deviation_percent, 0))) AS p95_deviation,
            COUNT(*) FILTER (WHERE ABS(COALESCE(deviation_percent, 0)) <= 10.0)::FLOAT
                / NULLIF(COUNT(*) FILTER (WHERE deviation_percent IS NOT NULL), 0)
                AS within_10_pct
         FROM gas_estimation_history
         WHERE contract_id = $1 AND method_name = $2 AND network = $3 AND actual_gas IS NOT NULL",
    )
    .bind(contract_id)
    .bind(method)
    .bind(network)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    row.filter(|r| r.sample_count > 0).map(|r| GasAccuracy {
        historical_samples: r.sample_count,
        mean_deviation_percent: r.mean_deviation.unwrap_or(0.0),
        p95_deviation_percent: r.p95_deviation.unwrap_or(0.0),
        within_10_percent: r.within_10_pct.unwrap_or(0.0) * 100.0,
    })
}

async fn record_estimation(
    state: &AppState,
    contract_id: Uuid,
    method: &str,
    estimated_gas: i64,
    estimated_fee: i64,
    network: &str,
    cpu_instructions: u64,
    memory_bytes: u64,
    read_bytes: u64,
    write_bytes: u64,
) {
    let _ = sqlx::query(
        "INSERT INTO gas_estimation_history
            (contract_id, method_name, estimated_gas, estimated_fee, cpu_instructions,
             memory_bytes, read_bytes, write_bytes, network)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(contract_id)
    .bind(method)
    .bind(estimated_gas)
    .bind(estimated_fee)
    .bind(cpu_instructions as i64)
    .bind(memory_bytes as i64)
    .bind(read_bytes as i64)
    .bind(write_bytes as i64)
    .bind(network)
    .execute(&state.db)
    .await;
}

fn generate_optimization_suggestions(
    cpu: u64,
    mem: u64,
    write_bytes: u64,
    read_bytes: u64,
    tx_size: u64,
) -> Vec<OptimizationSuggestion> {
    let mut suggestions = Vec::new();

    if cpu > 10_000_000 {
        suggestions.push(OptimizationSuggestion {
            category: "compute".to_string(),
            description: "High CPU usage. Simplify logic, cache intermediate results, \
                or split into multiple smaller transactions."
                .to_string(),
            estimated_savings_percent: 15.0,
            priority: "high".to_string(),
        });
    }

    if write_bytes > 4096 {
        suggestions.push(OptimizationSuggestion {
            category: "storage".to_string(),
            description: "Large storage writes. Use compact data structures, \
                prefer Temporary storage for ephemeral data, and batch updates."
                .to_string(),
            estimated_savings_percent: 20.0,
            priority: "high".to_string(),
        });
    }

    if read_bytes > 8192 {
        suggestions.push(OptimizationSuggestion {
            category: "storage".to_string(),
            description: "High read volume. Consolidate related data into fewer \
                ledger entries and use Instance storage for hot state."
                .to_string(),
            estimated_savings_percent: 10.0,
            priority: "medium".to_string(),
        });
    }

    if tx_size > 2048 {
        suggestions.push(OptimizationSuggestion {
            category: "bandwidth".to_string(),
            description: "Large transaction size. Reduce parameter sizes or split \
                operations across multiple transactions."
                .to_string(),
            estimated_savings_percent: 8.0,
            priority: "low".to_string(),
        });
    }

    if mem > 5_000_000 {
        suggestions.push(OptimizationSuggestion {
            category: "memory".to_string(),
            description: "High memory usage. Avoid large allocations inside loops \
                and prefer streaming over collecting into vectors."
                .to_string(),
            estimated_savings_percent: 10.0,
            priority: "medium".to_string(),
        });
    }

    suggestions
}

// ─── Request/response types ──────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct RecordActualGasRequest {
    pub method_name: String,
    pub actual_gas: i64,
    pub actual_fee: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MethodAccuracy {
    pub method_name: String,
    pub network: String,
    pub sample_count: i64,
    pub mean_deviation_percent: f64,
    pub within_10_percent_ratio: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct AccuracyRow {
    sample_count: i64,
    mean_deviation: Option<f64>,
    p95_deviation: Option<f64>,
    within_10_pct: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct MethodAccuracyRow {
    method_name: String,
    network: String,
    sample_count: i64,
    mean_deviation: Option<f64>,
    within_10_pct: Option<f64>,
}
