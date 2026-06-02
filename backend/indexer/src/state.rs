/// State persistence module
/// Tracks and persists the last indexed ledger height for safe resume after restarts
use shared::Network;
use sqlx::PgPool;
use sqlx::Row;
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum StateError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("State not found for network: {0:?}")]
    StateNotFound(Network),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Indexer state
#[derive(Debug, Clone)]
pub struct IndexerState {
    pub network: Network,
    pub last_indexed_ledger_height: u64,
    pub last_indexed_ledger_hash: Option<String>,
    pub last_checkpoint_ledger_height: u64,
    pub consecutive_failures: i32,
}

impl IndexerState {
    /// Get the next ledger to process
    pub fn next_ledger_to_process(&self) -> u64 {
        self.last_indexed_ledger_height + 1
    }

    /// Update checkpoint on successful processing
    pub fn update_checkpoint(&mut self, ledger_height: u64) {
        self.last_checkpoint_ledger_height = ledger_height;
    }

    /// Record a processing failure
    pub fn record_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
    }

    /// Clear failures on successful operation
    pub fn clear_failures(&mut self) {
        self.consecutive_failures = 0;
    }
}

/// State manager for reading/writing indexer state
pub struct StateManager {
    pool: PgPool,
}

impl StateManager {
    /// Create new state manager
    pub fn new(pool: PgPool) -> Self {
        StateManager { pool }
    }

    /// Load current state for a network
    pub async fn load_state(&self, network: &Network) -> Result<IndexerState, StateError> {
        let network_str = network_to_str(network);
        debug!("Loading indexer state for network: {}", network_str);

        let query_string = r#"
            SELECT 
                network::text,
                last_indexed_ledger_height,
                last_indexed_ledger_hash,
                last_checkpoint_ledger_height,
                consecutive_failures
            FROM indexer_state
            WHERE network = $1::network_type
        "#;

        let row = sqlx::query(query_string)
            .bind(network_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StateError::DatabaseError(e.to_string()))?
            .ok_or_else(|| StateError::StateNotFound(network.clone()))?;

        Ok(IndexerState {
            network: network.clone(),
            last_indexed_ledger_height: row
                .try_get::<i64, _>("last_indexed_ledger_height")
                .unwrap_or(0) as u64,
            last_indexed_ledger_hash: row
                .try_get::<Option<String>, _>("last_indexed_ledger_hash")
                .unwrap_or(None),
            last_checkpoint_ledger_height: row
                .try_get::<i64, _>("last_checkpoint_ledger_height")
                .unwrap_or(0) as u64,
            consecutive_failures: row.try_get::<i32, _>("consecutive_failures").unwrap_or(0),
        })
    }

    /// Update state after successfully processing a ledger
    pub async fn update_state(&self, state: &IndexerState) -> Result<(), StateError> {
        let network_str = network_to_str(&state.network);
        debug!(
            "Updating indexer state: network={}, ledger_height={}",
            network_str, state.last_indexed_ledger_height
        );

        sqlx::query(
            r#"
            UPDATE indexer_state
            SET 
                last_indexed_ledger_height = $1,
                last_indexed_ledger_hash = $2,
                last_checkpoint_ledger_height = $3,
                consecutive_failures = $4,
                indexed_at = NOW()
            WHERE network = $5::network_type
        "#,
        )
        .bind(state.last_indexed_ledger_height as i64)
        .bind(&state.last_indexed_ledger_hash)
        .bind(state.last_checkpoint_ledger_height as i64)
        .bind(state.consecutive_failures)
        .bind(network_str)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update indexer state: {}", e);
            StateError::DatabaseError(e.to_string())
        })?;

        info!(
            "State updated successfully: network={}, ledger_height={}",
            network_str, state.last_indexed_ledger_height
        );

        Ok(())
    }

    /// Update checkpoint for reorg recovery
    pub async fn update_checkpoint(
        &self,
        network: &Network,
        checkpoint_height: u64,
    ) -> Result<(), StateError> {
        let network_str = network_to_str(network);
        debug!(
            "Updating checkpoint: network={}, height={}",
            network_str, checkpoint_height
        );

        sqlx::query(
            r#"
            UPDATE indexer_state
            SET 
                last_checkpoint_ledger_height = $1,
                checkpoint_at = NOW()
            WHERE network = $2::network_type
        "#,
        )
        .bind(checkpoint_height as i64)
        .bind(network_str)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update checkpoint: {}", e);
            StateError::DatabaseError(e.to_string())
        })?;

        info!(
            "Checkpoint updated: network={}, height={}",
            network_str, checkpoint_height
        );

        Ok(())
    }

    /// Record error state
    pub async fn record_error(
        &self,
        network: &Network,
        error_message: &str,
    ) -> Result<(), StateError> {
        let network_str = network_to_str(network);
        warn!(
            "Recording error state: network={}, error={}",
            network_str, error_message
        );

        sqlx::query(
            r#"
            UPDATE indexer_state
            SET 
                error_message = $1,
                consecutive_failures = consecutive_failures + 1,
                updated_at = NOW()
            WHERE network = $2::network_type
        "#,
        )
        .bind(error_message)
        .bind(network_str)
        .execute(&self.pool)
        .await
        .map_err(|e| StateError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get all current states (useful for monitoring)
    pub async fn get_all_states(&self) -> Result<Vec<IndexerState>, StateError> {
        // Use runtime query execution instead of compile-time macros
        let query_string = r#"
            SELECT 
                network::text as network,
                last_indexed_ledger_height,
                last_indexed_ledger_hash,
                last_checkpoint_ledger_height,
                consecutive_failures
            FROM indexer_state
            ORDER BY network
        "#;

        let rows = sqlx::query(query_string)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StateError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let network_str: String = row.try_get("network").ok()?;
                let network = match network_str.as_str() {
                    "mainnet" => Network::Mainnet,
                    "testnet" => Network::Testnet,
                    "futurenet" => Network::Futurenet,
                    _ => return None,
                };

                Some(IndexerState {
                    network,
                    last_indexed_ledger_height: row
                        .try_get::<i64, _>("last_indexed_ledger_height")
                        .ok()? as u64,
                    last_indexed_ledger_hash: row
                        .try_get::<Option<String>, _>("last_indexed_ledger_hash")
                        .ok()?,
                    last_checkpoint_ledger_height: row
                        .try_get::<i64, _>("last_checkpoint_ledger_height")
                        .ok()? as u64,
                    consecutive_failures: row.try_get("consecutive_failures").ok()?,
                })
            })
            .collect())
    }
}

