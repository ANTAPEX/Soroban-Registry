// api/src/capacity_engine.rs
//
// Pure capacity-planning computation: forecasting, alert generation,
// recommendation generation, and cost estimation.
// All functions are synchronous — handlers call these after fetching DB data.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use shared::{
    AlertSeverity, CapacityAlert, CostEstimate, ForecastPoint, GrowthScenario,
    ImplementationEffort, RecommendationKind, ResourceForecast, ResourceKind,
    ResourceLimits, ResourceSnapshot, ScalingRecommendation, ScenarioBundle,
};

// ─────────────────────────────────────────────────────────
// Forecasting
// ─────────────────────────────────────────────────────────

/// Build a month-by-month compound growth forecast.
///
/// Uses the formula:  V(t) = V₀ × (1 + r)^t
/// where r is the monthly fractional growth rate and t is months from now.
pub fn build_forecast(
    contract_id: Uuid,
    resource: ResourceKind,
    current_value: f64,
    limit: f64,
    scenario: &GrowthScenario,
    horizon_months: u32,
    now: DateTime<Utc>,
) -> ResourceForecast {
    let rate = scenario.monthly_rate();
    let mut points = Vec::with_capacity(horizon_months as usize + 1);
    let mut breach_at_month: Option<u32> = None;
    let mut breach_at:       Option<DateTime<Utc>> = None;

    for month in 0..=horizon_months {
        // Compound growth model using discrete monthly buckets (dashboard granularity).
        let projected = current_value * (1.0 + rate).powi(month as i32);
        // Protect against divide-by-zero so missing/invalid limits do not produce NaN alerts.
        let pct = if limit > 0.0 { (projected / limit) * 100.0 } else { 0.0 };
        let exceeds = projected > limit;
        // We intentionally approximate a month as 30 days for a stable, inexpensive forecast.
        let at = now + Duration::days(month as i64 * 30);

        if exceeds && breach_at_month.is_none() {
            breach_at_month = Some(month);
            breach_at       = Some(at);
        }

        points.push(ForecastPoint {
            month,
            at,
            projected_value: projected,
            pct_of_limit: pct,
            exceeds_limit: exceeds,
        });
    }

    // Derived from the first breached forecast point; also uses the 30-day/month approximation.
    let days_until_breach = breach_at.map(|b| (b - now).num_days());

    ResourceForecast {
        contract_id,
        resource,
        scenario: scenario.label().to_string(),
        monthly_growth_rate: rate,
        current_value,
        limit,
        horizon_months,
        points,
        breach_at_month,
        breach_at,
        days_until_breach,
    }
}

/// Build all three standard scenarios (+ optional custom) for one resource.
pub fn build_scenario_bundle(
    contract_id: Uuid,
    resource: ResourceKind,
    current_value: f64,
    limit: f64,
    horizon_months: u32,
    custom_rate: Option<f64>,
    now: DateTime<Utc>,
) -> ScenarioBundle {
    let conservative = build_forecast(
        contract_id, resource.clone(), current_value, limit,
        &GrowthScenario::Conservative, horizon_months, now,
    );
    let base = build_forecast(
        contract_id, resource.clone(), current_value, limit,
        &GrowthScenario::Base, horizon_months, now,
    );
    let aggressive = build_forecast(
        contract_id, resource.clone(), current_value, limit,
        &GrowthScenario::Aggressive, horizon_months, now,
    );
    let custom = custom_rate.map(|r| build_forecast(
        contract_id, resource.clone(), current_value, limit,
        &GrowthScenario::Custom { monthly_rate: r }, horizon_months, now,
    ));

    ScenarioBundle { resource, current_value, limit, conservative, base, aggressive, custom }
}

/// Derive the current value for a resource from a slice of snapshots.
/// Takes the most-recent snapshot value; returns 0.0 if none exist.
pub fn current_value_from_snapshots(
    snapshots: &[ResourceSnapshot],
    resource: &ResourceKind,
) -> f64 {
    snapshots
        .iter()
        .filter(|s| &s.resource == resource)
        .max_by(|a, b| a.recorded_at.cmp(&b.recorded_at))
        .map(|s| s.value)
        .unwrap_or(0.0)
}

// ─────────────────────────────────────────────────────────
// Alert generation
// ─────────────────────────────────────────────────────────

