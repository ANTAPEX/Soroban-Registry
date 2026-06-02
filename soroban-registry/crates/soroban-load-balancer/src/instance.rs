use crate::types::{HealthStatus, InstanceMetrics, Region};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// Represents a single contract instance that can receive routed calls
#[derive(Debug)]
pub struct ContractInstance {
    pub id: String,
    pub contract_id: String,
    pub rpc_endpoint: String,
    pub region: Region,
    pub weight: u32, // for weighted algorithms
    pub health: RwLock<HealthStatus>,
    pub active_connections: AtomicU32,
    pub total_requests: AtomicU64,
    pub failed_requests: AtomicU64,
    pub consecutive_failures: AtomicU32,
    pub consecutive_successes: AtomicU32,
    pub avg_response_ms: RwLock<f64>,
}

impl ContractInstance {
    /// Create a new instance
    pub fn new(
        id: impl Into<String>,
        contract_id: impl Into<String>,
        rpc_endpoint: impl Into<String>,
        region: Region,
        weight: u32,
    ) -> Arc<Self> {
        Arc::new(Self {
            id: id.into(),
            contract_id: contract_id.into(),
            rpc_endpoint: rpc_endpoint.into(),
            region,
            weight,
            health: RwLock::new(HealthStatus::Unknown),
            active_connections: AtomicU32::new(0),
            total_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            consecutive_failures: AtomicU32::new(0),
            consecutive_successes: AtomicU32::new(0),
            avg_response_ms: RwLock::new(0.0),
        })
    }

    /// Check if this instance is eligible to receive traffic
    pub fn is_available(&self) -> bool {
        let health = self.health.read();
        matches!(*health, HealthStatus::Healthy | HealthStatus::Degraded)
    }

    /// Record a successful request and update metrics
    pub fn record_success(&self, response_ms: f64) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.consecutive_successes.fetch_add(1, Ordering::Relaxed);

        // Exponential moving average for response time
        let mut avg = self.avg_response_ms.write();
        *avg = (*avg * 0.8) + (response_ms * 0.2);
    }

    /// Record a failed request and update metrics
    pub fn record_failure(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);
    }

    /// Increment active connection count when routing to this instance
    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current load score (used by least-loaded algorithm)
    pub fn load_score(&self) -> f64 {
        let connections = self.active_connections.load(Ordering::Relaxed) as f64;
        let avg_ms = *self.avg_response_ms.read();
        // Weighted score: connections matter more than response time
        (connections * 10.0) + (avg_ms * 0.1)
    }

    /// Snapshot current metrics
    pub fn metrics(&self) -> InstanceMetrics {
        InstanceMetrics {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            avg_response_ms: *self.avg_response_ms.read(),
            last_checked: None,
        }
    }
}
