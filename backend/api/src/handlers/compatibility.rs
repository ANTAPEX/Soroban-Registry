// handlers/compatibility.rs
// Handlers for the contract version compatibility matrix API.

use axum::{
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::{
        AddCompatibilityRequest, CompatibilityEntry, CompatibilityExportRow,
        CompatibilityMatrixResponse, CompatibilityRow,
    },
    state::AppState,
};

/// GET /api/contracts/:id/compatibility
///
/// Returns the full compatibility matrix for a given contract: what other
/// contracts / versions it is (or isn't) compatible with, grouped by the
/// source version.
pub async fn get_contract_compatibility(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<CompatibilityMatrixResponse>> {
    // Verify the contract exists
    let exists: bool = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM contracts WHERE id = $1",
        contract_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?
    .unwrap_or(false);

    if !exists {
        return Err(ApiError::not_found("NotFound", "Contract not found"));
    }

    // Fetch all compatibility rows for this contract (as source)
    let rows = sqlx::query_as!(
        CompatibilityRow,
        r#"
        SELECT
            cvc.id,
            cvc.source_contract_id,
            cvc.source_version,
            cvc.target_contract_id,
            tc.contract_id AS target_contract_stellar_id,
            tc.name AS target_contract_name,
            cvc.target_version,
            cvc.stellar_version,
            cvc.is_compatible,
            cvc.created_at,
            cvc.updated_at
        FROM contract_version_compatibility cvc
        JOIN contracts tc ON tc.id = cvc.target_contract_id
        WHERE cvc.source_contract_id = $1
        ORDER BY cvc.source_version, tc.name, cvc.target_version
        "#,
        contract_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    // Group by source_version
    let mut versions: std::collections::BTreeMap<String, Vec<CompatibilityEntry>> =
        std::collections::BTreeMap::new();

    for row in &rows {
        let entry = CompatibilityEntry {
            target_contract_id: row.target_contract_id,
            target_contract_stellar_id: row.target_contract_stellar_id.clone(),
            target_contract_name: row.target_contract_name.clone(),
            target_version: row.target_version.clone(),
            stellar_version: row.stellar_version.clone(),
            is_compatible: row.is_compatible,
        };
        versions
            .entry(row.source_version.clone())
            .or_default()
            .push(entry);
    }

    // Build a list of incompatible warnings
    let warnings: Vec<String> = rows
        .iter()
        .filter(|r| !r.is_compatible)
        .map(|r| {
            format!(
                "Version {} is INCOMPATIBLE with {} v{}",
                r.source_version, r.target_contract_name, r.target_version
            )
        })
        .collect();

    let response = CompatibilityMatrixResponse {
        contract_id,
        versions,
        warnings,
        total_entries: rows.len(),
    };

    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct ExportFormat {
    /// "csv" or "json" (default: json)
    pub format: Option<String>,
}

/// GET /api/contracts/:id/compatibility/export?format=csv|json
///
/// Exports the full compatibility matrix for a contract as CSV or JSON.
pub async fn export_contract_compatibility(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(params): Query<ExportFormat>,
) -> ApiResult<impl IntoResponse> {
    let rows = sqlx::query_as!(
        CompatibilityExportRow,
        r#"
        SELECT
            cvc.source_version,
            tc.contract_id AS target_contract_stellar_id,
            tc.name AS target_contract_name,
            cvc.target_version,
            cvc.stellar_version,
            cvc.is_compatible
        FROM contract_version_compatibility cvc
        JOIN contracts tc ON tc.id = cvc.target_contract_id
        WHERE cvc.source_contract_id = $1
        ORDER BY cvc.source_version, tc.name, cvc.target_version
        "#,
        contract_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let format = params.format.as_deref().unwrap_or("json");

    match format {
        "csv" => {
            let mut csv = String::from(
                "source_version,target_contract_stellar_id,target_contract_name,target_version,stellar_version,is_compatible\n",
            );
            for r in &rows {
                csv.push_str(&format!(
                    "{},{},{},{},{},{}\n",
                    r.source_version,
                    r.target_contract_stellar_id,
                    r.target_contract_name,
                    r.target_version,
                    r.stellar_version.as_deref().unwrap_or(""),
                    r.is_compatible
                ));
            }
            Ok((
                axum::http::StatusCode::OK,
                [(
                    header::CONTENT_TYPE,
                    "text/csv; charset=utf-8",
                ),(
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"compatibility.csv\"",
                )],
                csv,
            )
                .into_response())
        }
        _ => {
            // JSON (default)
            Ok(Json(rows).into_response())
        }
    }
}

/// POST /api/contracts/:id/compatibility
///
/// Add or update a compatibility entry for this contract.
pub async fn add_contract_compatibility(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(body): Json<AddCompatibilityRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate that target contract exists
    let target_exists: bool = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM contracts WHERE id = $1",
        body.target_contract_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?
    .unwrap_or(false);

    if !target_exists {
        return Err(ApiError::not_found(
            "NotFound",
            "Target contract not found",
        ));
    }

    sqlx::query(
        r#"
        INSERT INTO contract_version_compatibility
            (source_contract_id, source_version, target_contract_id, target_version, stellar_version, is_compatible)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (source_contract_id, source_version, target_contract_id, target_version)
        DO UPDATE SET
            stellar_version = EXCLUDED.stellar_version,
            is_compatible = EXCLUDED.is_compatible,
            updated_at = NOW()
        "#,
    )
    .bind(contract_id)
    .bind(&body.source_version)
    .bind(body.target_contract_id)
    .bind(&body.target_version)
    .bind(&body.stellar_version)
    .bind(body.is_compatible)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(serde_json::json!({
        "message": "Compatibility entry saved",
        "source_contract_id": contract_id,
        "source_version": body.source_version,
        "target_contract_id": body.target_contract_id,
        "target_version": body.target_version,
        "is_compatible": body.is_compatible,
    })))
}