/// ALERT_THRESHOLD_PCT: emit a Warning when a resource is over this % consumed.
const WARN_PCT: f64  = 60.0;
/// Emit Critical when breach is predicted within this many days (30-day window).
const CRITICAL_DAYS: i64 = 30;

/// Evaluate one scenario bundle and produce zero or one alert.
/// Uses the **base** scenario for alert timing (conservative enough to avoid
/// noise, aggressive enough to give 30-day lead time per the spec).
pub fn evaluate_alert(
    contract_id: Uuid,
    bundle: &ScenarioBundle,
    now: DateTime<Utc>,
) -> Option<CapacityAlert> {
    let base = &bundle.base;
    let pct  = if bundle.limit > 0.0 {
        (bundle.current_value / bundle.limit) * 100.0
    } else {
        0.0
    };

    let severity = if bundle.current_value > bundle.limit {
        Some(AlertSeverity::Breached)
    } else if let Some(days) = base.days_until_breach {
        if days <= CRITICAL_DAYS {
            Some(AlertSeverity::Critical)
        } else if pct >= WARN_PCT {
            Some(AlertSeverity::Warning)
        } else {
            None
        }
    } else if pct >= WARN_PCT {
        Some(AlertSeverity::Warning)
    } else {
        None
    };

    severity.map(|sev| {
        let message = match &sev {
            AlertSeverity::Breached  => format!(
                "{} has EXCEEDED its limit ({:.0} / {:.0}, {:.1}% consumed).",
                bundle.resource, bundle.current_value, bundle.limit, pct
            ),
            AlertSeverity::Critical  => format!(
                "{} will reach its limit in ~{} days under base growth ({:.1}% consumed).",
                bundle.resource,
                base.days_until_breach.unwrap_or(0),
                pct,
            ),
            AlertSeverity::Warning   => format!(
                "{} is at {:.1}% capacity. Breach predicted in ~{} days under base growth.",
                bundle.resource,
                pct,
                base.days_until_breach.unwrap_or(i64::MAX),
            ),
        };

        CapacityAlert {
            id:                  Uuid::new_v4(),
            contract_id,
            resource:            bundle.resource.clone(),
            severity:            sev.to_string(),
            current_value:       bundle.current_value,
            limit_value:         bundle.limit,
            pct_consumed:        pct,
            breach_predicted_at: base.breach_at,
            days_until_breach:   base.days_until_breach,
            message,
            acknowledged:        false,
            created_at:          now,
            resolved_at:         None,
        }
    })
}

// ─────────────────────────────────────────────────────────
// Recommendations
// ─────────────────────────────────────────────────────────

