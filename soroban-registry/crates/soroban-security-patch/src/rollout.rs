use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{RolloutPlan, RolloutStage, SecurityPatchError};

// ---------------------------------------------------------------------------
// Rollout state
// ---------------------------------------------------------------------------

/// Tracks the progress of a staged rollout for a single patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutState {
    /// Patch ID this rollout belongs to.
    pub patch_id: String,
    /// The rollout plan / configuration.
    pub plan: RolloutPlan,
    /// Current stage of the rollout.
    pub current_stage: RolloutStage,
    /// Contracts assigned to each stage.
    pub stage_assignments: StageAssignments,
    /// Per-contract application results.
    pub results: Vec<ContractRolloutResult>,
    /// Whether the rollout has been paused (awaiting manual approval).
    pub paused: bool,
    /// Timestamp when the rollout started.
    pub started_at: DateTime<Utc>,
    /// Timestamp when the current stage started.
    pub stage_started_at: DateTime<Utc>,
    /// Whether the rollout has been completed or rolled back.
    pub completed: bool,
}

/// Assignment of contracts to each rollout stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageAssignments {
    pub canary: Vec<String>,
    pub early_adopter: Vec<String>,
    pub general_availability: Vec<String>,
}

/// Result of applying a patch to a single contract during rollout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRolloutResult {
    pub contract_id: String,
    pub stage: RolloutStage,
    pub success: bool,
    pub error: Option<String>,
    pub applied_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Rollout engine
// ---------------------------------------------------------------------------

/// Orchestrates staged rollouts for security patches.
#[derive(Debug, Default)]
pub struct RolloutEngine {
    rollouts: Vec<RolloutState>,
}

impl RolloutEngine {
    pub fn new() -> Self {
        Self {
            rollouts: Vec::new(),
        }
    }

    /// Start a staged rollout for a given patch.
    ///
    /// The `affected_contracts` list is partitioned into stages according to
    /// the percentages defined in `plan`.
    pub fn start_rollout(
        &mut self,
        patch_id: &str,
        affected_contracts: &[String],
        plan: RolloutPlan,
    ) -> Result<&RolloutState, SecurityPatchError> {
        if affected_contracts.is_empty() {
            return Err(SecurityPatchError::NoVulnerableContracts(
                patch_id.to_string(),
            ));
        }

        let assignments = Self::partition_contracts(affected_contracts, &plan);
        let now = Utc::now();

        let state = RolloutState {
            patch_id: patch_id.to_string(),
            plan,
            current_stage: RolloutStage::Canary,
            stage_assignments: assignments,
            results: Vec::new(),
            paused: false,
            started_at: now,
            stage_started_at: now,
            completed: false,
        };

        self.rollouts.push(state);
        Ok(self.rollouts.last().expect("just pushed"))
    }

    /// Execute the current stage for a given rollout.
    ///
    /// In production, this would actually deploy the patch; here we simulate
    /// success for each contract in the current stage.
    pub fn execute_current_stage(
        &mut self,
        patch_id: &str,
    ) -> Result<Vec<ContractRolloutResult>, SecurityPatchError> {
        let state = self.get_rollout_mut(patch_id)?;

        if state.paused {
            return Err(SecurityPatchError::RolloutFailed {
                stage: state.current_stage,
                reason: "Rollout is paused — manual approval required".into(),
            });
        }

        if state.completed {
            return Err(SecurityPatchError::RolloutFailed {
                stage: state.current_stage,
                reason: "Rollout is already completed".into(),
            });
        }

        let contracts = match state.current_stage {
            RolloutStage::Canary => state.stage_assignments.canary.clone(),
            RolloutStage::EarlyAdopter => state.stage_assignments.early_adopter.clone(),
            RolloutStage::GeneralAvailability => {
                state.stage_assignments.general_availability.clone()
            }
        };

        let now = Utc::now();
        let mut stage_results = Vec::new();

        for contract_id in &contracts {
            // Simulate application — in production this would be an actual deploy.
            let result = ContractRolloutResult {
                contract_id: contract_id.clone(),
                stage: state.current_stage,
                success: true,
                error: None,
                applied_at: now,
            };
            stage_results.push(result);
        }

        state.results.extend(stage_results.clone());

        Ok(stage_results)
    }