/// Convert Network enum to string for database queries
fn network_to_str(network: &Network) -> &str {
    match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
        Network::Futurenet => "futurenet",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(last_indexed: u64, failures: i32) -> IndexerState {
        IndexerState {
            network: Network::Testnet,
            last_indexed_ledger_height: last_indexed,
            last_indexed_ledger_hash: None,
            last_checkpoint_ledger_height: last_indexed,
            consecutive_failures: failures,
        }
    }

    #[test]
    fn test_state_next_ledger() {
        let state = IndexerState {
            network: Network::Testnet,
            last_indexed_ledger_height: 100,
            last_indexed_ledger_hash: None,
            last_checkpoint_ledger_height: 100,
            consecutive_failures: 0,
        };
        assert_eq!(state.next_ledger_to_process(), 101);
    }

    #[test]
    fn test_state_next_ledger_from_zero() {
        let state = make_state(0, 0);
        assert_eq!(state.next_ledger_to_process(), 1);
    }

    #[test]
    fn test_state_record_failure() {
        let mut state = IndexerState {
            network: Network::Testnet,
            last_indexed_ledger_height: 100,
            last_indexed_ledger_hash: None,
            last_checkpoint_ledger_height: 100,
            consecutive_failures: 0,
        };

        state.record_failure();
        assert_eq!(state.consecutive_failures, 1);

        state.record_failure();
        assert_eq!(state.consecutive_failures, 2);
    }

    #[test]
    fn test_state_clear_failures() {
        let mut state = IndexerState {
            network: Network::Testnet,
            last_indexed_ledger_height: 100,
            last_indexed_ledger_hash: None,
            last_checkpoint_ledger_height: 100,
            consecutive_failures: 5,
        };

        state.clear_failures();
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_network_to_str() {
        assert_eq!(network_to_str(&Network::Mainnet), "mainnet");
        assert_eq!(network_to_str(&Network::Testnet), "testnet");
        assert_eq!(network_to_str(&Network::Futurenet), "futurenet");
    }

    // ─── Off-by-one / caught-up boundary tests ─────────────────────────

    #[test]
    fn test_caught_up_next_ledger_exceeds_latest() {
        // When last_indexed == latest_ledger, next_ledger = latest + 1
        // The indexer is caught up and should NOT try to process anything
        let state = make_state(500, 0);
        let next = state.next_ledger_to_process(); // 501
        let latest_sequence: u64 = 500;

        assert!(
            next > latest_sequence,
            "next_ledger ({}) should exceed latest ({})",
            next,
            latest_sequence
        );
    }

    #[test]
    fn test_exactly_at_latest_should_process_one() {
        // When last_indexed == latest - 1, next_ledger == latest_ledger
        // Should process exactly one ledger
        let state = make_state(499, 0);
        let next = state.next_ledger_to_process(); // 500
        let latest_sequence: u64 = 500;

        assert_eq!(next, latest_sequence);
        // ledgers_to_process = min(latest - next + 1, max) = min(1, 10) = 1
        let ledgers_to_process = std::cmp::min(latest_sequence - next + 1, 10);
        assert_eq!(ledgers_to_process, 1);
    }

    #[test]
    fn test_behind_by_five_should_process_five() {
        let state = make_state(95, 0);
        let next = state.next_ledger_to_process(); // 96
        let latest_sequence: u64 = 100;

        let ledgers_to_process = std::cmp::min(latest_sequence - next + 1, 10);
        assert_eq!(ledgers_to_process, 5);
    }

    #[test]
    fn test_behind_by_more_than_max_capped() {
        let state = make_state(50, 0);
        let next = state.next_ledger_to_process(); // 51
        let latest_sequence: u64 = 100;
        let max_per_cycle: u64 = 10;

        let ledgers_to_process = std::cmp::min(latest_sequence - next + 1, max_per_cycle);
        assert_eq!(ledgers_to_process, 10);
    }
}
