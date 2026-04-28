use crate::{
    state::AppState,
    state_monitor::{StateChangeEntry, AnomalyInfo},
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct StateHistoryResponse {
    pub contract_id: String,
    pub changes: Vec<StateChangeEntry>,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomaliesResponse {
    pub anomalies: Vec<AnomalyInfo>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct StateHistoryQuery {
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AnomaliesQuery {
    pub severity: Option<String>,
    pub limit: Option<i32>,
}

/// Get state change history for a contract
pub async fn get_state_history_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<StateHistoryQuery>,
) -> Result<Json<StateHistoryResponse>, ApiError> {
    let monitor = state.state_monitor.as_ref()
        .ok_or_else(|| ApiError::service_unavailable("STATE_MONITOR_DISABLED", "State monitor service is not enabled"))?;

    let limit = params.limit.unwrap_or(50).clamp(1, 1000);
    
    let changes = monitor.get_state_history(&contract_id, limit)
        .await
        .map_err(|e| ApiError::internal_error("STATE_HISTORY_ERROR", e.to_string()))?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_state_history WHERE contract_id = $1"
    )
    .bind(Uuid::parse_str(&contract_id).map_err(|_| ApiError::bad_request("INVALID_ID", "Invalid UUID"))?)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    Ok(Json(StateHistoryResponse {
        contract_id,
        changes,
        total,
    }))
}

/// Get anomalies for a specific contract or all contracts
pub async fn get_anomalies_handler(
    State(state): State<AppState>,
    Query(params): Query<AnomaliesQuery>,
) -> Result<Json<AnomaliesResponse>, ApiError> {
    let monitor = state.state_monitor.as_ref()
        .ok_or_else(|| ApiError::service_unavailable("STATE_MONITOR_DISABLED", "State monitor service is not enabled"))?;

    let limit = params.limit.unwrap_or(50).clamp(1, 1000);
    
    let anomalies = monitor.get_anomalies(None, params.severity.as_deref(), limit)
        .await
        .map_err(|e| ApiError::internal_error("ANOMALY_ERROR", e.to_string()))?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM state_anomalies WHERE is_resolved = FALSE"
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    Ok(Json(AnomaliesResponse {
        anomalies,
        total,
    }))
}

/// Get anomalies for a specific contract
pub async fn get_contract_anomalies_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<AnomaliesQuery>,
) -> Result<Json<AnomaliesResponse>, ApiError> {
    let monitor = state.state_monitor.as_ref()
        .ok_or_else(|| ApiError::service_unavailable("STATE_MONITOR_DISABLED", "State monitor service is not enabled"))?;

    let limit = params.limit.unwrap_or(50).clamp(1, 1000);
    
    let anomalies = monitor.get_anomalies(Some(&contract_id), params.severity.as_deref(), limit)
        .await
        .map_err(|e| ApiError::internal_error("ANOMALY_ERROR", e.to_string()))?;

    let contract_uuid = Uuid::parse_str(&contract_id)
        .map_err(|_| ApiError::bad_request("INVALID_ID", "Invalid UUID"))?;
    
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM state_anomalies 
         WHERE contract_id = $1 AND is_resolved = FALSE"
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    Ok(Json(AnomaliesResponse {
        anomalies,
        total,
    }))
}

/// Resolve an anomaly
pub async fn resolve_anomaly_handler(
    State(state): State<AppState>,
    Path(anomaly_id): Path<String>,
    Json(payload): Json<ResolveAnomalyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let monitor = state.state_monitor.as_ref()
        .ok_or_else(|| ApiError::service_unavailable("STATE_MONITOR_DISABLED", "State monitor service is not enabled"))?;

    monitor.resolve_anomaly(&anomaly_id, payload.resolution_notes.as_deref())
        .await
        .map_err(|e| ApiError::internal_error("RESOLVE_ERROR", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Anomaly resolved successfully",
        "anomaly_id": anomaly_id,
    })))
}

#[derive(Debug, Deserialize)]
pub struct ResolveAnomalyRequest {
    pub resolution_notes: Option<String>,
}
