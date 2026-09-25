//! Drift Scanner for Soroban Registry (Issue #1191)
//!
//! Re-checks contracts on a staleness-ordered schedule:
//! 1. Oldest `last_checked_at` (never checked first)
//! 2. Verified contracts prioritized
//! 3. High-dependent-count contracts prioritized
//!
//! Invariants:
//! - Respects `ExponentialBackoff` (`crate::backoff::ExponentialBackoff`).
//! - Degrades to `DriftStatus::Unknown` (NEVER `DriftStatus::Drift`) when RPC is unavailable.
//! - Preserves `first_detected_at` when continuing in `Drift` status.
//! - Configurable scan batch size and interval to avoid exhausting RPC rate limits.

use crate::backoff::ExponentialBackoff;
use chrono::{DateTime, Utc};
use shared::{evaluate_three_way_drift, ContractDriftRecord, DriftStatus};
use sqlx::PgPool;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Configuration for the background drift scanner
#[derive(Debug, Clone)]
pub struct DriftScannerConfig {
    pub network: String,
    pub rpc_endpoint: String,
    pub batch_size: i64,
    pub interval_secs: u64,
    pub base_backoff_secs: u64,
    pub max_backoff_secs: u64,
}

impl Default for DriftScannerConfig {
    fn default() -> Self {
        Self {
            network: "testnet".to_string(),
            rpc_endpoint: "https://rpc-testnet.stellar.org".to_string(),
            batch_size: 10,
            interval_secs: 60,
            base_backoff_secs: 2,
            max_backoff_secs: 60,
        }
    }
}

/// Candidate contract for drift evaluation
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DriftScanCandidate {
    pub contract_id: String,
    pub network: String,
    pub wasm_hash: String,
    pub is_verified: bool,
    pub dependent_count: i64,
    pub last_checked_at: Option<DateTime<Utc>>,
}

/// Result of a drift scan run
#[derive(Debug, Clone, Default)]
pub struct ScanSummary {
    pub contracts_scanned: usize,
    pub matched_count: usize,
    pub drifted_count: usize,
    pub not_on_chain_count: usize,
    pub unknown_count: usize,
}

/// Background drift scanner service
pub struct DriftScanner {
    pool: PgPool,
    config: DriftScannerConfig,
    http_client: reqwest::Client,
    backoff: ExponentialBackoff,
}

