use crate::rpc::StellarRpcClient;
/// Ledger reorganization handling module
/// Detects when ledgers have been reorganized on-chain and safely recovers to a checkpoint
use crate::state::{IndexerState, StateManager};
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum ReorgError {
    #[error("Ledger hash mismatch: stored={0}, current={1}")]
    HashMismatch(String, String),
    #[error("RPC error during reorg check: {0}")]
    Rpc(String),
    #[error("State manager error: {0}")]
    State(String),
}

/// Reorg detector and recovery handler
pub struct ReorgHandler {
    checkpoint_depth: u64,
}

impl ReorgHandler {
    /// Create new reorg handler
    pub fn new(checkpoint_depth: u64) -> Self {
        ReorgHandler { checkpoint_depth }
    }

    /// Detect if a reorg has occurred by comparing ledger hashes
    pub async fn check_for_reorg(
        &self,
        rpc_client: &StellarRpcClient,
        state: &IndexerState,
    ) -> Result<bool, ReorgError> {
        if state.last_indexed_ledger_height == 0 {
            // No ledgers yet, no reorg possible
            return Ok(false);
        }

        let stored_hash = match &state.last_indexed_ledger_hash {
            Some(hash) => hash,
            None => {
                // If we don't have a stored hash but have indexed ledgers,
                // we should probably assume no reorg for now but warn
                warn!(
                    "No stored ledger hash for height {}, skipping reorg check",
                    state.last_indexed_ledger_height
                );
                return Ok(false);
            }
        };

        // Fetch the last indexed ledger to verify its hash
        let ledger = rpc_client
            .get_ledger(state.last_indexed_ledger_height)
            .await
            .map_err(|e| {
                ReorgError::Rpc(format!(
                    "Failed to fetch ledger {}: {}",
                    state.last_indexed_ledger_height, e
                ))
            })?;

        if &ledger.hash != stored_hash {
            warn!(
                "Reorg detected! Hash mismatch at height {}: stored={}, current={}",
                state.last_indexed_ledger_height, stored_hash, ledger.hash
            );
            return Ok(true);
        }

        Ok(false)
    }

    /// Recover from a reorg by falling back to checkpoint
    pub async fn recover_from_reorg(
        &self,
        state: &mut IndexerState,
        state_manager: &StateManager,
    ) -> Result<(), ReorgError> {
        warn!(
            "Recovering from reorg: falling back from {} to checkpoint {}",
            state.last_indexed_ledger_height, state.last_checkpoint_ledger_height
        );

        // Fall back to last checkpoint
        state.last_indexed_ledger_height = state.last_checkpoint_ledger_height;
        state.last_indexed_ledger_hash = None;

        // Persist the recovery
        state_manager
            .update_state(state)
            .await
            .map_err(|e| ReorgError::State(e.to_string()))?;

        info!(
            "Recovered from reorg: resumed from ledger height {}",
            state.last_indexed_ledger_height
        );

        Ok(())
    }

    /// Check if we should force a checkpoint update (every N ledgers for safety)
    pub fn should_update_checkpoint(&self, ledger_height: u64, last_checkpoint: u64) -> bool {
        ledger_height >= last_checkpoint + self.checkpoint_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reorg_handler_creation() {
        let handler = ReorgHandler::new(100);
        assert_eq!(handler.checkpoint_depth, 100);
    }

    #[test]
    fn test_should_update_checkpoint() {
        let handler = ReorgHandler::new(100);

        // Exactly at checkpoint boundary
        assert!(handler.should_update_checkpoint(100, 0));
        // Not enough ledgers since last checkpoint
        assert!(!handler.should_update_checkpoint(50, 0));

        // Beyond checkpoint boundary
        assert!(handler.should_update_checkpoint(105, 0));
        // Not yet beyond the checkpoint when last_checkpoint is 50
        assert!(!handler.should_update_checkpoint(105, 50));
    }

    #[test]
    fn test_reorg_handler_state() {
        let mut state = IndexerState {
            network: shared::Network::Testnet,
            last_indexed_ledger_height: 500,
            last_indexed_ledger_hash: None,
            last_checkpoint_ledger_height: 400,
            consecutive_failures: 2,
        };

        // Simulate recovery
        state.last_indexed_ledger_height = state.last_checkpoint_ledger_height;
        state.clear_failures();

        assert_eq!(state.last_indexed_ledger_height, 400);
        assert_eq!(state.consecutive_failures, 0);
    }
}
