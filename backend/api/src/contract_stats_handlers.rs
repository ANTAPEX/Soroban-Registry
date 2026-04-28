//! Contract usage statistics handlers (Issue #732)
//!
//! Provides endpoints for retrieving contract usage metrics and trending contracts:
//!
//!   GET /api/contracts/:id/stats          – per-contract usage stats with time-series
//!   GET /api/contracts/trending           – trending contracts ranked by usage velocity
//!
//! Stats are aggregated hourly from `contract_interaction_daily_aggregates` and
//! `contract_interactions` tables.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, NaiveDate, Utc};
use serde::Deserialize;
use shared::{
    ContractStatsTimeSeriesResponse, ContractUsageStats, StatsPeriod, StatsTimeSeriesPoint,
    TrendingContractStats, TrendingContractsResponse,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ── Query parameters ─────────────────────────────────────────────────────────

/// Query parameters for GET /api/contracts/{id}/stats
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ContractStatsQuery {
    /// Time period: 7d, 30d, 90d (default: 30d)
    pub period: Option<String>,
    /// Response format: json (default) or csv
    pub format: Option<String>,
}

/// Query parameters for GET /api/contracts/trending
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TrendingQuery {
    /// Time period for ranking: 7d, 30d, 90d (default: 7d)
    pub period: Option<String>,
    /// Maximum number of results (default: 20)
    pub limit: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/contracts/{id}/stats
///
/// Returns usage statistics for a single contract over the requested period.
/// Includes deployment counts, call counts, error rates, unique callers,
/// and a daily time-series breakdown.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/stats",
    tag = "contracts",
    params(
        ("id" = Uuid, Path, description = "Contract UUID"),
        ContractStatsQuery
    ),
    responses(
        (status = 200, description = "Contract usage statistics", body = ContractUsageStats),
        (status = 404, description = "Contract not found"),
    )
)]
pub async fn get_contract_stats(
    Path(contract_id): Path<Uuid>,
    Query(params): Query<ContractStatsQuery>,
    State(state): State<AppState>,
) -> ApiResult<Json<ContractUsageStats>> {
    let period = parse_period(&params.period)?;
    let days = period.days();
    let period_end = Utc::now().date_naive();
    let period_start = period_end - Duration::days(days);

    // Fetch contract name for the response
    let contract_name = sqlx::query_scalar::<_, String>("SELECT name FROM contracts WHERE id = $1")
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found(&format!("Contract {} not found", contract_id)))?;

    // Query aggregated stats for the period
    let stats = sqlx::query_as::<_, ContractUsageStatsRow>(
        r#"
        SELECT
            $1 AS contract_id,
            COALESCE(SUM(count) FILTER (WHERE interaction_type = 'deploy'), 0) AS deployment_count,
            COALESCE(SUM(count) FILTER (WHERE interaction_type IN ('invoke', 'transfer', 'query')), 0) AS call_count,
            COALESCE(SUM(count) FILTER (WHERE interaction_type = 'publish_failed'), 0) AS error_count,
            COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL AND ci.interaction_type IN ('invoke', 'transfer', 'query')) AS unique_callers,
            COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL AND ci.interaction_type = 'deploy') AS unique_deployers,
            COALESCE(SUM(count), 0) AS total_interactions
        FROM contract_interaction_daily_aggregates agg
        LEFT JOIN contract_interactions ci ON ci.contract_id = agg.contract_id
            AND DATE(ci.interaction_timestamp) BETWEEN $3 AND $4
            AND ci.interaction_type IN ('invoke', 'transfer', 'query', 'deploy', 'publish_failed')
        WHERE agg.contract_id = $1
          AND agg.day BETWEEN $3 AND $4
        "#,
    )
    .bind(contract_id)
    .bind(period.as_str())
    .bind(period_start)
    .bind(period_end)
    .fetch_one(&state.pool)
    .await?;

    let days_f64 = days as f64;
    let avg_calls_per_day = if days_f64 > 0.0 {
        stats.call_count as f64 / days_f64
    } else {
        0.0
    };
    let error_rate = if stats.total_interactions > 0 {
        stats.error_count as f64 / stats.total_interactions as f64
    } else {
        0.0
    };

    Ok(Json(ContractUsageStats {
        contract_id,
        contract_name,
        period: period.as_str().to_string(),
        period_start,
        period_end,
        deployment_count: stats.deployment_count,
        call_count: stats.call_count,
        error_count: stats.error_count,
        unique_callers: stats.unique_callers,
        unique_deployers: stats.unique_deployers,
        total_interactions: stats.total_interactions,
        avg_calls_per_day,
        error_rate,
    }))
}

