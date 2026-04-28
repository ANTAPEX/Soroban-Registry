// Security Scanning Handlers (#498)
// Automated contract security scanning integration

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    ml_detector,
    state::AppState,
};
use shared::{
    ContractSecuritySummary, CreateSecurityScannerRequest, IssueSeverity, IssueStatus,
    SecurityScan, SecurityScanHistoryResponse, SecurityScanSummary, SecurityScanner,
    SecurityScoreHistory, TriggerSecurityScanRequest, UpdateSecurityIssueRequest,
};

/// Query parameters for listing security scans
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListScansQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status: Option<String>,
    pub scan_type: Option<String>,
}

/// Trigger a security scan for a contract
///
/// POST /api/contracts/:id/scans
pub async fn trigger_security_scan(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<TriggerSecurityScanRequest>,
) -> ApiResult<Json<SecurityScan>> {
    // Verify contract exists
    let contract_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
            .bind(contract_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !contract_exists {
        return Err(ApiError::not_found("contract", "Contract not found"));
    }

    // Get contract version if specified, otherwise fall back to the latest version.
    let contract_version_id = if let Some(version) = &req.version {
        let version_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM contract_versions WHERE contract_id = $1 AND version = $2",
        )
        .bind(contract_id)
        .bind(version)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

        version_id
    } else {
        sqlx::query_scalar(
            "SELECT id FROM contract_versions WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    };

    let requested_scan_type = req.scan_type.as_deref().unwrap_or("full");
    let ml_scanner_requested = if requested_scan_type.eq_ignore_ascii_case("ml") {
        true
    } else if let Some(scanner_ids) = req.scanner_ids.as_ref() {
        if scanner_ids.is_empty() {
            false
        } else {
            let ml_scanner_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM security_scanners WHERE id = ANY($1) AND scanner_type = 'ml_local' AND is_active = true",
            )
            .bind(scanner_ids)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
            ml_scanner_count > 0
        }
    } else {
        false
    };

    if ml_scanner_requested {
        let contract_version_id = contract_version_id.ok_or_else(|| {
            ApiError::not_found(
                "contract_version",
                "ML scans require a contract version with verified source code",
            )
        })?;

        let (source_code, _verification_id) = ml_detector::source_for_contract(
            &state,
            contract_id,
            req.version.as_deref(),
        )
        .await?;

        let scan = ml_detector::persist_ml_scan(
            &state,
            contract_id,
            contract_version_id,
            source_code,
        )
        .await?;

        return Ok(Json(scan));
    }

    // Create scan record
    let scan = sqlx::query_as::<_, SecurityScan>(
        r#"
        INSERT INTO security_scans
        (contract_id, contract_version_id, status, scan_type, triggered_by_event, created_at, updated_at)
        VALUES ($1, $2, 'pending', $3, 'manual', NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(contract_version_id)
    .bind(req.scan_type.as_deref().unwrap_or("full"))
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create scan: {}", e)))?;

    // In a real implementation, this would queue the scan for processing
    // For now, we just create the record

    Ok(Json(scan))
}

/// List security scans for a contract
///
/// GET /api/contracts/:id/scans
pub async fn list_security_scans(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<ListScansQuery>,
) -> ApiResult<Json<SecurityScanHistoryResponse>> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let base_query = r#"
        SELECT 
            id, status, scan_type, total_issues, critical_issues, 
            high_issues, medium_issues, low_issues, completed_at
        FROM security_scans
        WHERE contract_id = $1
    "#;

    let count_query = r#"
        SELECT COUNT(*) FROM security_scans WHERE contract_id = $1
    "#;

    let scans = sqlx::query_as::<_, SecurityScanSummary>(&format!(
        "{} ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        base_query
    ))
    .bind(contract_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let total_count: i64 = sqlx::query_scalar(count_query)
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(SecurityScanHistoryResponse { scans, total_count }))
}

/// Get details of a specific security scan
///
/// GET /api/contracts/:id/scans/:scan_id
pub async fn get_security_scan(
    State(state): State<AppState>,
    Path((contract_id, scan_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<SecurityScan>> {
    let scan = sqlx::query_as::<_, SecurityScan>(
        "SELECT * FROM security_scans WHERE id = $1 AND contract_id = $2",
    )
    .bind(scan_id)
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("security_scan", "Security scan not found"))?;

    Ok(Json(scan))
}

/// Get security summary for a contract
///
/// GET /api/contracts/:id/security
pub async fn get_contract_security_summary(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<ContractSecuritySummary>> {
    // Get contract name
    let contract_name = sqlx::query_scalar("SELECT name FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    // Get latest scan
    let latest_scan: Option<SecurityScanSummary> = sqlx::query_as(
        r#"
        SELECT 
            id, status, scan_type, total_issues, critical_issues, 
            high_issues, medium_issues, low_issues, completed_at
        FROM security_scans
        WHERE contract_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get total scans count
    let total_scans: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM security_scans WHERE contract_id = $1")
            .bind(contract_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get open issues count
    let open_issues: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM security_issues WHERE contract_id = $1 AND status = 'open'",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get critical open issues
    let critical_open: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM security_issues WHERE contract_id = $1 AND severity = 'critical' AND status = 'open'",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get high open issues
    let high_open: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM security_issues WHERE contract_id = $1 AND severity = 'high' AND status = 'open'",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get latest security score
    let security_score: Option<i32> = sqlx::query_scalar(
        "SELECT overall_score FROM security_score_history WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .flatten();

    Ok(Json(ContractSecuritySummary {
        contract_id,
        contract_name,
        latest_scan,
        total_scans,
        open_issues,
        critical_open,
        high_open,
        security_score,
    }))
}

/// List security issues for a contract
///
/// GET /api/contracts/:id/issues
pub async fn list_security_issues(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<ListIssuesQuery>,
) -> ApiResult<Json<Vec<shared::SecurityIssue>>> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut where_clause = "WHERE contract_id = $1".to_string();
    let mut params: Vec<&(dyn sqlx::Encode<sqlx::Postgres> + Send + Sync)> = vec![&contract_id];
    let mut param_count = 1;

    if let Some(severity) = &query.severity {
        param_count += 1;
        where_clause.push_str(&format!(" AND severity = ${}", param_count));
        params.push(severity);
    }

    if let Some(status) = &query.status {
        param_count += 1;
        where_clause.push_str(&format!(" AND status = ${}", param_count));
        params.push(status);
    }

    let issues = sqlx::query_as::<_, shared::SecurityIssue>(&format!(
        "SELECT * FROM security_issues {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause,
        param_count + 1,
        param_count + 2
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(issues))
}

/// Query parameters for listing security issues
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListIssuesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub severity: Option<String>,
    pub status: Option<String>,
}

/// Update security issue status
///
/// PATCH /api/contracts/:id/issues/:issue_id
pub async fn update_security_issue(
    State(state): State<AppState>,
    Path((contract_id, issue_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateSecurityIssueRequest>,
) -> ApiResult<Json<shared::SecurityIssue>> {
    // Verify issue belongs to contract
    let existing: Option<shared::SecurityIssue> =
        sqlx::query_as("SELECT * FROM security_issues WHERE id = $1 AND contract_id = $2")
            .bind(issue_id)
            .bind(contract_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let mut issue = existing
        .ok_or_else(|| ApiError::not_found("security_issue", "Security issue not found"))?;

    let _old_status = issue.status;

    // Update issue status
    issue = sqlx::query_as::<_, shared::SecurityIssue>(
        r#"
        UPDATE security_issues
        SET status = $1, updated_at = NOW()
        WHERE id = $2 AND contract_id = $3
        RETURNING *
        "#,
    )
    .bind(&req.status)
    .bind(issue_id)
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update issue: {}", e)))?;

    // Log the action
    if _old_status != req.status {
        let _ = sqlx::query(
            r#"
            INSERT INTO security_issue_actions
            (issue_id, action_type, previous_status, new_status, notes, created_at)
            VALUES ($1, 'status_changed', $2, $3, $4, NOW())
            "#,
        )
        .bind(issue_id)
        .bind(_old_status)
        .bind(&req.status)
        .bind(&req.notes)
        .execute(&state.db)
        .await;
    }

    Ok(Json(issue))
}

/// List configured security scanners
///
/// GET /api/security/scanners
pub async fn list_security_scanners(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<SecurityScanner>>> {
    let scanners = sqlx::query_as::<_, SecurityScanner>(
        "SELECT * FROM security_scanners WHERE is_active = true ORDER BY name",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(scanners))
}

/// Register a new security scanner
///
/// POST /api/security/scanners
pub async fn create_security_scanner(
    State(state): State<AppState>,
    Json(req): Json<CreateSecurityScannerRequest>,
) -> ApiResult<Json<SecurityScanner>> {
    // In production, api_key should be encrypted before storage
    let scanner = sqlx::query_as::<_, SecurityScanner>(
        r#"
        INSERT INTO security_scanners
        (name, description, scanner_type, api_endpoint, is_active, configuration, timeout_seconds, max_concurrent_scans, created_at, updated_at)
        VALUES ($1, $2, $3, $4, true, $5, $6, 5, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.scanner_type)
    .bind(&req.api_endpoint)
    .bind(req.configuration.as_ref().unwrap_or(&serde_json::json!({})))
    .bind(req.timeout_seconds.unwrap_or(300))
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create scanner: {}", e)))?;

    Ok(Json(scanner))
}

/// Get security score history for a contract
///
/// GET /api/contracts/:id/security/score-history
pub async fn get_security_score_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<SecurityScoreHistory>>> {
    let history = sqlx::query_as::<_, SecurityScoreHistory>(
        r#"
        SELECT * FROM security_score_history
        WHERE contract_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(history))
}

/// Internal function to trigger automatic scan on contract upload
/// This would be called from the contract publish/creation handlers
pub async fn auto_scan_contract(
    state: &AppState,
    contract_id: Uuid,
    contract_version_id: Option<Uuid>,
) -> Result<Uuid, ApiError> {
    let scan = sqlx::query_as::<_, SecurityScan>(
        r#"
        INSERT INTO security_scans
        (contract_id, contract_version_id, status, scan_type, triggered_by_event, created_at, updated_at)
        VALUES ($1, $2, 'pending', 'full', 'upload', NOW(), NOW())
        RETURNING id
        "#,
    )
    .bind(contract_id)
    .bind(contract_version_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create auto scan: {}", e)))?;

    Ok(scan.id)
}
