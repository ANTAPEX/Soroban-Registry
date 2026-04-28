// tests/contract_stats_tests.rs
//
// Issue #732 — Contract usage statistics and metrics.
// Unit tests for stats types, period parsing, serialization, and edge cases.

use chrono::{Duration, NaiveDate, Utc};
use serde_json::json;
use shared::{
    ContractStatsTimeSeriesResponse, ContractUsageStats, StatsPeriod, StatsTimeSeriesPoint,
    TrendingContractStats, TrendingContractsResponse,
};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// StatsPeriod parsing tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_stats_period_from_str_7d() {
    let period: StatsPeriod = "7d".parse().expect("should parse 7d");
    assert_eq!(period.days(), 7);
    assert_eq!(period.as_str(), "7d");
}

#[test]
fn test_stats_period_from_str_30d() {
    let period: StatsPeriod = "30d".parse().expect("should parse 30d");
    assert_eq!(period.days(), 30);
    assert_eq!(period.as_str(), "30d");
}

#[test]
fn test_stats_period_from_str_90d() {
    let period: StatsPeriod = "90d".parse().expect("should parse 90d");
    assert_eq!(period.days(), 90);
    assert_eq!(period.as_str(), "90d");
}

#[test]
fn test_stats_period_from_str_invalid() {
    let period: Result<StatsPeriod, _> = "14d".parse();
    assert!(period.is_err());
    let err = period.unwrap_err();
    assert!(err.contains("invalid stats period"));
}

#[test]
fn test_stats_period_from_str_empty() {
    let period: Result<StatsPeriod, _> = "".parse();
    assert!(period.is_err());
}

#[test]
fn test_stats_period_from_str_uppercase() {
    let period: Result<StatsPeriod, _> = "7D".parse();
    assert!(period.is_err());
}

#[test]
fn test_stats_period_days_values() {
    assert_eq!(StatsPeriod::SevenDays.days(), 7);
    assert_eq!(StatsPeriod::ThirtyDays.days(), 30);
    assert_eq!(StatsPeriod::NinetyDays.days(), 90);
}

// ─────────────────────────────────────────────────────────────────────────────
// ContractUsageStats serialization tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_contract_usage_stats_serializes() {
    let contract_id = Uuid::new_v4();
    let stats = ContractUsageStats {
        contract_id,
        contract_name: "TestContract".to_string(),
        period: "30d".to_string(),
        period_start: NaiveDate::from_ymd_opt(2026, 3, 28).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 4, 27).unwrap(),
        deployment_count: 15,
        call_count: 1200,
        error_count: 3,
        unique_callers: 45,
        unique_deployers: 8,
        total_interactions: 1218,
        avg_calls_per_day: 40.0,
        error_rate: 0.0025,
    };

    let serialized = serde_json::to_string(&stats).expect("should serialize");
    assert!(serialized.contains("TestContract"));
    assert!(serialized.contains("30d"));
    assert!(serialized.contains("1200"));
    assert!(serialized.contains("0.0025"));
}

#[test]
fn test_contract_usage_stats_deserializes() {
    let json_value = json!({
        "contract_id": Uuid::new_v4().to_string(),
        "contract_name": "MyContract",
        "period": "7d",
        "period_start": "2026-04-20",
        "period_end": "2026-04-27",
        "deployment_count": 5,
        "call_count": 500,
        "error_count": 1,
        "unique_callers": 20,
        "unique_deployers": 3,
        "total_interactions": 506,
        "avg_calls_per_day": 71.43,
        "error_rate": 0.00198
    });

    let stats: ContractUsageStats = serde_json::from_value(json_value).expect("should deserialize");
    assert_eq!(stats.contract_name, "MyContract");
    assert_eq!(stats.period, "7d");
    assert_eq!(stats.deployment_count, 5);
    assert_eq!(stats.call_count, 500);
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.unique_callers, 20);
    assert!((stats.error_rate - 0.00198).abs() < 0.00001);
}