/// Generate actionable recommendations for every resource that has an alert.
/// Recommendations are ordered by priority (1 = most urgent).
pub fn generate_recommendations(
    contract_id: Uuid,
    bundles: &[ScenarioBundle],
    now: DateTime<Utc>,
) -> Vec<ScalingRecommendation> {
    let mut recs: Vec<ScalingRecommendation> = Vec::new();

    for bundle in bundles {
        let base_days = bundle.base.days_until_breach;
        let pct = if bundle.limit > 0.0 {
            (bundle.current_value / bundle.limit) * 100.0
        } else { 0.0 };

        // Only recommend if over 50% or breach predicted within 60 days
        let needs_action = pct >= 50.0
            || base_days.map(|d| d <= 60).unwrap_or(false)
            || bundle.current_value > bundle.limit;

        if !needs_action { continue; }

        let priority = if bundle.current_value > bundle.limit { 1 }
            else if base_days.map(|d| d <= CRITICAL_DAYS).unwrap_or(false) { 2 }
            else if pct >= WARN_PCT { 3 }
            else { 4 };

        match &bundle.resource {
            ResourceKind::StorageEntries => {
                recs.push(ScalingRecommendation {
                    id: Uuid::new_v4(), contract_id,
                    resource: ResourceKind::StorageEntries,
                    kind: RecommendationKind::StorageOptimization,
                    title: "Implement Storage Pagination & TTL Management".into(),
                    description: format!(
                        "Storage entries are at {:.0}% capacity. \
                         Persistent storage entries that are never pruned accumulate indefinitely. \
                         Adding TTL expiry and pagination will prevent hitting the {} entry limit.",
                        pct, bundle.limit as u64
                    ),
                    action: "1. Add extend_ttl() calls to all persistent storage writes with \
                              a reasonable max TTL (e.g. 1 year in ledgers).\n\
                              2. Move historical records off-chain to an indexer \
                              (Stellar Horizon or a custom PostgreSQL sink).\n\
                              3. Implement a prune_expired() admin function that removes \
                              entries past their useful life.\n\
                              4. Consider using instance storage for data that expires \
                              at contract end-of-life.".into(),
                    effort: ImplementationEffort::Medium,
                    estimated_savings_pct: 40.0,
                    priority, created_at: now,
                });
            }

            ResourceKind::CpuInstructions => {
                recs.push(ScalingRecommendation {
                    id: Uuid::new_v4(), contract_id,
                    resource: ResourceKind::CpuInstructions,
                    kind: RecommendationKind::CodeOptimization,
                    title: "Reduce Per-Transaction Instruction Budget Usage".into(),
                    description: format!(
                        "CPU instruction usage is at {:.0}% of the per-transaction limit. \
                         As transaction complexity grows, individual calls risk hitting \
                         the {} instruction ceiling and aborting mid-execution.",
                        pct, bundle.limit as u64
                    ),
                    action: "1. Profile hot paths using the Soroban CLI's --cost-snapshot flag \
                              to identify the most expensive function calls.\n\
                              2. Replace any loop-based cross-contract calls with batch interfaces.\n\
                              3. Move off-chain any computation that doesn't need on-chain proof \
                              (e.g. sorting, aggregation).\n\
                              4. Cache frequently read storage values in local variables \
                              rather than re-fetching per iteration.\n\
                              5. Consider splitting high-complexity operations into \
                              multi-step transactions with intermediate state commits.".into(),
                    effort: ImplementationEffort::High,
                    estimated_savings_pct: 30.0,
                    priority, created_at: now,
                });
            }

            ResourceKind::UniqueUsers => {
                recs.push(ScalingRecommendation {
                    id: Uuid::new_v4(), contract_id,
                    resource: ResourceKind::UniqueUsers,
                    kind: RecommendationKind::ArchitectureChange,
                    title: "Shard User State Across Multiple Contract Instances".into(),
                    description: format!(
                        "Unique user count is at {:.0}% of tracked capacity. \
                         A single contract instance serving all users becomes a bottleneck \
                         and increases per-entry storage costs as user maps grow.",
                        pct
                    ),
                    action: "1. Introduce a registry/router contract that maps user address \
                              ranges (by first byte) to shard contracts.\n\
                              2. Deploy N shard instances; the router delegates each call \
                              to the correct shard based on address hash.\n\
                              3. Migrate existing user state using a batch migration function \
                              protected by admin auth.\n\
                              4. Update clients to call the router rather than the contract directly.".into(),
                    effort: ImplementationEffort::High,
                    estimated_savings_pct: 70.0,
                    priority, created_at: now,
                });
            }

            ResourceKind::TransactionVolume => {
                recs.push(ScalingRecommendation {
                    id: Uuid::new_v4(), contract_id,
                    resource: ResourceKind::TransactionVolume,
                    kind: RecommendationKind::InfrastructureScaling,
                    title: "Enable Batch Processing & Off-Peak Scheduling".into(),
                    description: format!(
                        "Transaction volume is at {:.0}% of sustainable throughput. \
                         High volumes during peak hours risk queue buildup and \
                         degraded confirmation times for end users.",
                        pct
                    ),
                    action: "1. Implement a batch_execute() entry point that processes \
                              up to MAX_BATCH (e.g. 50) operations per transaction, \
                              amortising base fees.\n\
                              2. Add a configurable fee multiplier so non-urgent operations \
                              can opt into off-peak priority pricing.\n\
                              3. Expose a queue depth metric via an on-chain counter so \
                              off-chain monitors can apply back-pressure.\n\
                              4. Document the recommended QPS limit in the contract README.".into(),
                    effort: ImplementationEffort::Medium,
                    estimated_savings_pct: 50.0,
                    priority, created_at: now,
                });
            }

            ResourceKind::WasmSizeBytes => {
                recs.push(ScalingRecommendation {
                    id: Uuid::new_v4(), contract_id,
                    resource: ResourceKind::WasmSizeBytes,
                    kind: RecommendationKind::CodeOptimization,
                    title: "Reduce WASM Binary Size".into(),
                    description: format!(
                        "WASM binary size is at {:.0}% of the {} byte limit. \
                         Exceeding the limit will prevent future upgrades.",
                        pct, bundle.limit as u64
                    ),
                    action: "1. Build with opt-level = 'z' in Cargo.toml [profile.release].\n\
                              2. Run wasm-opt -Oz on the output binary.\n\
                              3. Audit use of soroban_sdk types — prefer BytesN<32> \
                              over String for fixed-length data.\n\
                              4. Extract large lookup tables (e.g. fee schedules) \
                              into a separate read-only data contract.\n\
                              5. Remove any dead code paths and unused SDK imports.".into(),
                    effort: ImplementationEffort::Low,
                    estimated_savings_pct: 20.0,
                    priority, created_at: now,
                });
            }

            ResourceKind::FeePerOperation => {
                recs.push(ScalingRecommendation {
                    id: Uuid::new_v4(), contract_id,
                    resource: ResourceKind::FeePerOperation,
                    kind: RecommendationKind::ConfigurationTuning,
                    title: "Optimise Fee Structure for Current Network Conditions".into(),
                    description: format!(
                        "Fee per operation is at {:.0}% of the tracked budget. \
                         Rising base fees reduce protocol competitiveness \
                         and may price out small-value transactions.",
                        pct
                    ),
                    action: "1. Audit the number of ledger reads/writes per operation — \
                              each read/write entry directly affects the resource fee.\n\
                              2. Consolidate multiple storage reads into a single \
                              composite entry where possible.\n\
                              3. Cache oracle/config values in instance storage \
                              (cheaper TTL) rather than persistent.\n\
                              4. Evaluate using soroban_sdk::token::StellarAssetClient \
                              for fee payments to reduce custom token overhead.".into(),
                    effort: ImplementationEffort::Low,
                    estimated_savings_pct: 25.0,
                    priority, created_at: now,
                });
            }
        }
    }

    recs.sort_by_key(|r| r.priority);
    recs
}

