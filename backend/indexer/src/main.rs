#![allow(dead_code, unused)]

/// Stellar Blockchain Indexer Service
/// Continuously monitors Stellar network for contract deployments and syncs to registry database
///
/// This service:
/// - Polls Stellar RPC endpoint on 30-second intervals (configurable)
/// - Detects createContract operations in new ledgers
/// - Extracts contract metadata (ID, deployer, network)
/// - Writes unverified contract records to database
/// - Tracks last indexed ledger for safe resume after restarts
/// - Handles RPC failures with exponential backoff
/// - Detects and recovers from ledger reorgs
/// - Provides structured logging for observability

mod backoff;
mod config;
mod db;
mod detector;
mod reorg;
mod rpc;
mod state;

use anyhow::Result;
use config::{DatabaseConfig, ServiceConfig};
use db::DatabaseWriter;
use reorg::ReorgHandler;
use rpc::StellarRpcClient;
use state::{IndexerState, StateManager};
use std::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

struct IndexerService {
    config: ServiceConfig,
    rpc_client: StellarRpcClient,
    db_writer: DatabaseWriter,
    state_manager: StateManager,
    reorg_handler: ReorgHandler,
    backoff: backoff::ExponentialBackoff,
}

impl IndexerService {
    /// Initialize the indexer service
    async fn new(config: ServiceConfig) -> Result<Self> {
        // Initialize database connection
        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .connect(&config.database.connection_string)
            .await?;

        let rpc_client = StellarRpcClient::new(config.network.rpc_endpoint.clone());
        let db_writer = DatabaseWriter::new(db_pool.clone());
        let state_manager = StateManager::new(db_pool);
        let reorg_handler = ReorgHandler::new(config.reorg_checkpoint_depth);
        let backoff = backoff::ExponentialBackoff::new(
            config.backoff_base_interval_secs,
            config.backoff_max_interval_secs,
        );

        Ok(IndexerService {
            config,
            rpc_client,
            db_writer,
            state_manager,
            reorg_handler,
            backoff,
        })
    }

    /// Run the main indexing loop
    async fn run(&mut self) -> Result<()> {
        info!(
            "Starting indexer service for network: {}",
            self.config.network.network_name()
        );

        // Load initial state
        let mut state = match self.state_manager.load_state(&self.config.network.network).await {
            Ok(s) => {
                info!(
                    "Loaded indexer state: last_indexed_ledger={}",
                    s.last_indexed_ledger_height
                );
                s
            }
            Err(e) => {
                error!(
                    "Failed to load indexer state: {}, initializing with defaults",
                    e
                );
                IndexerState {
                    network: self.config.network.network.clone(),
                    last_indexed_ledger_height: 0,
                    last_checkpoint_ledger_height: 0,
                    consecutive_failures: 0,
                }
            }
        };

        // Health check before starting
        match self.rpc_client.health_check().await {
            Ok(_) => info!("RPC endpoint health check passed"),
            Err(e) => warn!("Initial RPC health check failed: {}, will retry", e),
        }

        // Main polling loop
        loop {
            let poll_duration = Duration::from_secs(self.config.network.poll_interval_secs);

            match self.poll_and_index(&mut state).await {
                Ok(_) => {
                    self.backoff.on_success();
                }
                Err(e) => {
                    error!("Error during polling cycle: {}", e);
                    state.record_failure();

                    let backoff_duration = self.backoff.on_failure(&e.to_string());
                    let backoff_secs = backoff_duration.as_secs();

                    // Record error in state manager
                    let _ = self
                        .state_manager
                        .record_error(&self.config.network.network, &e.to_string())
                        .await;

                    warn!(
                        attempt = self.backoff.attempts(),
                        backoff_secs = backoff_secs,
                        "Backing off before retry"
                    );

                    tokio::time::sleep(backoff_duration).await;
                    continue;
                }
            }

            // Wait for next poll cycle
            tokio::time::sleep(poll_duration).await;
        }
    }

