//! Handlers for three-way WASM drift detection (Issue #1191)
//!
//! Endpoints:
//! - GET /api/contracts/:id/drift: Get three-way drift state for a single contract
//! - GET /api/contracts/drift: Paginated registry-wide list of contracts and their drift states

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::{ContractDriftRecord, DriftStatus, ThreeWayDriftState};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct DriftQueryParams {
    pub network: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct DriftListQueryParams {
    pub status: Option<String>,
    pub network: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DriftListResponse {
    pub contracts: Vec<ContractDriftRecord>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

/// GET /api/contracts/:id/drift
pub async fn get_contract_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<DriftQueryParams>,
) -> ApiResult<impl IntoResponse> {
    let network = params.network.unwrap_or_else(|| "testnet".to_string());

    // Check if contract drift state is recorded
    let record: Option<ContractDriftRecord> = sqlx::query_as(
        r#"
        SELECT id, contract_id, network, registry_wasm_hash, onchain_wasm_hash,
               status, diverged_leg, first_detected_at, last_checked_at,
               observed_at_ledger, created_at, updated_at
        FROM contract_drift_state
        WHERE contract_id = $1 AND network = $2
        "#,
    )
    .bind(&id)
    .bind(&network)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to query drift state: {e}")))?;

    if let Some(record) = record {
        let response = ThreeWayDriftState {
            contract_id: record.contract_id,
            lockfile_hash: None,
            registry_hash: record.registry_wasm_hash,
            onchain_hash: record.onchain_wasm_hash,
            status: record.status,
            diverged_leg: record.diverged_leg,
            first_detected_at: record.first_detected_at,
            last_checked_at: record.last_checked_at,
            observed_at_ledger: record.observed_at_ledger,
        };
        return Ok(Json(response));
    }

    // If not in contract_drift_state, check if contract exists in registry
    let contract_opt: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT contract_id, wasm_hash, created_at
        FROM contracts
        WHERE contract_id = $1 AND network = $2
        "#,
    )
    .bind(&id)
    .bind(&network)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to query contract: {e}")))?;

    match contract_opt {
        Some((contract_id, wasm_hash, created_at)) => {
            // Not yet scanned by indexer, report Unknown rather than 404
            let response = ThreeWayDriftState {
                contract_id,
                lockfile_hash: None,
                registry_hash: wasm_hash,
                onchain_hash: None,
                status: DriftStatus::Unknown,
                diverged_leg: None,
                first_detected_at: created_at,
                last_checked_at: created_at,
                observed_at_ledger: None,
            };
            Ok(Json(response))
        }
        None => Err(ApiError::not_found(
            "CONTRACT_NOT_FOUND",
            format!("Contract '{}' not found on network '{}'", id, network),
        )),
    }
}

/// GET /api/contracts/drift
pub async fn list_drift_contracts(
    State(state): State<AppState>,
    Query(params): Query<DriftListQueryParams>,
) -> ApiResult<impl IntoResponse> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let status_filter = params.status.as_deref();
    let network_filter = params.network.as_deref();

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM contract_drift_state
        WHERE ($1::varchar IS NULL OR status = $1)
          AND ($2::varchar IS NULL OR network = $2)
        "#,
    )
    .bind(status_filter)
    .bind(network_filter)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to count drift contracts: {e}")))?;

    let records: Vec<ContractDriftRecord> = sqlx::query_as(
        r#"
        SELECT id, contract_id, network, registry_wasm_hash, onchain_wasm_hash,
               status, diverged_leg, first_detected_at, last_checked_at,
               observed_at_ledger, created_at, updated_at
        FROM contract_drift_state
        WHERE ($1::varchar IS NULL OR status = $1)
          AND ($2::varchar IS NULL OR network = $2)
        ORDER BY
          CASE WHEN status = 'drift' THEN 0 ELSE 1 END,
          first_detected_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(status_filter)
    .bind(network_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to list drift contracts: {e}")))?;

    let total_pages = if total == 0 {
        1
    } else {
        (total + limit - 1) / limit
    };

    Ok(Json(DriftListResponse {
        contracts: records,
        total,
        page,
        limit,
        total_pages,
    }))
}
