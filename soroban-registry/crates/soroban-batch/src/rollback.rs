use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Represents a single undo action recorded during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackAction {
    /// Undo a publish: remove the deployed contract.
    UnpublishContract { contract_id: String },

    /// Undo a metadata update: restore previous metadata.
    RestoreMetadata {
        contract_id: String,
        previous_metadata: serde_json::Value,
    },

    /// Undo a network change: restore the previous network.
    RestoreNetwork {
        contract_id: String,
        previous_network: String,
    },

    /// Undo a retirement: reactivate the contract.
    ReactivateContract { contract_id: String },
}

/// Collects rollback actions during a batch run and can execute them in reverse.
#[derive(Debug, Default)]
pub struct RollbackLog {
    actions: Vec<RollbackAction>,
}

impl RollbackLog {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// Record a rollback action to be executed if the batch fails.
    pub fn record(&mut self, action: RollbackAction) {
        self.actions.push(action);
    }

    /// Number of recorded actions.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Whether there are no recorded actions.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Execute all rollback actions in reverse order (LIFO).
    /// Collects errors but does not stop; best-effort undo.
    pub fn execute(&self) -> Vec<RollbackResult> {
        let mut results: Vec<RollbackResult> = Vec::new();

        for action in self.actions.iter().rev() {
            let result = execute_single_rollback(action);
            results.push(result);
        }

        results
    }
}

/// Result of attempting a single rollback action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    pub action_description: String,
    pub success: bool,
    pub error: Option<String>,
}

fn execute_single_rollback(action: &RollbackAction) -> RollbackResult {
    let description = match action {
        RollbackAction::UnpublishContract { contract_id } => {
            format!("Unpublish contract {}", contract_id)
        }
        RollbackAction::RestoreMetadata { contract_id, .. } => {
            format!("Restore metadata for {}", contract_id)
        }
        RollbackAction::RestoreNetwork {
            contract_id,
            previous_network,
        } => {
            format!(
                "Restore network for {} to {}",
                contract_id, previous_network
            )
        }
        RollbackAction::ReactivateContract { contract_id } => {
            format!("Reactivate contract {}", contract_id)
        }
    };

    // ------------------------------------------------------------------
    // TODO: Replace with real rollback logic once the registry backend
    //       exposes undo/revert endpoints.  For now, each arm simulates
    //       a successful rollback so the pipeline is testable end-to-end.
    // ------------------------------------------------------------------
    match action {
        RollbackAction::UnpublishContract { contract_id: _ } => RollbackResult {
            action_description: description,
            success: true,
            error: None,
        },
        RollbackAction::RestoreMetadata { .. } => RollbackResult {
            action_description: description,
            success: true,
            error: None,
        },
        RollbackAction::RestoreNetwork { .. } => RollbackResult {
            action_description: description,
            success: true,
            error: None,
        },
        RollbackAction::ReactivateContract { contract_id: _ } => RollbackResult {
            action_description: description,
            success: true,
            error: None,
        },
    }
}