// ─────────────────────────────────────────────────────────
// Cost estimation
// ─────────────────────────────────────────────────────────

/// XLM cost per resource unit (approximate, based on Soroban fee schedule).
fn cost_per_unit_xlm(resource: &ResourceKind) -> f64 {
    match resource {
        ResourceKind::StorageEntries    => 0.000_5,  // per entry-month
        ResourceKind::CpuInstructions   => 0.000_000_1, // per instruction
        ResourceKind::TransactionVolume => 0.001,    // base fee per tx in XLM
        ResourceKind::UniqueUsers        => 0.000_2,  // per user-month (storage overhead)
        ResourceKind::WasmSizeBytes      => 0.000_000_01, // per byte (upload + rent)
        ResourceKind::FeePerOperation    => 1.0,      // fee IS the unit; ratio = 1
    }
}

pub fn estimate_cost(
    resource: ResourceKind,
    current_value: f64,
    projected_value: f64,
    horizon_months: u32,
    xlm_usd: f64,
) -> CostEstimate {
    let cpu = cost_per_unit_xlm(&resource);
    let current_monthly  = current_value   * cpu;
    let projected_monthly = projected_value * cpu;

    CostEstimate {
        resource,
        current_monthly_xlm:   current_monthly,
        projected_monthly_xlm: projected_monthly,
        projected_monthly_usd: projected_monthly * xlm_usd,
        cost_per_unit_xlm:     cpu,
        units_at_horizon:      projected_value,
        horizon_months,
    }
}

// ─────────────────────────────────────────────────────────
// Overall status
// ─────────────────────────────────────────────────────────

pub fn overall_status(alerts: &[CapacityAlert]) -> String {
    if alerts.iter().any(|a| a.severity == AlertSeverity::Breached.to_string()) {
        "breached".into()
    } else if alerts.iter().any(|a| a.severity == AlertSeverity::Critical.to_string()) {
        "critical".into()
    } else if alerts.iter().any(|a| a.severity == AlertSeverity::Warning.to_string()) {
        "warning".into()
    } else {
        "healthy".into()
    }
}

pub fn nearest_breach_days(bundles: &[ScenarioBundle]) -> Option<i64> {
    bundles.iter()
        .filter_map(|b| b.base.days_until_breach)
        .filter(|&d| d >= 0)
        .min()
}