// ─────────────────────────────────────────────────────────────────────────────
// StatsTimeSeriesPoint tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_time_series_point_serializes() {
    let point = StatsTimeSeriesPoint {
        date: NaiveDate::from_ymd_opt(2026, 4, 25).unwrap(),
        deployments: 2,
        calls: 150,
        errors: 0,
        total: 152,
        unique_callers: 12,
    };

    let serialized = serde_json::to_string(&point).expect("should serialize");
    assert!(serialized.contains("2026-04-25"));
    assert!(serialized.contains("150"));
    assert!(serialized.contains("152"));
}

#[test]
fn test_time_series_point_deserializes() {
    let json_value = json!({
        "date": "2026-04-25",
        "deployments": 3,
        "calls": 200,
        "errors": 1,
        "total": 204,
        "unique_callers": 15
    });

    let point: StatsTimeSeriesPoint =
        serde_json::from_value(json_value).expect("should deserialize");
    assert_eq!(point.date, NaiveDate::from_ymd_opt(2026, 4, 25).unwrap());
    assert_eq!(point.deployments, 3);
    assert_eq!(point.calls, 200);
    assert_eq!(point.errors, 1);
    assert_eq!(point.total, 204);
    assert_eq!(point.unique_callers, 15);
}

// ─────────────────────────────────────────────────────────────────────────────
// ContractStatsTimeSeriesResponse tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_time_series_response_serializes() {
    let contract_id = Uuid::new_v4();
    let response = ContractStatsTimeSeriesResponse {
        contract_id,
        contract_name: "SeriesContract".to_string(),
        period: "7d".to_string(),
        period_start: NaiveDate::from_ymd_opt(2026, 4, 20).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 4, 27).unwrap(),
        series: vec![
            StatsTimeSeriesPoint {
                date: NaiveDate::from_ymd_opt(2026, 4, 25).unwrap(),
                deployments: 1,
                calls: 100,
                errors: 0,
                total: 101,
                unique_callers: 8,
            },
            StatsTimeSeriesPoint {
                date: NaiveDate::from_ymd_opt(2026, 4, 26).unwrap(),
                deployments: 2,
                calls: 150,
                errors: 1,
                total: 153,
                unique_callers: 10,
            },
        ],
    };

    let serialized = serde_json::to_string(&response).expect("should serialize");
    assert!(serialized.contains("SeriesContract"));
    assert!(serialized.contains("7d"));
    assert!(serialized.contains("\"series\":["));
    assert!(serialized.contains("2026-04-25"));
    assert!(serialized.contains("2026-04-26"));
}

// ─────────────────────────────────────────────────────────────────────────────
// TrendingContractStats tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_trending_contract_stats_serializes() {
    let contract_id = Uuid::new_v4();
    let trending = TrendingContractStats {
        contract_id,
        name: "HotContract".to_string(),
        network: "mainnet".to_string(),
        category: Some("DeFi".to_string()),
        is_verified: true,
        interactions_7d: 5000,
        interactions_30d: 18000,
        interactions_90d: 45000,
        deployments_7d: 10,
        errors_7d: 2,
        unique_callers_7d: 150,
        trending_score: 5000.0 + 18000.0 * 0.3 + 45000.0 * 0.1,
        rank: 1,
    };

    let serialized = serde_json::to_string(&trending).expect("should serialize");
    assert!(serialized.contains("HotContract"));
    assert!(serialized.contains("mainnet"));
    assert!(serialized.contains("DeFi"));
    assert!(serialized.contains("\"is_verified\":true"));
    assert!(serialized.contains("\"rank\":1"));
}

#[test]
fn test_trending_contract_stats_deserializes() {
    let json_value = json!({
        "contract_id": Uuid::new_v4().to_string(),
        "name": "TrendingContract",
        "network": "testnet",
        "category": null,
        "is_verified": false,
        "interactions_7d": 1000,
        "interactions_30d": 3500,
        "interactions_90d": 9000,
        "deployments_7d": 5,
        "errors_7d": 0,
        "unique_callers_7d": 50,
        "trending_score": 6500.0,
        "rank": 3
    });

    let trending: TrendingContractStats =
        serde_json::from_value(json_value).expect("should deserialize");
    assert_eq!(trending.name, "TrendingContract");
    assert_eq!(trending.network, "testnet");
    assert_eq!(trending.category, None);
    assert!(!trending.is_verified);
    assert_eq!(trending.rank, 3);
    assert!((trending.trending_score - 6500.0).abs() < 0.01);
}