    /// Advance the rollout to the next stage.
    pub fn advance_stage(&mut self, patch_id: &str) -> Result<RolloutStage, SecurityPatchError> {
        let state = self.get_rollout_mut(patch_id)?;

        // Check failure rate of current stage before advancing.
        let current_results: Vec<_> = state
            .results
            .iter()
            .filter(|r| r.stage == state.current_stage)
            .collect();

        if current_results.is_empty() {
            return Err(SecurityPatchError::RolloutFailed {
                stage: state.current_stage,
                reason: "Current stage has not been executed yet".into(),
            });
        }

        let failures = current_results.iter().filter(|r| !r.success).count();
        let failure_rate = failures as f64 / current_results.len() as f64;

        if failure_rate > state.plan.max_failure_rate {
            return Err(SecurityPatchError::RolloutFailed {
                stage: state.current_stage,
                reason: format!(
                    "Failure rate {:.2}% exceeds max {:.2}%",
                    failure_rate * 100.0,
                    state.plan.max_failure_rate * 100.0
                ),
            });
        }

        let next_stage = match state.current_stage {
            RolloutStage::Canary => {
                state.current_stage = RolloutStage::EarlyAdopter;
                if state.plan.require_approval {
                    state.paused = true;
                }
                RolloutStage::EarlyAdopter
            }
            RolloutStage::EarlyAdopter => {
                state.current_stage = RolloutStage::GeneralAvailability;
                if state.plan.require_approval {
                    state.paused = true;
                }
                RolloutStage::GeneralAvailability
            }
            RolloutStage::GeneralAvailability => {
                state.completed = true;
                RolloutStage::GeneralAvailability
            }
        };

        state.stage_started_at = Utc::now();
        Ok(next_stage)
    }

    /// Manually approve advancement (un-pause).
    pub fn approve_stage(&mut self, patch_id: &str) -> Result<(), SecurityPatchError> {
        let state = self.get_rollout_mut(patch_id)?;
        state.paused = false;
        Ok(())
    }

    /// Roll back an in-progress rollout.
    pub fn rollback(&mut self, patch_id: &str) -> Result<(), SecurityPatchError> {
        let state = self.get_rollout_mut(patch_id)?;
        state.completed = true;
        state.paused = false;
        Ok(())
    }

    /// Get the current rollout state for a patch.
    pub fn get_rollout(&self, patch_id: &str) -> Result<&RolloutState, SecurityPatchError> {
        self.rollouts
            .iter()
            .find(|r| r.patch_id == patch_id)
            .ok_or_else(|| SecurityPatchError::PatchNotFound(patch_id.to_string()))
    }

    /// Get rollout progress as a percentage.
    pub fn rollout_progress(&self, patch_id: &str) -> Result<f64, SecurityPatchError> {
        let state = self.get_rollout(patch_id)?;
        let total = state.stage_assignments.canary.len()
            + state.stage_assignments.early_adopter.len()
            + state.stage_assignments.general_availability.len();

        if total == 0 {
            return Ok(100.0);
        }

        let applied = state.results.iter().filter(|r| r.success).count();
        Ok((applied as f64 / total as f64) * 100.0)
    }

    /// Total number of active rollouts.
    pub fn count(&self) -> usize {
        self.rollouts.len()
    }

    // ----- Private helpers -------------------------------------------------

    fn get_rollout_mut(&mut self, patch_id: &str) -> Result<&mut RolloutState, SecurityPatchError> {
        self.rollouts
            .iter_mut()
            .find(|r| r.patch_id == patch_id)
            .ok_or_else(|| SecurityPatchError::PatchNotFound(patch_id.to_string()))
    }