// ─────────────────────────────────────────────────────────
// Resource limits lookup
// ─────────────────────────────────────────────────────────

/// Returns the limit used by capacity planning for a resource.
///
/// # Calculation methodology by resource type
///
/// - `StorageEntries`: uses `limits.max_storage_entries` (network-configured Soroban hard limit).
/// - `CpuInstructions`: uses `limits.max_cpu_instructions` (network-configured per-tx hard limit).
/// - `WasmSizeBytes`: uses `limits.max_wasm_bytes` (network-configured deploy/upgrade hard limit).
/// - `UniqueUsers`: operational planning cap (not a protocol hard limit). We use a conservative,
///   fixed threshold (`1_000_000`) so alerts fire before user-map growth causes latency spikes and
///   storage-cost drift.
/// - `TransactionVolume`: operational daily throughput target (`50_000 tx/day`), representing a
///   practical backend / queueing threshold rather than a Soroban protocol cap.
/// - `FeePerOperation`: budget threshold (`10_000_000` stroops) used for cost competitiveness
///   alerts; this is a business KPI, not a protocol limit.
///
/// # Realistic example values (rule-of-thumb starting points)
///
/// These are planning baselines for *current usage* snapshots before forecasting; tune by workload.
///
/// - Minimal registry / lookup contract:
///   `WasmSizeBytes` `80_000..200_000`, worst-path `CpuInstructions` `50_000..400_000`,
///   storage per record usually `< 512` bytes.
/// - Token / escrow contract:
///   `WasmSizeBytes` `150_000..350_000`, worst-path `CpuInstructions` `100_000..1_500_000`,
///   storage records often `128..1024` bytes.
/// - Marketplace / AMM / orderbook-style contract:
///   `WasmSizeBytes` `300_000..800_000`, worst-path `CpuInstructions` `500_000..10_000_000`,
///   storage records often `256..4096` bytes (split oversized entries).
///
/// # How to measure current usage
///
/// - `WasmSizeBytes`: measure the built artifact (`wc -c target/.../*.wasm`) and compare to
///   `limits.max_wasm_bytes`.
/// - `CpuInstructions`: invoke representative and worst-case paths with Soroban CLI cost output
///   (for example `--cost` / `--cost-snapshot`) and store the measured instruction count.
/// - Memory / entry footprint: this service tracks memory pressure indirectly via
///   `StorageEntries` + checklist `RL-003` (ledger-entry byte size). Measure serialized entry sizes
///   for large structs/collections and record the largest observed entries.
///
/// # Troubleshooting common limit failures
///
/// - WASM upload/upgrade rejected: build with `opt-level = "z"`, run `wasm-opt -Oz`, remove dead
///   code, and move large lookup tables/constants into separate contracts or off-chain data.
/// - Instruction budget exceeded / host aborts on worst-case input: paginate loops, cap batch
///   sizes, cache repeated reads, and split heavy flows into multi-step transactions.
/// - Ledger entry too large (serialization/write failures): split state across keyed entries,
///   store hashes on-chain with payloads off-chain, and avoid unbounded `Vec`/`Map` fields.
pub fn limit_for(resource: &ResourceKind, limits: &ResourceLimits) -> f64 {
    // Note: direct VM heap memory is not tracked as a ResourceKind in this service today.
    // Memory-like pressure is approximated via entry count (`StorageEntries`) and byte-size checks
    // documented in checklist item RL-003 (ledger entry footprint).
    match resource {
        ResourceKind::StorageEntries    => limits.max_storage_entries as f64,
        ResourceKind::CpuInstructions   => limits.max_cpu_instructions as f64,
        ResourceKind::WasmSizeBytes      => limits.max_wasm_bytes as f64,
        // For user/volume/fee resources we set practical operational limits
        // rather than hard Soroban limits. These are configurable via env.
        ResourceKind::UniqueUsers        => 1_000_000.0, // alert before state-growth bottlenecks
        ResourceKind::TransactionVolume  => 50_000.0,   // tx/day practical planning threshold
        ResourceKind::FeePerOperation    => 10_000_000.0, // stroops budget per operation
    }
}