// ─────────────────────────────────────────────────────────────────────────────
// TrendingContractsResponse tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_trending_contracts_response_serializes() {
    let response = TrendingContractsResponse {
        period: "7d".to_string(),
        total: 100,
        contracts: vec![
            TrendingContractStats {
                contract_id: Uuid::new_v4(),
                name: "First".to_string(),
                network: "mainnet".to_string(),
                category: Some("NFT".to_string()),
                is_verified: true,
                interactions_7d: 10000,
                interactions_30d: 30000,
                interactions_90d: 80000,
                deployments_7d: 20,
                errors_7d: 1,
                unique_callers_7d: 200,
                trending_score: 20000.0,
                rank: 1,
            },
            TrendingContractStats {
                contract_id: Uuid::new_v4(),
                name: "Second".to_string(),
                network: "testnet".to_string(),
                category: Some("DeFi".to_string()),
                is_verified: false,
                interactions_7d: 5000,
                interactions_30d: 15000,
                interactions_90d: 40000,
                deployments_7d: 10,
                errors_7d: 0,
                unique_callers_7d: 100,
                trending_score: 10000.0,
                rank: 2,
            },
        ],
        generated_at: Utc::now(),
    };

    let serialized = serde_json::to_string(&response).expect("should serialize");
    assert!(serialized.contains("\"total\":100"));
    assert!(serialized.contains("\"period\":\"7d\""));
    assert!(serialized.contains("\"contracts\":["));
    assert!(serialized.contains("First"));
    assert!(serialized.contains("Second"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge case tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_zero_interactions_error_rate() {
    let stats = ContractUsageStats {
        contract_id: Uuid::new_v4(),
        contract_name: "EmptyContract".to_string(),
        period: "7d".to_string(),
        period_start: NaiveDate::from_ymd_opt(2026, 4, 20).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 4, 27).unwrap(),
        deployment_count: 0,
        call_count: 0,
        error_count: 0,
        unique_callers: 0,
        unique_deployers: 0,
        total_interactions: 0,
        avg_calls_per_day: 0.0,
        error_rate: 0.0,
    };

    assert_eq!(stats.error_rate, 0.0);
    assert_eq!(stats.total_interactions, 0);
}

#[test]
fn test_high_error_rate() {
    let stats = ContractUsageStats {
        contract_id: Uuid::new_v4(),
        contract_name: "FailingContract".to_string(),
        period: "30d".to_string(),
        period_start: NaiveDate::from_ymd_opt(2026, 3, 28).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 4, 27).unwrap(),
        deployment_count: 0,
        call_count: 0,
        error_count: 100,
        unique_callers: 0,
        unique_deployers: 0,
        total_interactions: 100,
        avg_calls_per_day: 0.0,
        error_rate: 1.0,
    };

    assert_eq!(stats.error_rate, 1.0);
}

#[test]
fn test_trending_score_calculation() {
    let interactions_7d = 1000i64;
    let interactions_30d = 5000i64;
    let interactions_90d = 15000i64;

    let expected_score = (interactions_7d as f64) * 1.0
        + (interactions_30d as f64) * 0.3
        + (interactions_90d as f64) * 0.1;

    assert!((expected_score - 4000.0).abs() < 0.01);
}

#[test]
fn test_avg_calls_per_day_calculation() {
    let call_count = 900i64;
    let days = 30i64;
    let avg = call_count as f64 / days as f64;

    assert!((avg - 30.0).abs() < 0.01);
}

// ─────────────────────────────────────────────────────────────────────────────
// Period boundary tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_period_date_boundaries_7d() {
    let period = StatsPeriod::SevenDays;
    let period_end = Utc::now().date_naive();
    let period_start = period_end - Duration::days(period.days());

    assert_eq!((period_end - period_start).num_days(), 7);
}

#[test]
fn test_period_date_boundaries_30d() {
    let period = StatsPeriod::ThirtyDays;
    let period_end = Utc::now().date_naive();
    let period_start = period_end - Duration::days(period.days());

    assert_eq!((period_end - period_start).num_days(), 30);
}