    /// Partition contracts into rollout stages based on the plan's percentages.
    fn partition_contracts(contracts: &[String], plan: &RolloutPlan) -> StageAssignments {
        let total = contracts.len();
        let canary_count =
            ((total as f64) * (plan.canary_percentage as f64 / 100.0)).ceil() as usize;
        let early_count =
            ((total as f64) * (plan.early_adopter_percentage as f64 / 100.0)).ceil() as usize;

        // Clamp so we don't exceed total.
        let canary_count = canary_count.min(total);
        let early_count = early_count.min(total.saturating_sub(canary_count));
        let ga_start = canary_count + early_count;

        StageAssignments {
            canary: contracts[..canary_count].to_vec(),
            early_adopter: contracts[canary_count..canary_count + early_count].to_vec(),
            general_availability: contracts[ga_start..].to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contracts(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("CONTRACT_{i}")).collect()
    }

    fn permissive_plan() -> RolloutPlan {
        RolloutPlan {
            canary_percentage: 10,
            early_adopter_percentage: 30,
            soak_time_secs: 60,
            max_failure_rate: 0.5,
            require_approval: false,
        }
    }

    #[test]
    fn test_start_rollout() {
        let mut engine = RolloutEngine::new();
        let contracts = sample_contracts(20);
        let plan = permissive_plan();

        let state = engine.start_rollout("patch-1", &contracts, plan).unwrap();
        assert_eq!(state.current_stage, RolloutStage::Canary);
        assert!(!state.completed);
        // 10% of 20 = 2 canary contracts
        assert_eq!(state.stage_assignments.canary.len(), 2);
    }

    #[test]
    fn test_start_rollout_empty_contracts_fails() {
        let mut engine = RolloutEngine::new();
        let err = engine.start_rollout("p1", &[], permissive_plan());
        assert!(err.is_err());
    }

    #[test]
    fn test_execute_and_advance() {
        let mut engine = RolloutEngine::new();
        let contracts = sample_contracts(10);
        engine
            .start_rollout("p1", &contracts, permissive_plan())
            .unwrap();

        // Execute canary stage
        let results = engine.execute_current_stage("p1").unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.success));

        // Advance to early adopter
        let next = engine.advance_stage("p1").unwrap();
        assert_eq!(next, RolloutStage::EarlyAdopter);
    }

    #[test]
    fn test_advance_without_execute_fails() {
        let mut engine = RolloutEngine::new();
        let contracts = sample_contracts(5);
        engine
            .start_rollout("p1", &contracts, permissive_plan())
            .unwrap();

        let err = engine.advance_stage("p1");
        assert!(err.is_err());
    }

    #[test]
    fn test_full_rollout_cycle() {
        let mut engine = RolloutEngine::new();
        let contracts = sample_contracts(10);
        engine
            .start_rollout("p1", &contracts, permissive_plan())
            .unwrap();

        // Canary
        engine.execute_current_stage("p1").unwrap();
        engine.advance_stage("p1").unwrap();

        // Early adopter
        engine.execute_current_stage("p1").unwrap();
        engine.advance_stage("p1").unwrap();

        // GA
        engine.execute_current_stage("p1").unwrap();
        engine.advance_stage("p1").unwrap(); // completes

        let state = engine.get_rollout("p1").unwrap();
        assert!(state.completed);
    }

    #[test]
    fn test_rollout_progress() {
        let mut engine = RolloutEngine::new();
        let contracts = sample_contracts(10);
        engine
            .start_rollout("p1", &contracts, permissive_plan())
            .unwrap();

        // Before any execution
        let progress = engine.rollout_progress("p1").unwrap();
        assert!((progress - 0.0).abs() < f64::EPSILON);

        // After canary
        engine.execute_current_stage("p1").unwrap();
        let progress = engine.rollout_progress("p1").unwrap();
        assert!(progress > 0.0);
    }

    #[test]
    fn test_rollback() {
        let mut engine = RolloutEngine::new();
        let contracts = sample_contracts(5);
        engine
            .start_rollout("p1", &contracts, permissive_plan())
            .unwrap();

        engine.execute_current_stage("p1").unwrap();
        engine.rollback("p1").unwrap();

        let state = engine.get_rollout("p1").unwrap();
        assert!(state.completed);
    }

    #[test]
    fn test_paused_rollout_with_approval() {
        let mut engine = RolloutEngine::new();
        let contracts = sample_contracts(10);
        let plan = RolloutPlan {
            require_approval: true,
            ..permissive_plan()
        };
        engine.start_rollout("p1", &contracts, plan).unwrap();

        engine.execute_current_stage("p1").unwrap();
        engine.advance_stage("p1").unwrap();

        // Should be paused now
        let err = engine.execute_current_stage("p1");
        assert!(err.is_err());

        // Approve
        engine.approve_stage("p1").unwrap();
        let results = engine.execute_current_stage("p1").unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_partition_contracts() {
        let contracts = sample_contracts(100);
        let plan = RolloutPlan {
            canary_percentage: 5,
            early_adopter_percentage: 25,
            ..Default::default()
        };

        let assignments = RolloutEngine::partition_contracts(&contracts, &plan);
        assert_eq!(assignments.canary.len(), 5);
        assert_eq!(assignments.early_adopter.len(), 25);
        assert_eq!(assignments.general_availability.len(), 70);
    }
}
