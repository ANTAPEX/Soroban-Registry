use crate::simulation::wasm_validator::WasmValidationResult;
use serde::{Deserialize, Serialize};

const STROOPS_PER_XLM: i64 = 10_000_000;
const BASE_DEPLOYMENT_COST: i64 = 50_000;
const COST_PER_KB: i64 = 5_000;
const COST_PER_FUNCTION: i64 = 1_000;
const COST_PER_TABLE: i64 = 2_000;
const COST_PER_MEMORY_PAGE: i64 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimationResult {
    pub total_cost_stroops: i64,
    pub total_cost_xlm: f64,
    pub deployment_cost_stroops: i64,
    pub storage_cost_stroops: i64,
    pub wasm_size_kb: f64,
    pub complexity_factor: f64,
}

pub fn estimate_gas(
    wasm_bytes: &[u8],
    validation_result: &WasmValidationResult,
) -> GasEstimationResult {
    let wasm_size_bytes = wasm_bytes.len() as i64;
    let wasm_size_kb = wasm_size_bytes as f64 / 1024.0;

    // Calculate deployment cost based on WASM size
    let size_cost = (wasm_size_kb as i64) * COST_PER_KB;

    // Calculate function complexity cost
    let function_cost = validation_result.function_count as i64 * COST_PER_FUNCTION;

    // Calculate table cost
    let table_cost = validation_result.table_count as i64 * COST_PER_TABLE;

    // Calculate memory cost
    let memory_cost = validation_result.memory_pages as i64 * COST_PER_MEMORY_PAGE;

    // Total deployment cost
    let deployment_cost =
        BASE_DEPLOYMENT_COST + size_cost + function_cost + table_cost + memory_cost;

    // Storage cost estimate (based on data section)
    let storage_cost = validation_result.data_section_size as i64 * COST_PER_KB / 10;

    // Total cost
    let total_cost_stroops = deployment_cost + storage_cost;

    // Calculate complexity factor (0.0 - 1.0)
    let complexity_factor = calculate_complexity_factor(
        validation_result.function_count,
        validation_result.table_count,
        validation_result.memory_pages,
        wasm_size_kb,
    );

    let total_cost_xlm = total_cost_stroops as f64 / STROOPS_PER_XLM as f64;

    GasEstimationResult {
        total_cost_stroops,
        total_cost_xlm,
        deployment_cost_stroops: deployment_cost,
        storage_cost_stroops: storage_cost,
        wasm_size_kb,
        complexity_factor,
    }
}

fn calculate_complexity_factor(
    function_count: u32,
    table_count: u32,
    memory_pages: u64,
    wasm_size_kb: f64,
) -> f64 {
    // Normalize each factor to a 0-1 scale
    let func_factor = (function_count as f64 / 100.0).min(1.0) * 0.3;
    let table_factor = (table_count as f64 / 10.0).min(1.0) * 0.2;
    let memory_factor = (memory_pages as f64 / 1024.0).min(1.0) * 0.2;
    let size_factor = (wasm_size_kb / 100.0).min(1.0) * 0.3;

    func_factor + table_factor + memory_factor + size_factor
}
