#![allow(dead_code)]

use std::collections::HashMap;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaMetrics {
    pub contract_id: String,
    pub uptime_percentage: f64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaViolation {
    pub metric: String,
    pub actual: f64,
    pub target: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaStatus {
    pub contract_id: String,
    pub total_records: usize,
    pub violations: Vec<SlaViolation>,
    pub penalty_accrued: f64,
    pub credits_issued: f64,
    pub compliant: bool,
}

#[derive(Debug, Clone)]
pub struct SlaTargets {
    pub min_uptime: f64,
    pub max_latency_ms: f64,
    pub max_error_rate: f64,
}

impl Default for SlaTargets {
    fn default() -> Self {
        Self {
            min_uptime: 99.5,
            max_latency_ms: 200.0,
            max_error_rate: 1.0,
        }
    }
}

const PENALTY_PER_VIOLATION: f64 = 50.0;
const CREDIT_RATE: f64 = 0.10;

pub struct SlaManager {
    pub targets: SlaTargets,
    records: HashMap<String, Vec<SlaMetrics>>,
    penalties: HashMap<String, f64>,
    credits: HashMap<String, f64>,
}

impl SlaManager {
    pub fn new() -> Self {
        Self {
            targets: SlaTargets::default(),
            records: HashMap::new(),
            penalties: HashMap::new(),
            credits: HashMap::new(),
        }
    }

    pub fn record(&mut self, contract_id: &str, uptime: f64, latency: f64, error_rate: f64) {
        let m = SlaMetrics {
            contract_id: contract_id.to_string(),
            uptime_percentage: uptime,
            avg_latency_ms: latency,
            error_rate,
            recorded_at: Utc::now(),
        };
        self.records
            .entry(contract_id.to_string())
            .or_default()
            .push(m);
    }

    pub fn check_violations(metrics: &SlaMetrics, targets: &SlaTargets) -> Vec<SlaViolation> {
        let mut v = Vec::new();
        if metrics.uptime_percentage < targets.min_uptime {
            v.push(SlaViolation {
                metric: "uptime".into(),
                actual: metrics.uptime_percentage,
                target: targets.min_uptime,
            });
        }
        if metrics.avg_latency_ms > targets.max_latency_ms {
            v.push(SlaViolation {
                metric: "latency".into(),
                actual: metrics.avg_latency_ms,
                target: targets.max_latency_ms,
            });
        }
        if metrics.error_rate > targets.max_error_rate {
            v.push(SlaViolation {
                metric: "error_rate".into(),
                actual: metrics.error_rate,
                target: targets.max_error_rate,
            });
        }
        v
    }

    pub fn evaluate(&mut self, contract_id: &str) -> Result<SlaStatus> {
        let history = match self.records.get(contract_id) {
            Some(h) if !h.is_empty() => h,
            _ => bail!("no SLA records found for contract: {}", contract_id),
        };

        let latest = history.last().unwrap();
        let violations = Self::check_violations(latest, &self.targets);
        let penalty_increment = violations.len() as f64 * PENALTY_PER_VIOLATION;

        let accrued = self.penalties.entry(contract_id.to_string()).or_insert(0.0);
        *accrued += penalty_increment;

        let credit_entry = self.credits.entry(contract_id.to_string()).or_insert(0.0);
        if violations.is_empty() && *accrued > 0.0 {
            let credit = *accrued * CREDIT_RATE;
            *credit_entry += credit;
        }

        Ok(SlaStatus {
            contract_id: contract_id.to_string(),
            total_records: history.len(),
            compliant: violations.is_empty(),
            violations,
            penalty_accrued: *self.penalties.get(contract_id).unwrap_or(&0.0),
            credits_issued: *self.credits.get(contract_id).unwrap_or(&0.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliant_metrics() {
        let mut mgr = SlaManager::new();
        mgr.record("c1", 99.9, 150.0, 0.5);
        let status = mgr.evaluate("c1").unwrap();
        assert!(status.compliant);
        assert!(status.violations.is_empty());
        assert_eq!(status.penalty_accrued, 0.0);
        assert_eq!(status.credits_issued, 0.0);
    }

    #[test]
    fn test_single_violation() {
        let mut mgr = SlaManager::new();
        mgr.record("c2", 98.0, 150.0, 0.5);
        let status = mgr.evaluate("c2").unwrap();
        assert!(!status.compliant);
        assert_eq!(status.violations.len(), 1);
        assert_eq!(status.violations[0].metric, "uptime");
        assert_eq!(status.penalty_accrued, 50.0);
    }

    #[test]
    fn test_multiple_violations() {
        let mut mgr = SlaManager::new();
        mgr.record("c3", 90.0, 500.0, 5.0);
        let status = mgr.evaluate("c3").unwrap();
        assert!(!status.compliant);
        assert_eq!(status.violations.len(), 3);
        assert_eq!(status.penalty_accrued, 150.0);
    }

    #[test]
    fn test_penalty_accrual_across_records() {
        let mut mgr = SlaManager::new();
        mgr.record("c4", 98.0, 150.0, 0.5);
        let s1 = mgr.evaluate("c4").unwrap();
        assert_eq!(s1.penalty_accrued, 50.0);

        mgr.record("c4", 97.0, 150.0, 0.5);
        let s2 = mgr.evaluate("c4").unwrap();
        assert_eq!(s2.penalty_accrued, 100.0);
    }

    #[test]
    fn test_credit_issuance() {
        let mut mgr = SlaManager::new();
        mgr.record("c5", 98.0, 150.0, 0.5);
        mgr.evaluate("c5").unwrap();

        mgr.record("c5", 99.9, 100.0, 0.1);
        let status = mgr.evaluate("c5").unwrap();
        assert!(status.compliant);
        assert_eq!(status.penalty_accrued, 50.0);
        assert_eq!(status.credits_issued, 5.0);
    }
}