    /// Single polling and indexing cycle
    async fn poll_and_index(&mut self, state: &mut IndexerState) -> Result<()> {
        let network_name = self.config.network.network_name();

        // Get latest ledger
        let latest_ledger = self.rpc_client.get_latest_ledger().await?;
        let next_ledger = state.next_ledger_to_process();

        info!(
            network = network_name,
            latest_ledger = latest_ledger.sequence,
            next_ledger = next_ledger,
            lag = latest_ledger.sequence.saturating_sub(next_ledger),
            "Poll cycle started"
        );

        // Check for reorg
        if self
            .reorg_handler
            .check_for_reorg(&self.rpc_client, state)
            .await?
        {
            warn!(
                network = network_name,
                "Reorg detected, recovering to checkpoint"
            );
            self.reorg_handler
                .recover_from_reorg(state, &self.state_manager)
                .await?;
            return Ok(());
        }

        // Process ledgers up to latest (but limit to prevent long processing cycles)
        let max_ledgers_per_cycle = 10;
        let ledgers_to_process = std::cmp::min(
            latest_ledger.sequence.saturating_sub(next_ledger) + 1,
            max_ledgers_per_cycle,
        );

        let mut total_contracts = 0;

        for i in 0..ledgers_to_process {
            let ledger_height = next_ledger + i;

            // Fetch ledger operations
            match self.rpc_client.get_ledger_operations(ledger_height).await {
                Ok(operations) => {
                    info!(
                        network = network_name,
                        ledger = ledger_height,
                        operations = operations.len(),
                        "Fetched ledger operations"
                    );

                    // Detect contract deployments
                    let deployments =
                        detector::detect_contract_deployments(&operations, ledger_height);

                    if !deployments.is_empty() {
                        info!(
                            network = network_name,
                            ledger = ledger_height,
                            contracts = deployments.len(),
                            "Found contract deployments"
                        );

                        // Write to database
                        match self
                            .db_writer
                            .write_contracts_batch(&deployments, &self.config.network.network)
                            .await
                        {
                            Ok((new_count, duplicate_count)) => {
                                info!(
                                    network = network_name,
                                    ledger = ledger_height,
                                    new = new_count,
                                    duplicates = duplicate_count,
                                    "Contracts written to database"
                                );
                                total_contracts += new_count;
                            }
                            Err(e) => {
                                error!(
                                    network = network_name,
                                    ledger = ledger_height,
                                    error = %e,
                                    "Failed to write contracts"
                                );
                                return Err(e.into());
                            }
                        }
                    }

                    // Update state
                    state.last_indexed_ledger_height = ledger_height;
                    state.clear_failures();

                    // Check if we should update checkpoint
                    if self.reorg_handler.should_update_checkpoint(
                        ledger_height,
                        state.last_checkpoint_ledger_height,
                    ) {
                        state.update_checkpoint(ledger_height);
                        self.state_manager
                            .update_checkpoint(&self.config.network.network, ledger_height)
                            .await?;
                    }
                }
                Err(e) => {
                    error!(
                        network = network_name,
                        ledger = ledger_height,
                        error = %e,
                        "Failed to fetch ledger operations"
                    );
                    return Err(e.into());
                }
            }
        }

        // Persist state after successful cycle
        self.state_manager.update_state(state).await?;

        info!(
            network = network_name,
            processed = ledgers_to_process,
            new_contracts = total_contracts,
            "Poll cycle completed successfully"
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "indexer=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .init();

    info!("Stellar Blockchain Indexer Service starting...");

    // Load configuration
    let config = ServiceConfig::from_env()?;

    // Initialize service
    let mut service = IndexerService::new(config).await?;

    // Setup graceful shutdown signal handler
    let shutdown_signal = signal_support::create_shutdown_signal();

    // Run service
    tokio::select! {
        result = service.run() => {
            match result {
                Ok(_) => {
                    info!("Indexer service completed normally");
                    Ok(())
                }
                Err(e) => {
                    error!("Indexer service encountered fatal error: {}", e);
                    Err(e)
                }
            }
        }
        _ = shutdown_signal => {
            info!("Received shutdown signal, gracefully exiting...");
            Ok(())
        }
    }
}

/// Signal handling support
mod signal_support {
    use std::future::Future;

    pub fn create_shutdown_signal() -> impl Future<Output = ()> {
        async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};

                let mut sigterm = signal(SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
                let mut sigint = signal(SignalKind::interrupt())
                    .expect("Failed to register SIGINT handler");

                tokio::select! {
                    _ = sigterm.recv() => {
                        tracing::info!("Received SIGTERM");
                    }
                    _ = sigint.recv() => {
                        tracing::info!("Received SIGINT");
                    }
                }
            }

            #[cfg(windows)]
            {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to listen for Ctrl+C");
                tracing::info!("Received Ctrl+C");
            }
        }
    }
}