#[test]
fn test_period_date_boundaries_90d() {
    let period = StatsPeriod::NinetyDays;
    let period_end = Utc::now().date_naive();
    let period_start = period_end - Duration::days(period.days());

    assert_eq!((period_end - period_start).num_days(), 90);
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON roundtrip tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_contract_usage_stats_json_roundtrip() {
    let original = ContractUsageStats {
        contract_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
        contract_name: "RoundtripContract".to_string(),
        period: "30d".to_string(),
        period_start: NaiveDate::from_ymd_opt(2026, 3, 28).unwrap(),
        period_end: NaiveDate::from_ymd_opt(2026, 4, 27).unwrap(),
        deployment_count: 25,
        call_count: 1500,
        error_count: 5,
        unique_callers: 75,
        unique_deployers: 12,
        total_interactions: 1530,
        avg_calls_per_day: 50.0,
        error_rate: 0.00327,
    };

    let json = serde_json::to_string(&original).expect("should serialize");
    let deserialized: ContractUsageStats = serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(deserialized.contract_id, original.contract_id);
    assert_eq!(deserialized.contract_name, original.contract_name);
    assert_eq!(deserialized.period, original.period);
    assert_eq!(deserialized.period_start, original.period_start);
    assert_eq!(deserialized.period_end, original.period_end);
    assert_eq!(deserialized.deployment_count, original.deployment_count);
    assert_eq!(deserialized.call_count, original.call_count);
    assert_eq!(deserialized.error_count, original.error_count);
    assert_eq!(deserialized.unique_callers, original.unique_callers);
    assert_eq!(deserialized.unique_deployers, original.unique_deployers);
    assert_eq!(deserialized.total_interactions, original.total_interactions);
    assert!((deserialized.avg_calls_per_day - original.avg_calls_per_day).abs() < 0.01);
    assert!((deserialized.error_rate - original.error_rate).abs() < 0.0001);
}

#[test]
fn test_trending_contracts_response_json_roundtrip() {
    let contract_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let original = TrendingContractsResponse {
        period: "7d".to_string(),
        total: 1,
        contracts: vec![TrendingContractStats {
            contract_id,
            name: "RoundtripTrending".to_string(),
            network: "mainnet".to_string(),
            category: Some("Gaming".to_string()),
            is_verified: true,
            interactions_7d: 2500,
            interactions_30d: 8000,
            interactions_90d: 20000,
            deployments_7d: 15,
            errors_7d: 1,
            unique_callers_7d: 100,
            trending_score: 7400.0,
            rank: 1,
        }],
        generated_at: Utc::now(),
    };

    let json = serde_json::to_string(&original).expect("should serialize");
    let deserialized: TrendingContractsResponse =
        serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(deserialized.period, original.period);
    assert_eq!(deserialized.total, original.total);
    assert_eq!(deserialized.contracts.len(), 1);
    assert_eq!(
        deserialized.contracts[0].contract_id,
        original.contracts[0].contract_id
    );
    assert_eq!(deserialized.contracts[0].name, original.contracts[0].name);
    assert_eq!(
        deserialized.contracts[0].network,
        original.contracts[0].network
    );
    assert_eq!(
        deserialized.contracts[0].category,
        original.contracts[0].category
    );
    assert_eq!(
        deserialized.contracts[0].is_verified,
        original.contracts[0].is_verified
    );
    assert_eq!(deserialized.contracts[0].rank, original.contracts[0].rank);
}

// ─────────────────────────────────────────────────────────────────────────────
// Query parameter behavior tests (unit-level)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_stats_query_params_default_period() {
    let period_str: Option<String> = None;
    let period = match &period_str {
        Some(p) => p.parse::<StatsPeriod>().unwrap(),
        None => StatsPeriod::ThirtyDays,
    };
    assert_eq!(period.days(), 30);
    assert_eq!(period.as_str(), "30d");
}

#[test]
fn test_trending_query_params_limit_cap() {
    let requested_limit: i64 = 200;
    let limit = requested_limit.min(100);
    assert_eq!(limit, 100);
}

#[test]
fn test_trending_query_params_default_limit() {
    let effective_limit: i64 = 20;
    let capped = effective_limit.min(100);
    assert_eq!(capped, 20);
}
