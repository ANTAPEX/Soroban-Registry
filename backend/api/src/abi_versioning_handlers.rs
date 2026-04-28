// ABI Versioning and Compatibility Checking handlers (#733)
//
// Endpoints:
//   GET  /api/contracts/:id/abi              — latest non-deprecated ABI
//   GET  /api/contracts/:id/abi/:version     — specific version
//   POST /api/contracts/:id/abi              — publish a new ABI version
//   POST /api/contracts/:id/check-compatibility — compare two versions

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    breaking_changes::{diff_abi, ChangeSeverity},
    error::{ApiError, ApiResult},
    state::AppState,
    type_safety::parser::parse_json_spec,
};

// ── Shared DB row ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContractAbiRecord {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub abi: Value,
    pub is_deprecated: bool,
    pub deprecated_at: Option<chrono::DateTime<Utc>>,
    pub changelog: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

// ── POST /api/contracts/:id/abi ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PublishAbiRequest {
    pub version: String,
    pub abi: Value,
    pub changelog: Option<String>,
    /// If true, auto-run compatibility check against the previous version.
    pub check_compatibility: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PublishAbiResponse {
    pub record: ContractAbiRecord,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<CompatibilityReport>,
}

pub async fn publish_abi(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<PublishAbiRequest>,
) -> ApiResult<Json<PublishAbiResponse>> {
    if req.version.trim().is_empty() {
        return Err(ApiError::bad_request("version must not be empty"));
    }

    let contract_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
            .bind(contract_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    if !contract_exists {
        return Err(ApiError::not_found("contract", "Contract not found"));
    }

    let abi_str = serde_json::to_string(&req.abi)
        .map_err(|e| ApiError::bad_request(format!("Invalid ABI JSON: {}", e)))?;

    // Validate it parses as a Soroban ABI.
    parse_json_spec(&abi_str, &contract_id.to_string())
        .map_err(|e| ApiError::bad_request(format!("ABI parse error: {}", e)))?;

    let record = sqlx::query_as::<_, ContractAbiRecord>(
        r#"
        INSERT INTO contract_abis (contract_id, version, abi, changelog, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (contract_id, version) DO UPDATE
            SET abi = EXCLUDED.abi, changelog = EXCLUDED.changelog
        RETURNING id, contract_id, version, abi, is_deprecated, deprecated_at, changelog, created_at
        "#,
    )
    .bind(contract_id)
    .bind(&req.version)
    .bind(&req.abi)
    .bind(&req.changelog)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to store ABI: {}", e)))?;

    // Auto compatibility check against the previous version if requested.
    let compatibility = if req.check_compatibility.unwrap_or(false) {
        let prev = sqlx::query_as::<_, ContractAbiRecord>(
            r#"
            SELECT id, contract_id, version, abi, is_deprecated, deprecated_at, changelog, created_at
            FROM contract_abis
            WHERE contract_id = $1 AND version <> $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(contract_id)
        .bind(&req.version)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

        if let Some(prev) = prev {
            Some(run_compatibility_check(&prev, &record)?)
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(PublishAbiResponse { record, compatibility }))
}

// ── GET /api/contracts/:id/abi/:version ───────────────────────────────────

pub async fn get_abi_version(
    State(state): State<AppState>,
    Path((contract_id, version)): Path<(Uuid, String)>,
) -> ApiResult<Json<ContractAbiRecord>> {
    let record = sqlx::query_as::<_, ContractAbiRecord>(
        r#"
        SELECT id, contract_id, version, abi, is_deprecated, deprecated_at, changelog, created_at
        FROM contract_abis
        WHERE contract_id = $1 AND version = $2
        "#,
    )
    .bind(contract_id)
    .bind(&version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("abi", format!("ABI version {} not found", version)))?;

    Ok(Json(record))
}

// ── POST /api/contracts/:id/check-compatibility ───────────────────────────

#[derive(Debug, Deserialize)]
pub struct CheckCompatibilityRequest {
    /// If omitted, the two most recent versions are compared.
    pub base_version: Option<String>,
    pub new_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub base_version: String,
    pub new_version: String,
    pub breaking_changes: Vec<CompatibilityChange>,
    pub non_breaking_changes: Vec<CompatibilityChange>,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityChange {
    pub category: String,
    pub message: String,
    pub migration_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

pub async fn check_compatibility(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CheckCompatibilityRequest>,
) -> ApiResult<Json<CompatibilityReport>> {
    let (base, new_rec) = if req.base_version.is_some() || req.new_version.is_some() {
        let bv = req.base_version.ok_or_else(|| {
            ApiError::bad_request("base_version is required when new_version is specified")
        })?;
        let nv = req.new_version.ok_or_else(|| {
            ApiError::bad_request("new_version is required when base_version is specified")
        })?;
        let b = fetch_abi_record(&state, contract_id, &bv).await?;
        let n = fetch_abi_record(&state, contract_id, &nv).await?;
        (b, n)
    } else {
        // Default: compare two most recent versions.
        let rows = sqlx::query_as::<_, ContractAbiRecord>(
            r#"
            SELECT id, contract_id, version, abi, is_deprecated, deprecated_at, changelog, created_at
            FROM contract_abis
            WHERE contract_id = $1
            ORDER BY created_at DESC
            LIMIT 2
            "#,
        )
        .bind(contract_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

        if rows.len() < 2 {
            return Err(ApiError::bad_request(
                "At least two ABI versions are required for compatibility checking",
            ));
        }
        let mut iter = rows.into_iter();
        let newer = iter.next().unwrap();
        let older = iter.next().unwrap();
        (older, newer)
    };

    Ok(Json(run_compatibility_check(&base, &new_rec)?))
}

// ── Helper: fetch one ABI record ──────────────────────────────────────────

async fn fetch_abi_record(
    state: &AppState,
    contract_id: Uuid,
    version: &str,
) -> ApiResult<ContractAbiRecord> {
    sqlx::query_as::<_, ContractAbiRecord>(
        r#"
        SELECT id, contract_id, version, abi, is_deprecated, deprecated_at, changelog, created_at
        FROM contract_abis
        WHERE contract_id = $1 AND version = $2
        "#,
    )
    .bind(contract_id)
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("abi", format!("ABI version {} not found", version)))
}

// ── Helper: run diff and build report ────────────────────────────────────

fn run_compatibility_check(
    base: &ContractAbiRecord,
    new_rec: &ContractAbiRecord,
) -> ApiResult<CompatibilityReport> {
    let base_str = serde_json::to_string(&base.abi)
        .map_err(|e| ApiError::internal(format!("Serialize error: {}", e)))?;
    let new_str = serde_json::to_string(&new_rec.abi)
        .map_err(|e| ApiError::internal(format!("Serialize error: {}", e)))?;

    let base_spec = parse_json_spec(&base_str, &base.version)
        .map_err(|e| ApiError::bad_request(format!("Invalid base ABI: {}", e)))?;
    let new_spec = parse_json_spec(&new_str, &new_rec.version)
        .map_err(|e| ApiError::bad_request(format!("Invalid new ABI: {}", e)))?;

    let raw_changes = diff_abi(&base_spec, &new_spec);

    let mut breaking = Vec::new();
    let mut non_breaking = Vec::new();

    for c in raw_changes {
        let migration_note = match c.severity {
            ChangeSeverity::Breaking => Some(build_migration_note(&c.category, &c.message, c.function.as_deref())),
            ChangeSeverity::NonBreaking => None,
        };

        let change = CompatibilityChange {
            category: c.category,
            message: c.message,
            migration_note,
            function: c.function,
        };

        match c.severity {
            ChangeSeverity::Breaking => breaking.push(change),
            ChangeSeverity::NonBreaking => non_breaking.push(change),
        }
    }

    Ok(CompatibilityReport {
        compatible: breaking.is_empty(),
        base_version: base.version.clone(),
        new_version: new_rec.version.clone(),
        breaking_changes: breaking,
        non_breaking_changes: non_breaking,
    })
}

fn build_migration_note(category: &str, message: &str, function: Option<&str>) -> String {
    match category {
        "function_removed" => {
            let fname = function.unwrap_or("(unknown)");
            format!(
                "Function '{}' was removed. Audit all call sites and remove or replace invocations before upgrading.",
                fname
            )
        }
        "parameter_changed" => {
            format!(
                "Parameter signature changed: {}. Update all callers to match the new argument order or types.",
                message
            )
        }
        "return_type_changed" => {
            format!(
                "Return type changed: {}. Callers that destructure or type-check the return value must be updated.",
                message
            )
        }
        _ => format!("Breaking change detected: {}. Review all affected call sites.", message),
    }
}