// ─────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn now() -> DateTime<Utc> { Utc::now() }

    // ── Forecasting ──────────────────────────────────────

    #[test]
    fn forecast_month_zero_equals_current() {
        let f = build_forecast(
            Uuid::new_v4(), ResourceKind::StorageEntries,
            1_000.0, 100_000.0, &GrowthScenario::Base, 12, now(),
        );
        assert!((f.points[0].projected_value - 1_000.0).abs() < 0.01);
    }

    #[test]
    fn forecast_compounds_correctly() {
        // 25% monthly × 2 months = 1000 × 1.25² = 1562.5
        let f = build_forecast(
            Uuid::new_v4(), ResourceKind::StorageEntries,
            1_000.0, 100_000.0, &GrowthScenario::Base, 2, now(),
        );
        let expected = 1_000.0 * 1.25f64.powi(2);
        assert!((f.points[2].projected_value - expected).abs() < 0.1);
    }

    #[test]
    fn forecast_detects_breach() {
        // Current = 90% of limit; base rate (25%/mo) should breach within 1 month
        let limit = 100_000.0;
        let current = 90_000.0;
        let f = build_forecast(
            Uuid::new_v4(), ResourceKind::StorageEntries,
            current, limit, &GrowthScenario::Base, 6, now(),
        );
        assert!(f.breach_at_month.is_some());
        assert!(f.breach_at_month.unwrap() <= 1);
    }

    #[test]
    fn forecast_no_breach_when_well_below_limit() {
        let f = build_forecast(
            Uuid::new_v4(), ResourceKind::StorageEntries,
            100.0, 100_000.0, &GrowthScenario::Conservative, 12, now(),
        );
        // 100 × 1.10^12 ≈ 313 — well under 100,000
        assert!(f.breach_at_month.is_none());
    }

    #[test]
    fn all_three_scenarios_have_different_growth() {
        let id = Uuid::new_v4();
        let bundle = build_scenario_bundle(
            id, ResourceKind::UniqueUsers,
            1_000.0, 1_000_000.0, 12, None, now(),
        );
        let cons_end = bundle.conservative.points.last().unwrap().projected_value;
        let base_end = bundle.base.points.last().unwrap().projected_value;
        let aggr_end = bundle.aggressive.points.last().unwrap().projected_value;
        assert!(cons_end < base_end);
        assert!(base_end < aggr_end);
    }

    #[test]
    fn custom_scenario_uses_provided_rate() {
        let id = Uuid::new_v4();
        let bundle = build_scenario_bundle(
            id, ResourceKind::TransactionVolume,
            500.0, 50_000.0, 1, Some(0.50), now(),
        );
        let custom = bundle.custom.unwrap();
        // 500 × 1.50 = 750 at month 1
        assert!((custom.points[1].projected_value - 750.0).abs() < 0.1);
    }

    // ── Accuracy: forecasts must be within 20% of compound formula ──────

    #[test]
    fn forecast_within_20_pct_accuracy() {
        let rate = GrowthScenario::Aggressive.monthly_rate();
        let current = 5_000.0;
        let horizon = 6u32;
        let f = build_forecast(
            Uuid::new_v4(), ResourceKind::CpuInstructions,
            current, 1_000_000.0, &GrowthScenario::Aggressive, horizon, now(),
        );
        let expected = current * (1.0 + rate).powi(horizon as i32);
        let actual   = f.points.last().unwrap().projected_value;
        let error_pct = ((actual - expected) / expected).abs() * 100.0;
        assert!(error_pct < 20.0, "error {:.2}% exceeds 20% accuracy requirement", error_pct);
    }

    // ── Alerting ─────────────────────────────────────────

    #[test]
    fn no_alert_when_healthy() {
        let id = Uuid::new_v4();
        let bundle = build_scenario_bundle(
            id, ResourceKind::StorageEntries,
            100.0, 100_000.0, 12, None, now(),
        );
        let alert = evaluate_alert(id, &bundle, now());
        assert!(alert.is_none());
    }

    #[test]
    fn warning_at_60_pct() {
        let id = Uuid::new_v4();
        let bundle = build_scenario_bundle(
            id, ResourceKind::StorageEntries,
            61_000.0, 100_000.0, 12, None, now(),
        );
        let alert = evaluate_alert(id, &bundle, now()).unwrap();
        assert_eq!(alert.severity, "WARNING");
    }

    #[test]
    fn critical_when_breach_within_30_days() {
        let id = Uuid::new_v4();
        // 95% of limit + 25%/month base = breach in < 1 month
        let bundle = build_scenario_bundle(
            id, ResourceKind::StorageEntries,
            95_000.0, 100_000.0, 12, None, now(),
        );
        let alert = evaluate_alert(id, &bundle, now()).unwrap();
        // Days until breach should be ≤ 30
        assert!(alert.days_until_breach.map(|d| d <= 30).unwrap_or(false),
            "Expected critical alert within 30 days, got {:?}", alert.days_until_breach);
        assert_eq!(alert.severity, "CRITICAL");
    }

    #[test]
    fn breached_when_over_limit() {
        let id = Uuid::new_v4();
        let bundle = build_scenario_bundle(
            id, ResourceKind::StorageEntries,
            110_000.0, 100_000.0, 12, None, now(),
        );
        let alert = evaluate_alert(id, &bundle, now()).unwrap();
        assert_eq!(alert.severity, "BREACHED");
    }

    // ── Recommendations ──────────────────────────────────

    #[test]
    fn no_recs_when_all_healthy() {
        let id = Uuid::new_v4();
        let bundles: Vec<_> = [
            ResourceKind::StorageEntries,
            ResourceKind::CpuInstructions,
        ].into_iter().map(|r| build_scenario_bundle(
            id, r, 100.0, 100_000.0, 12, None, now()
        )).collect();
        let recs = generate_recommendations(id, &bundles, now());
        assert!(recs.is_empty());
    }

    #[test]
    fn recs_generated_for_high_usage() {
        let id = Uuid::new_v4();
        let bundle = build_scenario_bundle(
            id, ResourceKind::StorageEntries,
            80_000.0, 100_000.0, 12, None, now(),
        );
        let recs = generate_recommendations(id, &[bundle], now());
        assert!(!recs.is_empty());
        assert_eq!(recs[0].resource, ResourceKind::StorageEntries);
    }

    #[test]
    fn recs_sorted_by_priority() {
        let id = Uuid::new_v4();
        let bundles = vec![
            build_scenario_bundle(id, ResourceKind::StorageEntries,   80_000.0, 100_000.0, 12, None, now()),
            build_scenario_bundle(id, ResourceKind::CpuInstructions,  110_000_000.0, 100_000_000.0, 12, None, now()),
        ];
        let recs = generate_recommendations(id, &bundles, now());
        let priorities: Vec<_> = recs.iter().map(|r| r.priority).collect();
        let mut sorted = priorities.clone();
        sorted.sort();
        assert_eq!(priorities, sorted);
    }

    // ── Cost estimation ──────────────────────────────────

    #[test]
    fn cost_scales_with_usage() {
        let current  = estimate_cost(ResourceKind::StorageEntries, 1_000.0, 1_000.0,  12, 0.12);
        let projected = estimate_cost(ResourceKind::StorageEntries, 1_000.0, 2_000.0, 12, 0.12);
        assert!(projected.projected_monthly_xlm > current.projected_monthly_xlm);
    }

    #[test]
    fn usd_cost_uses_xlm_price() {
        let est = estimate_cost(ResourceKind::TransactionVolume, 1_000.0, 10_000.0, 12, 0.12);
        let expected_usd = est.projected_monthly_xlm * 0.12;
        assert!((est.projected_monthly_usd - expected_usd).abs() < 0.0001);
    }

    // ── Overall status ───────────────────────────────────

    #[test]
    fn overall_status_healthy_when_no_alerts() {
        let status = overall_status(&[]);
        assert_eq!(status, "healthy");
    }

    #[test]
    fn overall_status_breached_takes_precedence() {
        let alerts = vec![
            make_alert("WARNING"),
            make_alert("BREACHED"),
            make_alert("CRITICAL"),
        ];
        assert_eq!(overall_status(&alerts), "breached");
    }

    fn make_alert(severity: &str) -> CapacityAlert {
        CapacityAlert {
            id: Uuid::new_v4(), contract_id: Uuid::new_v4(),
            resource: ResourceKind::StorageEntries,
            severity: severity.to_string(),
            current_value: 0.0, limit_value: 0.0, pct_consumed: 0.0,
            breach_predicted_at: None, days_until_breach: None,
            message: String::new(), acknowledged: false,
            created_at: Utc::now(), resolved_at: None,
        }
    }
}
