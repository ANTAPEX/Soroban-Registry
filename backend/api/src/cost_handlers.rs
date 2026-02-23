use axum::{
    extract::{Path, State},
    Json,
};
use shared::models::{
    BatchCostEstimate, CostEstimate, CostEstimateRequest, CostForecast, CostOptimization,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

const SAFETY_MAX_STROOPS: i64 = 9_000_000_000_000; // 9e12: well below i64::MAX, keeps cost math bounded

fn safe_checked_mul(lhs: i64, rhs: i64, context: &str) -> ApiResult<i64> {
    lhs.checked_mul(rhs).ok_or_else(|| ApiError::unprocessable(
        "ArithmeticOverflow",
        format!("Overflow during {}", context),
    ))
}

fn safe_checked_add(lhs: i64, rhs: i64, context: &str) -> ApiResult<i64> {
    lhs.checked_add(rhs).ok_or_else(|| ApiError::unprocessable(
        "ArithmeticOverflow",
        format!("Overflow during {}", context),
    ))
}

fn safe_checked_sub(lhs: i64, rhs: i64, context: &str) -> ApiResult<i64> {
    lhs.checked_sub(rhs).ok_or_else(|| ApiError::unprocessable(
        "ArithmeticOverflow",
        format!("Underflow during {}", context),
    ))
}

fn saturating_apply_discount(amount: i64, discount_bps: i64) -> ApiResult<i64> {
    if amount < 0 {
        return Err(ApiError::unprocessable(
            "InvalidAmount",
            "Cost values must be non-negative",
        ));
    }
    if discount_bps < 0 || discount_bps > 10_000 {
        return Err(ApiError::unprocessable(
            "InvalidDiscount",
            "Discount must be in basis points between 0 and 10000",
        ));
    }
    let raw = safe_checked_mul(amount, 10_000 - discount_bps, "discount multiply")?;
    Ok(raw / 10_000)
}

fn ensure_reasonable_cost(value: i64, context: &str) -> ApiResult<i64> {
    if value > SAFETY_MAX_STROOPS {
        return Err(ApiError::unprocessable(
            "CostOutOfRange",
            format!("{} exceeds safe limit", context),
        ));
    }
    Ok(value)
}

// Stellar network constants (approximate)
const STROOPS_PER_XLM: i64 = 10_000_000;
const BASE_GAS_COST: i64 = 100_000; // stroops
const STORAGE_COST_PER_KB: i64 = 50_000; // stroops
const BANDWIDTH_COST_PER_KB: i64 = 10_000; // stroops

pub async fn estimate_cost(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CostEstimateRequest>,
) -> ApiResult<Json<CostEstimate>> {
    let invocations = req.invocations.unwrap_or(1);
    let storage_kb = req.storage_growth_kb.unwrap_or(0);

    // Try to get historical data
    let historical_gas = sqlx::query_scalar::<_, i64>(
        "SELECT avg_gas_cost FROM cost_estimates WHERE contract_id = $1 AND method_name = $2",
    )
    .bind(contract_id)
    .bind(&req.method_name)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let gas_cost = safe_checked_mul(historical_gas.unwrap_or(BASE_GAS_COST), invocations, "gas cost")?;
    let storage_cost = safe_checked_mul(storage_kb, STORAGE_COST_PER_KB, "storage cost")?;
    let bandwidth_cost = safe_checked_mul(storage_kb / 4, BANDWIDTH_COST_PER_KB, "bandwidth cost")?;

    let total_stroops = safe_checked_add(
        safe_checked_add(gas_cost, storage_cost, "total cost add 1")?,
        bandwidth_cost,
        "total cost add 2",
    )?;
    let total_stroops = ensure_reasonable_cost(total_stroops, "total cost")?;
    let total_xlm = total_stroops as f64 / STROOPS_PER_XLM as f64;

    Ok(Json(CostEstimate {
        method_name: req.method_name,
        gas_cost,
        storage_cost,
        bandwidth_cost,
        total_stroops,
        total_xlm,
        invocations,
    }))
}

pub async fn batch_estimate(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(requests): Json<Vec<CostEstimateRequest>>,
) -> ApiResult<Json<BatchCostEstimate>> {
    let mut estimates = Vec::new();
    let mut total_stroops = 0i64;

    for req in requests {
        let invocations = req.invocations.unwrap_or(1);
        let storage_kb = req.storage_growth_kb.unwrap_or(0);

        let historical_gas = sqlx::query_scalar::<_, i64>(
            "SELECT avg_gas_cost FROM cost_estimates WHERE contract_id = $1 AND method_name = $2",
        )
        .bind(contract_id)
        .bind(&req.method_name)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        let gas_cost = safe_checked_mul(historical_gas.unwrap_or(BASE_GAS_COST), invocations, "gas cost")?;
        let storage_cost = safe_checked_mul(storage_kb, STORAGE_COST_PER_KB, "storage cost")?;
        let bandwidth_cost = safe_checked_mul(storage_kb / 4, BANDWIDTH_COST_PER_KB, "bandwidth cost")?;

        let estimate_total = safe_checked_add(
            safe_checked_add(gas_cost, storage_cost, "total cost add 1")?,
            bandwidth_cost,
            "total cost add 2",
        )?;
        total_stroops = safe_checked_add(total_stroops, estimate_total, "batch total add")?;
        let estimate_total = ensure_reasonable_cost(estimate_total, "estimate total")?;
        total_stroops = ensure_reasonable_cost(total_stroops, "batch total")?;

        estimates.push(CostEstimate {
            method_name: req.method_name,
            gas_cost,
            storage_cost,
            bandwidth_cost,
            total_stroops: estimate_total,
            total_xlm: estimate_total as f64 / STROOPS_PER_XLM as f64,
            invocations,
        });
    }

    Ok(Json(BatchCostEstimate {
        estimates,
        total_stroops,
        total_xlm: total_stroops as f64 / STROOPS_PER_XLM as f64,
    }))
}

pub async fn optimize_costs(
    State(_state): State<AppState>,
    Path(_contract_id): Path<Uuid>,
    Json(estimate): Json<CostEstimate>,
) -> ApiResult<Json<CostOptimization>> {
    let mut suggestions = Vec::new();
    let current_cost = estimate.total_stroops;
    let mut optimized_cost = current_cost;

    // Suggest batching if multiple invocations
    if estimate.invocations > 1 {
        suggestions.push("Batch multiple operations into single transaction".to_string());
        optimized_cost = saturating_apply_discount(optimized_cost, 1500)?; // 15% savings
    }

    // Suggest storage optimization
    if estimate.storage_cost > estimate.gas_cost {
        suggestions.push("Optimize data structures to reduce storage footprint".to_string());
        optimized_cost = saturating_apply_discount(optimized_cost, 1000)?; // 10% savings
    }

    // Suggest caching
    if estimate.gas_cost > 500_000 {
        suggestions.push("Implement caching to reduce redundant computations".to_string());
        optimized_cost = saturating_apply_discount(optimized_cost, 800)?; // 8% savings
    }

    let savings_raw = safe_checked_sub(current_cost, optimized_cost, "savings computation")?;
    let savings_percent = if current_cost > 0 {
        (savings_raw as f64 / current_cost as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(CostOptimization {
        current_cost,
        optimized_cost,
        savings_percent,
        suggestions,
    }))
}

pub async fn forecast_costs(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CostEstimateRequest>,
) -> ApiResult<Json<CostForecast>> {
    let daily_invocations = req.invocations.unwrap_or(100);
    let storage_kb = req.storage_growth_kb.unwrap_or(1);

    let historical_gas = sqlx::query_scalar::<_, i64>(
        "SELECT avg_gas_cost FROM cost_estimates WHERE contract_id = $1 AND method_name = $2",
    )
    .bind(contract_id)
    .bind(&req.method_name)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let gas_per_call = historical_gas.unwrap_or(BASE_GAS_COST);
    let storage_cost = safe_checked_mul(storage_kb, STORAGE_COST_PER_KB, "storage cost")?;
    let bandwidth_cost = safe_checked_mul(storage_kb / 4, BANDWIDTH_COST_PER_KB, "bandwidth cost")?;
    let gas_total = safe_checked_mul(gas_per_call, daily_invocations, "gas cost")?;
    let daily_cost_stroops = safe_checked_add(
        safe_checked_add(gas_total, storage_cost, "daily cost add 1")?,
        bandwidth_cost,
        "daily cost add 2",
    )?;
    let daily_cost_stroops = ensure_reasonable_cost(daily_cost_stroops, "daily cost")?;
    let daily_cost_xlm = daily_cost_stroops as f64 / STROOPS_PER_XLM as f64;

    Ok(Json(CostForecast {
        daily_cost_xlm,
        monthly_cost_xlm: daily_cost_xlm * 30.0,
        yearly_cost_xlm: daily_cost_xlm * 365.0,
        usage_pattern: format!("{} invocations/day, {} KB storage/day", daily_invocations, storage_kb),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_checked_mul_overflow() {
        let result = safe_checked_mul(i64::MAX, 2, "mul");
        assert!(result.is_err());
    }

    #[test]
    fn safe_checked_sub_underflow() {
        let result = safe_checked_sub(1, 2, "sub");
        assert!(result.is_err());
    }

    #[test]
    fn discount_bounds() {
        let result = saturating_apply_discount(1000, 12000);
        assert!(result.is_err());
    }

    #[test]
    fn discount_applies() {
        let result = saturating_apply_discount(1000, 1000).unwrap();
        assert_eq!(result, 900);
    }
}
