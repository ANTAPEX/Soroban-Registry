use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use shared::models::{TrustFactor, TrustScore, TrustScoreBreakdown, TrustScoreWeights};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

const MAX_VERIFIED_POINTS: i32 = 30;
const MAX_AUDIT_POINTS: i32 = 25;
const MAX_USAGE_POINTS: i32 = 20;
const MAX_AGE_POINTS: i32 = 15;
const MAX_VULNERABILITY_PENALTY: i32 = 10;

pub async fn calculate_trust_score(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<TrustScoreBreakdown>> {
    let contract = sqlx::query!(
        "SELECT is_verified, created_at FROM contracts WHERE id = $1",
        contract_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    let weights = TrustScoreWeights::default();

    // Verified status (30 points max)
    let verified_points = if contract.is_verified {
        MAX_VERIFIED_POINTS
    } else {
        0
    };

    // Audit history (25 points max)
    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM security_audits WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let audit_points = (audit_count.min(5) * 5) as i32;

    // Usage count (20 points max)
    let usage_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contract_interactions WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let usage_points = ((usage_count / 10).min(20)) as i32;

    // Age (15 points max - older is more trusted)
    let age_days = (Utc::now() - contract.created_at).num_days();
    let age_points = ((age_days / 30).min(15)) as i32;

    // Vulnerability penalty (10 points max)
    let vulnerability_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM security_patches WHERE contract_id = $1 AND severity = 'critical' AND status != 'resolved'",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let vulnerability_penalty = (vulnerability_count.min(10)) as i32;

    let score = (verified_points + audit_points + usage_points + age_points - vulnerability_penalty)
        .max(0)
        .min(100);

    let tier = match score {
        90..=100 => "platinum",
        75..=89 => "gold",
        50..=74 => "silver",
        _ => "bronze",
    };

    // Store score
    sqlx::query(
        r#"
        INSERT INTO trust_scores (contract_id, score, verified_points, audit_points, usage_points, age_points, vulnerability_penalty)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (contract_id) DO UPDATE 
        SET score = $2, verified_points = $3, audit_points = $4, usage_points = $5, age_points = $6, vulnerability_penalty = $7, calculated_at = NOW()
        "#,
    )
    .bind(contract_id)
    .bind(score)
    .bind(verified_points)
    .bind(audit_points)
    .bind(usage_points)
    .bind(age_points)
    .bind(vulnerability_penalty)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to store score: {}", e)))?;

    let factors = vec![
        TrustFactor {
            name: "Verified Status".to_string(),
            points: verified_points,
            max_points: MAX_VERIFIED_POINTS,
            weight: weights.verified,
            explanation: if contract.is_verified {
                "Contract source code is verified".to_string()
            } else {
                "Contract source code not verified".to_string()
            },
        },
        TrustFactor {
            name: "Audit History".to_string(),
            points: audit_points,
            max_points: MAX_AUDIT_POINTS,
            weight: weights.audit,
            explanation: format!("{} security audits completed", audit_count),
        },
        TrustFactor {
            name: "Usage Count".to_string(),
            points: usage_points,
            max_points: MAX_USAGE_POINTS,
            weight: weights.usage,
            explanation: format!("{} contract interactions", usage_count),
        },
        TrustFactor {
            name: "Contract Age".to_string(),
            points: age_points,
            max_points: MAX_AGE_POINTS,
            weight: weights.age,
            explanation: format!("{} days since deployment", age_days),
        },
        TrustFactor {
            name: "Vulnerability Penalty".to_string(),
            points: -vulnerability_penalty,
            max_points: MAX_VULNERABILITY_PENALTY,
            weight: weights.vulnerability,
            explanation: format!("{} unresolved critical vulnerabilities", vulnerability_count),
        },
    ];

    Ok(Json(TrustScoreBreakdown {
        score,
        tier: tier.to_string(),
        factors,
        calculated_at: Utc::now(),
    }))
}

pub async fn get_trust_score(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<TrustScore>> {
    let score = sqlx::query_as::<_, TrustScore>(
        "SELECT * FROM trust_scores WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if let Some(score) = score {
        Ok(Json(score))
    } else {
        // Calculate if not exists
        let breakdown = calculate_trust_score(State(state), Path(contract_id)).await?;
        let score = sqlx::query_as::<_, TrustScore>(
            "SELECT * FROM trust_scores WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
        Ok(Json(score))
    }
}

pub async fn get_score_weights() -> ApiResult<Json<TrustScoreWeights>> {
    Ok(Json(TrustScoreWeights::default()))
}