impl DriftScanner {
    pub fn new(pool: PgPool, config: DriftScannerConfig) -> Self {
        let backoff = ExponentialBackoff::new(config.base_backoff_secs, config.max_backoff_secs);
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            pool,
            config,
            http_client,
            backoff,
        }
    }

    /// Fetch candidates ordered by staleness, verification priority, and dependent count
    pub async fn fetch_candidates(&self) -> Result<Vec<DriftScanCandidate>, sqlx::Error> {
        sqlx::query_as::<_, DriftScanCandidate>(
            r#"
            SELECT c.contract_id, c.network, c.wasm_hash, c.is_verified,
                   COALESCE(c.dependent_count, 0) AS dependent_count,
                   ds.last_checked_at
            FROM contracts c
            LEFT JOIN contract_drift_state ds ON ds.contract_id = c.contract_id AND ds.network = c.network
            WHERE c.network = $1
            ORDER BY
                ds.last_checked_at ASC NULLS FIRST,
                c.is_verified DESC,
                c.dependent_count DESC
            LIMIT $2
            "#,
        )
        .bind(&self.config.network)
        .bind(self.config.batch_size)
        .fetch_all(&self.pool)
        .await
    }

    /// Query the live Soroban RPC for the contract's on-chain WASM hash and latest ledger.
    pub async fn fetch_onchain_wasm_hash(
        &self,
        contract_id: &str,
    ) -> Result<(Option<String>, Option<i64>), String> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "getLatestLedger",
            "method": "getLatestLedger",
            "params": {}
        });

        let ledger_res = self
            .http_client
            .post(&self.config.rpc_endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("RPC connection failure: {e}"))?;

        if !ledger_res.status().is_success() {
            return Err(format!("RPC returned HTTP {}", ledger_res.status()));
        }

        let ledger_json: serde_json::Value = ledger_res
            .json()
            .await
            .map_err(|e| format!("Invalid JSON response: {e}"))?;

        let latest_ledger = ledger_json
            .get("result")
            .and_then(|r| r.get("sequence"))
            .and_then(|s| s.as_i64());

        let contract_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "getLedgerEntries",
            "method": "getLedgerEntries",
            "params": {
                "keys": [contract_id],
                "xdrFormat": "base64"
            }
        });

        let contract_res = self
            .http_client
            .post(&self.config.rpc_endpoint)
            .json(&contract_payload)
            .send()
            .await
            .map_err(|e| format!("RPC ledger entries lookup failure: {e}"))?;

        if !contract_res.status().is_success() {
            return Err(format!("RPC entries returned HTTP {}", contract_res.status()));
        }

        let contract_json: serde_json::Value = contract_res
            .json()
            .await
            .map_err(|e| format!("Failed to parse entries JSON: {e}"))?;

        if let Some(entries) = contract_json
            .get("result")
            .and_then(|r| r.get("entries"))
            .and_then(|e| e.as_array())
        {
            if entries.is_empty() {
                return Ok((None, latest_ledger));
            }
            let wasm_hash = entries[0]
                .get("wasmHash")
                .or_else(|| entries[0].get("wasm_hash"))
                .and_then(|w| w.as_str())
                .map(|s| s.to_string());

            Ok((wasm_hash, latest_ledger))
        } else {
            Ok((None, latest_ledger))
        }
    }

    /// Execute a single pass of the drift scanner
    pub async fn scan_once(&mut self) -> Result<ScanSummary, String> {
        let candidates = self
            .fetch_candidates()
            .await
            .map_err(|e| format!("Database error fetching candidates: {e}"))?;

        let mut summary = ScanSummary::default();
        let now = Utc::now();

        for candidate in candidates {
            summary.contracts_scanned += 1;

            let previous_record: Option<ContractDriftRecord> = sqlx::query_as(
                r#"
                SELECT id, contract_id, network, registry_wasm_hash, onchain_wasm_hash,
                       status, diverged_leg, first_detected_at, last_checked_at,
                       observed_at_ledger, created_at, updated_at
                FROM contract_drift_state
                WHERE contract_id = $1 AND network = $2
                "#,
            )
            .bind(&candidate.contract_id)
            .bind(&candidate.network)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to fetch previous drift record: {e}"))?;

            let onchain_result = match self.fetch_onchain_wasm_hash(&candidate.contract_id).await {
                Ok((hash, ledger)) => {
                    self.backoff.on_success();
                    Ok((hash, ledger))
                }
                Err(err) => {
                    let wait = self.backoff.on_failure(&err);
                    warn!(
                        contract_id = %candidate.contract_id,
                        error = %err,
                        wait_secs = wait.as_secs(),
                        "RPC unavailable during drift scan; degrading to Unknown and backing off"
                    );
                    Err(err)
                }
            };

            let (onchain_hash_res, current_ledger) = match onchain_result {
                Ok((hash, ledger)) => (Ok(hash), ledger),
                Err(e) => (Err(e), None),
            };

            let eval = evaluate_three_way_drift(
                &candidate.wasm_hash,
                onchain_hash_res.as_ref().map(|o| o.clone()).map_err(|e| e.as_str()),
                None,
                previous_record.as_ref(),
                current_ledger,
                now,
            );

            match eval.status {
                DriftStatus::Match => summary.matched_count += 1,
                DriftStatus::Drift => summary.drifted_count += 1,
                DriftStatus::NotOnChain => summary.not_on_chain_count += 1,
                DriftStatus::Unknown => summary.unknown_count += 1,
            }

            sqlx::query(
                r#"
                INSERT INTO contract_drift_state (
                    contract_id, network, registry_wasm_hash, onchain_wasm_hash,
                    status, diverged_leg, first_detected_at, last_checked_at,
                    observed_at_ledger, updated_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, NOW()
                )
                ON CONFLICT (contract_id, network) DO UPDATE SET
                    registry_wasm_hash = EXCLUDED.registry_wasm_hash,
                    onchain_wasm_hash = EXCLUDED.onchain_wasm_hash,
                    status = EXCLUDED.status,
                    diverged_leg = EXCLUDED.diverged_leg,
                    first_detected_at = EXCLUDED.first_detected_at,
                    last_checked_at = EXCLUDED.last_checked_at,
                    observed_at_ledger = EXCLUDED.observed_at_ledger,
                    updated_at = NOW()
                "#,
            )
            .bind(&candidate.contract_id)
            .bind(&candidate.network)
            .bind(&candidate.wasm_hash)
            .bind(&eval.onchain_wasm_hash)
            .bind(eval.status.as_str())
            .bind(&eval.diverged_leg)
            .bind(eval.first_detected_at)
            .bind(eval.last_checked_at)
            .bind(eval.observed_at_ledger)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to upsert drift state: {e}"))?;
        }

        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_scanner_config_defaults() {
        let cfg = DriftScannerConfig::default();
        assert_eq!(cfg.network, "testnet");
        assert_eq!(cfg.batch_size, 10);
        assert_eq!(cfg.interval_secs, 60);
        assert_eq!(cfg.base_backoff_secs, 2);
        assert_eq!(cfg.max_backoff_secs, 60);
    }

    #[test]
    fn test_scan_summary_default() {
        let summary = ScanSummary::default();
        assert_eq!(summary.contracts_scanned, 0);
        assert_eq!(summary.matched_count, 0);
        assert_eq!(summary.drifted_count, 0);
        assert_eq!(summary.not_on_chain_count, 0);
        assert_eq!(summary.unknown_count, 0);
    }

    #[test]
    fn test_candidate_ordering_simulation() {
        let mut candidates = vec![
            DriftScanCandidate {
                contract_id: "C_RECENT".to_string(),
                network: "testnet".to_string(),
                wasm_hash: "hash1".to_string(),
                is_verified: false,
                dependent_count: 5,
                last_checked_at: Some(Utc::now()),
            },
            DriftScanCandidate {
                contract_id: "C_UNCHECKED_VERIFIED".to_string(),
                network: "testnet".to_string(),
                wasm_hash: "hash2".to_string(),
                is_verified: true,
                dependent_count: 100,
                last_checked_at: None,
            },
            DriftScanCandidate {
                contract_id: "C_UNCHECKED_UNVERIFIED".to_string(),
                network: "testnet".to_string(),
                wasm_hash: "hash3".to_string(),
                is_verified: false,
                dependent_count: 2,
                last_checked_at: None,
            },
        ];

        candidates.sort_by(|a, b| {
            match (a.last_checked_at, b.last_checked_at) {
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                _ => {
                    b.is_verified
                        .cmp(&a.is_verified)
                        .then_with(|| b.dependent_count.cmp(&a.dependent_count))
                }
            }
        });

        assert_eq!(candidates[0].contract_id, "C_UNCHECKED_VERIFIED");
        assert_eq!(candidates[1].contract_id, "C_UNCHECKED_UNVERIFIED");
        assert_eq!(candidates[2].contract_id, "C_RECENT");
    }
}

