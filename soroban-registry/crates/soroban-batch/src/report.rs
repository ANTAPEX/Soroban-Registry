use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::rollback::RollbackResult;

/// Outcome of a single operation within the batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationOutcome {
    pub index: usize,
    pub label: String,
    pub success: bool,
    pub duration_ms: u128,
    pub error: Option<String>,
}

/// Full report produced after a batch execution attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReport {
    pub manifest_name: Option<String>,
    pub total_operations: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub rolled_back: bool,
    pub rollback_results: Vec<RollbackResult>,
    pub outcomes: Vec<OperationOutcome>,
    pub total_duration_ms: u128,
}

impl BatchReport {
    pub fn new(manifest_name: Option<String>, total_operations: usize) -> Self {
        Self {
            manifest_name,
            total_operations,
            succeeded: 0,
            failed: 0,
            rolled_back: false,
            rollback_results: Vec::new(),
            outcomes: Vec::new(),
            total_duration_ms: 0,
        }
    }

    pub fn record_success(&mut self, index: usize, label: String, duration: Duration) {
        self.succeeded += 1;
        self.outcomes.push(OperationOutcome {
            index,
            label,
            success: true,
            duration_ms: duration.as_millis(),
            error: None,
        });
    }

    pub fn record_failure(
        &mut self,
        index: usize,
        label: String,
        duration: Duration,
        error: String,
    ) {
        self.failed += 1;
        self.outcomes.push(OperationOutcome {
            index,
            label,
            success: false,
            duration_ms: duration.as_millis(),
            error: Some(error),
        });
    }

    pub fn set_total_duration(&mut self, duration: Duration) {
        self.total_duration_ms = duration.as_millis();
    }

    pub fn set_rollback(&mut self, results: Vec<RollbackResult>) {
        self.rolled_back = true;
        self.rollback_results = results;
    }

    /// Pretty-print the report to stdout.
    pub fn print_human(&self) {
        println!("═══════════════════════════════════════════");
        println!("  BATCH EXECUTION REPORT");
        if let Some(name) = &self.manifest_name {
            println!("  Manifest: {}", name);
        }
        println!("═══════════════════════════════════════════");
        println!();

        for outcome in &self.outcomes {
            let status = if outcome.success { "✅" } else { "❌" };
            println!(
                "  {} [{:>4}ms] {}",
                status, outcome.duration_ms, outcome.label
            );
            if let Some(err) = &outcome.error {
                println!("              └─ {}", err);
            }
        }

        println!();
        println!("───────────────────────────────────────────");
        println!(
            "  Total: {}  |  Passed: {}  |  Failed: {}",
            self.total_operations, self.succeeded, self.failed
        );
        println!("  Duration: {}ms", self.total_duration_ms);

        if self.rolled_back {
            println!();
            println!("  ⚠️  ROLLBACK EXECUTED");
            for rb in &self.rollback_results {
                let status = if rb.success { "✅" } else { "❌" };
                println!("    {} {}", status, rb.action_description);
                if let Some(err) = &rb.error {
                    println!("       └─ {}", err);
                }
            }
        }

        println!("═══════════════════════════════════════════");
    }

    /// Return the report as a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}