/// GET /api/contracts/{id}/stats/timeseries
///
/// Returns daily time-series data for a contract's usage metrics.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/stats/timeseries",
    tag = "contracts",
    params(
        ("id" = Uuid, Path, description = "Contract UUID"),
        ContractStatsQuery
    ),
    responses(
        (status = 200, description = "Time-series data", body = ContractStatsTimeSeriesResponse),
        (status = 404, description = "Contract not found"),
    )
)]
pub async fn get_contract_stats_timeseries(
    Path(contract_id): Path<Uuid>,
    Query(params): Query<ContractStatsQuery>,
    State(state): State<AppState>,
) -> ApiResult<Json<ContractStatsTimeSeriesResponse>> {
    let period = parse_period(&params.period)?;
    let days = period.days();
    let period_end = Utc::now().date_naive();
    let period_start = period_end - Duration::days(days);

    let contract_name = sqlx::query_scalar::<_, String>("SELECT name FROM contracts WHERE id = $1")
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found(&format!("Contract {} not found", contract_id)))?;

    let rows = sqlx::query_as::<_, TimeSeriesRow>(
        r#"
        SELECT
            agg.day AS date,
            COALESCE(SUM(count) FILTER (WHERE interaction_type = 'deploy'), 0) AS deployments,
            COALESCE(SUM(count) FILTER (WHERE interaction_type IN ('invoke', 'transfer', 'query')), 0) AS calls,
            COALESCE(SUM(count) FILTER (WHERE interaction_type = 'publish_failed'), 0) AS errors,
            COALESCE(SUM(count), 0) AS total,
            COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL AND ci.interaction_type IN ('invoke', 'transfer', 'query')) AS unique_callers
        FROM contract_interaction_daily_aggregates agg
        LEFT JOIN contract_interactions ci ON ci.contract_id = agg.contract_id
            AND DATE(ci.interaction_timestamp) = agg.day
            AND ci.interaction_type IN ('invoke', 'transfer', 'query')
        WHERE agg.contract_id = $1
          AND agg.day BETWEEN $2 AND $3
        GROUP BY agg.day
        ORDER BY agg.day ASC
        "#,
    )
    .bind(contract_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(&state.pool)
    .await?;

    let series: Vec<StatsTimeSeriesPoint> = rows
        .into_iter()
        .map(|r| StatsTimeSeriesPoint {
            date: r.date,
            deployments: r.deployments,
            calls: r.calls,
            errors: r.errors,
            total: r.total,
            unique_callers: r.unique_callers,
        })
        .collect();

    Ok(Json(ContractStatsTimeSeriesResponse {
        contract_id,
        contract_name,
        period: period.as_str().to_string(),
        period_start,
        period_end,
        series,
    }))
}

/// GET /api/contracts/trending
///
/// Returns contracts ranked by usage velocity over the requested period.
#[utoipa::path(
    get,
    path = "/api/contracts/trending",
    tag = "contracts",
    params(TrendingQuery),
    responses(
        (status = 200, description = "Trending contracts list", body = TrendingContractsResponse),
    )
)]
pub async fn get_trending_contracts(
    Query(params): Query<TrendingQuery>,
    State(state): State<AppState>,
) -> ApiResult<Json<TrendingContractsResponse>> {
    let period = parse_period(&params.period)?;
    let limit = params.limit.unwrap_or(20).min(100);

    let column = match period {
        StatsPeriod::SevenDays => "interactions_7d",
        StatsPeriod::ThirtyDays => "interactions_30d",
        StatsPeriod::NinetyDays => "interactions_90d",
    };

    let rows = sqlx::query_as::<_, TrendingContractStats>(&format!(
        r#"
            SELECT
                contract_id,
                name,
                network,
                category,
                is_verified,
                interactions_7d,
                interactions_30d,
                interactions_90d,
                deployments_7d,
                errors_7d,
                unique_callers_7d,
                trending_score,
                ROW_NUMBER() OVER (ORDER BY {} DESC) AS rank
            FROM trending_contracts_mv
            ORDER BY {} DESC
            LIMIT $1
            "#,
        column, column
    ))
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM trending_contracts_mv")
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(TrendingContractsResponse {
        period: period.as_str().to_string(),
        total,
        contracts: rows,
        generated_at: Utc::now(),
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_period(period: &Option<String>) -> ApiResult<StatsPeriod> {
    match period {
        Some(p) => p
            .parse::<StatsPeriod>()
            .map_err(|e| ApiError::bad_request(&e)),
        None => Ok(StatsPeriod::ThirtyDays),
    }
}

// ── Internal row types ───────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct ContractUsageStatsRow {
    contract_id: Uuid,
    deployment_count: i64,
    call_count: i64,
    error_count: i64,
    unique_callers: i64,
    unique_deployers: i64,
    total_interactions: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct TimeSeriesRow {
    date: NaiveDate,
    deployments: i64,
    calls: i64,
    errors: i64,
    total: i64,
    unique_callers: i64,
}
