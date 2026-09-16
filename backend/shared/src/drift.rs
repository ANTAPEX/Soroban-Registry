//! Three-way WASM drift detection module (Issue #1191)
//!
//! Provides core domain models, state persistence types, and drift evaluation logic
//! comparing the three legs of truth:
//! 1. Local lockfile (`soroban-registry.lock.json`)
//! 2. Registry catalog entry (`Contract.wasm_hash`)
//! 3. Stellar on-chain deployed state (`ContractCodeEntry`)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Status of contract WASM drift detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DriftStatus {
    /// All available sources (lockfile, registry, chain) have matching WASM hashes
    Match,
    /// A cryptographic mismatch exists between any of the legs
    Drift,
    /// The contract was not found deployed on the specified chain
    NotOnChain,
    /// An RPC, timeout, or network failure prevented verifying chain state.
    /// MUST NEVER be treated or reported as drift.
    Unknown,
}

impl DriftStatus {
    /// String representation matching database column CHECK constraint
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Drift => "drift",
            Self::NotOnChain => "not_on_chain",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for DriftStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DriftStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "match" => Ok(Self::Match),
            "drift" => Ok(Self::Drift),
            "not_on_chain" => Ok(Self::NotOnChain),
            "unknown" => Ok(Self::Unknown),
            other => Err(format!("Unknown drift status: '{}'", other)),
        }
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for DriftStatus {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse::<DriftStatus>()
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Send + Sync>)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for DriftStatus {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = self.as_str();
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

impl sqlx::Type<sqlx::Postgres> for DriftStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

/// Persisted database record for contract drift state
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ContractDriftRecord {
    pub id: Uuid,
    pub contract_id: String,
    pub network: String,
    pub registry_wasm_hash: String,
    pub onchain_wasm_hash: Option<String>,
    pub status: DriftStatus,
    pub diverged_leg: Option<String>,
    pub first_detected_at: DateTime<Utc>,
    pub last_checked_at: DateTime<Utc>,
    pub observed_at_ledger: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Machine-readable three-way drift state exposed via API and CLI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct ThreeWayDriftState {
    pub contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockfile_hash: Option<String>,
    pub registry_hash: String,
    pub onchain_hash: Option<String>,
    pub status: DriftStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diverged_leg: Option<String>,
    pub first_detected_at: DateTime<Utc>,
    pub last_checked_at: DateTime<Utc>,
    pub observed_at_ledger: Option<i64>,
}

/// Evaluation result containing evaluated status and metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftEvaluation {
    pub status: DriftStatus,
    pub diverged_leg: Option<String>,
    pub onchain_wasm_hash: Option<String>,
    pub first_detected_at: DateTime<Utc>,
    pub last_checked_at: DateTime<Utc>,
    pub observed_at_ledger: Option<i64>,
}

/// Evaluates three-way WASM drift across lockfile, registry, and on-chain state.
///
/// Invariants:
/// - RPC error / timeout degrades to `Unknown`, NEVER `Drift`.
/// - Contract not found on chain yields `NotOnChain`.
/// - Cryptographic hash divergence yields `Drift`.
/// - `first_detected_at` is preserved when continuing in `Drift` status.
/// - When recovering from `Drift` to `Match`, `first_detected_at` is reset to `now`.
pub fn evaluate_three_way_drift(
    registry_wasm_hash: &str,
    onchain_result: Result<Option<String>, &str>,
    lockfile_wasm_hash: Option<&str>,
    previous_record: Option<&ContractDriftRecord>,
    current_ledger: Option<i64>,
    now: DateTime<Utc>,
) -> DriftEvaluation {
    let last_checked_at = now;
    let observed_at_ledger = current_ledger.or_else(|| previous_record.and_then(|r| r.observed_at_ledger));

    match onchain_result {
        Err(_) => {
            // RPC or network failure — must be Unknown, NEVER drift
            let first_detected_at = previous_record
                .map(|r| r.first_detected_at)
                .unwrap_or(now);

            DriftEvaluation {
                status: DriftStatus::Unknown,
                diverged_leg: None,
                onchain_wasm_hash: previous_record.and_then(|r| r.onchain_wasm_hash.clone()),
                first_detected_at,
                last_checked_at,
                observed_at_ledger,
            }
        }
        Ok(None) => {
            // Contract not found on-chain
            let first_detected_at = match previous_record {
                Some(prev) if prev.status == DriftStatus::NotOnChain => prev.first_detected_at,
                _ => now,
            };

            DriftEvaluation {
                status: DriftStatus::NotOnChain,
                diverged_leg: Some("contract_not_on_chain".to_string()),
                onchain_wasm_hash: None,
                first_detected_at,
                last_checked_at,
                observed_at_ledger,
            }
        }
        Ok(Some(chain_hash)) => {
            let registry_matches_chain = registry_wasm_hash.eq_ignore_ascii_case(&chain_hash);

            let (status, diverged_leg) = match lockfile_wasm_hash {
                None => {
                    if registry_matches_chain {
                        (DriftStatus::Match, None)
                    } else {
                        (DriftStatus::Drift, Some("registry_vs_chain".to_string()))
                    }
                }
                Some(lock_hash) => {
                    let lock_matches_registry = lock_hash.eq_ignore_ascii_case(registry_wasm_hash);
                    let lock_matches_chain = lock_hash.eq_ignore_ascii_case(&chain_hash);

                    if registry_matches_chain && lock_matches_registry {
                        // All three legs match perfectly
                        (DriftStatus::Match, None)
                    } else if lock_matches_registry && !registry_matches_chain {
                        // Lockfile and registry agree, but chain upgraded/diverged
                        (DriftStatus::Drift, Some("registry_vs_chain".to_string()))
                    } else if !lock_matches_registry && registry_matches_chain {
                        // Registry and chain agree, but local lockfile is stale
                        (DriftStatus::Drift, Some("lockfile_vs_registry".to_string()))
                    } else if lock_matches_chain && !registry_matches_chain {
                        // Lockfile and chain agree, but registry is stale
                        (DriftStatus::Drift, Some("registry_vs_chain".to_string()))
                    } else {
                        // All three hashes differ
                        (DriftStatus::Drift, Some("three_way_divergence".to_string()))
                    }
                }
            };

            let first_detected_at = match status {
                DriftStatus::Drift => {
                    match previous_record {
                        Some(prev) if prev.status == DriftStatus::Drift => prev.first_detected_at,
                        _ => now,
                    }
                }
                DriftStatus::Match => now,
                DriftStatus::NotOnChain => {
                    match previous_record {
                        Some(prev) if prev.status == DriftStatus::NotOnChain => prev.first_detected_at,
                        _ => now,
                    }
                }
                DriftStatus::Unknown => {
                    previous_record.map(|r| r.first_detected_at).unwrap_or(now)
                }
            };

            DriftEvaluation {
                status,
                diverged_leg,
                onchain_wasm_hash: Some(chain_hash),
                first_detected_at,
                last_checked_at,
                observed_at_ledger,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_drift_status_serialization_and_parsing() {
        for (status, expected_str) in [
            (DriftStatus::Match, "match"),
            (DriftStatus::Drift, "drift"),
            (DriftStatus::NotOnChain, "not_on_chain"),
            (DriftStatus::Unknown, "unknown"),
        ] {
            assert_eq!(status.as_str(), expected_str);
            assert_eq!(status.to_string(), expected_str);
            assert_eq!(expected_str.parse::<DriftStatus>().unwrap(), status);

            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", expected_str));
            let deserialized: DriftStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_three_way_match() {
        let now = Utc::now();
        let hash = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";

        let eval = evaluate_three_way_drift(
            hash,
            Ok(Some(hash.to_string())),
            Some(hash),
            None,
            Some(58231900),
            now,
        );

        assert_eq!(eval.status, DriftStatus::Match);
        assert_eq!(eval.diverged_leg, None);
        assert_eq!(eval.onchain_wasm_hash.as_deref(), Some(hash));
        assert_eq!(eval.observed_at_ledger, Some(58231900));
    }

    #[test]
    fn test_hash_divergence_registry_vs_chain() {
        let now = Utc::now();
        let reg_hash = "9f2caaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let chain_hash = "41abaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        let eval = evaluate_three_way_drift(
            reg_hash,
            Ok(Some(chain_hash.to_string())),
            Some(reg_hash),
            None,
            Some(58231904),
            now,
        );

        assert_eq!(eval.status, DriftStatus::Drift);
        assert_eq!(eval.diverged_leg.as_deref(), Some("registry_vs_chain"));
        assert_eq!(eval.onchain_wasm_hash.as_deref(), Some(chain_hash));
    }

    #[test]
    fn test_hash_divergence_lockfile_vs_registry() {
        let now = Utc::now();
        let old_lock_hash = "1111111111111111111111111111111111111111111111111111111111111111";
        let current_hash = "2222222222222222222222222222222222222222222222222222222222222222";

        let eval = evaluate_three_way_drift(
            current_hash,
            Ok(Some(current_hash.to_string())),
            Some(old_lock_hash),
            None,
            Some(100),
            now,
        );

        assert_eq!(eval.status, DriftStatus::Drift);
        assert_eq!(eval.diverged_leg.as_deref(), Some("lockfile_vs_registry"));
    }

    #[test]
    fn test_contract_not_on_chain() {
        let now = Utc::now();
        let hash = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

        let eval = evaluate_three_way_drift(
            hash,
            Ok(None),
            Some(hash),
            None,
            Some(100),
            now,
        );

        assert_eq!(eval.status, DriftStatus::NotOnChain);
        assert_eq!(eval.diverged_leg.as_deref(), Some("contract_not_on_chain"));
        assert_eq!(eval.onchain_wasm_hash, None);
    }

    #[test]
    fn test_rpc_failure_yields_unknown_never_drift() {
        let now = Utc::now();
        let hash = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

        let eval = evaluate_three_way_drift(
            hash,
            Err("RPC connection timeout to horizon/soroban-rpc"),
            Some(hash),
            None,
            None,
            now,
        );

        assert_eq!(eval.status, DriftStatus::Unknown);
        assert_eq!(eval.diverged_leg, None);
        assert_ne!(eval.status, DriftStatus::Drift);
    }

    #[test]
    fn test_first_detected_at_stability_across_subsequent_scans() {
        let initial_time = Utc.with_ymd_and_hms(2026, 9, 2, 11, 4, 0).unwrap();
        let subsequent_time = Utc.with_ymd_and_hms(2026, 9, 14, 8, 15, 0).unwrap();

        let initial_record = ContractDriftRecord {
            id: Uuid::new_v4(),
            contract_id: "CB11111111111111111111111111111111111111111111111111111111".to_string(),
            network: "testnet".to_string(),
            registry_wasm_hash: "9f2caaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            onchain_wasm_hash: Some("41abaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            status: DriftStatus::Drift,
            diverged_leg: Some("registry_vs_chain".to_string()),
            first_detected_at: initial_time,
            last_checked_at: initial_time,
            observed_at_ledger: Some(58000000),
            created_at: initial_time,
            updated_at: initial_time,
        };

        // Subsequent scan 12 days later still detecting drift
        let eval = evaluate_three_way_drift(
            &initial_record.registry_wasm_hash,
            Ok(Some("41abaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())),
            Some(&initial_record.registry_wasm_hash),
            Some(&initial_record),
            Some(58231904),
            subsequent_time,
        );

        assert_eq!(eval.status, DriftStatus::Drift);
        // CRITICAL ACCEPTANCE CRITERIA: first_detected_at must be PRESERVED
        assert_eq!(eval.first_detected_at, initial_time);
        assert_eq!(eval.last_checked_at, subsequent_time);
        assert_eq!(eval.observed_at_ledger, Some(58231904));
    }

    #[test]
    fn test_drifted_contract_restored_to_matching() {
        let initial_time = Utc.with_ymd_and_hms(2026, 9, 2, 11, 4, 0).unwrap();
        let restore_time = Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap();
        let matching_hash = "9f2caaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        let drifted_record = ContractDriftRecord {
            id: Uuid::new_v4(),
            contract_id: "CB11111111111111111111111111111111111111111111111111111111".to_string(),
            network: "testnet".to_string(),
            registry_wasm_hash: matching_hash.to_string(),
            onchain_wasm_hash: Some("41abaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            status: DriftStatus::Drift,
            diverged_leg: Some("registry_vs_chain".to_string()),
            first_detected_at: initial_time,
            last_checked_at: initial_time,
            observed_at_ledger: Some(58000000),
            created_at: initial_time,
            updated_at: initial_time,
        };

        // Registry entry or on-chain upgraded back to matching
        let eval = evaluate_three_way_drift(
            matching_hash,
            Ok(Some(matching_hash.to_string())),
            Some(matching_hash),
            Some(&drifted_record),
            Some(58300000),
            restore_time,
        );

        assert_eq!(eval.status, DriftStatus::Match);
        assert_eq!(eval.diverged_leg, None);
        assert_eq!(eval.first_detected_at, restore_time);
        assert_eq!(eval.last_checked_at, restore_time);
    }

    #[test]
    fn test_three_way_drift_state_json_matches_issue_format() {
        let first_detected = Utc.with_ymd_and_hms(2026, 9, 2, 11, 4, 0).unwrap();
        let last_checked = Utc.with_ymd_and_hms(2026, 9, 14, 8, 15, 0).unwrap();

        let state = ThreeWayDriftState {
            contract_id: "CB...".to_string(),
            lockfile_hash: Some("9f2c...".to_string()),
            registry_hash: "9f2c...".to_string(),
            onchain_hash: Some("41ab...".to_string()),
            status: DriftStatus::Drift,
            diverged_leg: Some("registry_vs_chain".to_string()),
            first_detected_at: first_detected,
            last_checked_at: last_checked,
            observed_at_ledger: Some(58231904),
        };

        let json_value = serde_json::to_value(&state).unwrap();
        assert_eq!(json_value["contract_id"], "CB...");
        assert_eq!(json_value["lockfile_hash"], "9f2c...");
        assert_eq!(json_value["registry_hash"], "9f2c...");
        assert_eq!(json_value["onchain_hash"], "41ab...");
        assert_eq!(json_value["status"], "drift");
        assert_eq!(json_value["diverged_leg"], "registry_vs_chain");
        assert_eq!(json_value["first_detected_at"], "2026-09-02T11:04:00Z");
        assert_eq!(json_value["last_checked_at"], "2026-09-14T08:15:00Z");
        assert_eq!(json_value["observed_at_ledger"], 58231904);
    }
}
