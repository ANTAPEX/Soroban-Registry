use serde::{Deserialize, Serialize};

/// Supported load balancing algorithms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BalancingAlgorithm {
    #[default]
    RoundRobin,
    LeastLoaded,
    Geographic,
}

/// Health status of a contract instance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,  // responding but slow
    Unhealthy, // failing health checks
    #[default]
    Unknown, // not yet checked
}

/// Geographic region of a contract instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Region {
    UsEast,
    UsWest,
    EuWest,
    EuCentral,
    ApSoutheast,
    ApNortheast,
    Custom(String),
}

/// Metrics snapshot for a single instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceMetrics {
    pub active_connections: u32,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub avg_response_ms: f64,
    pub last_checked: Option<String>, // ISO 8601 timestamp
}

impl Default for InstanceMetrics {
    fn default() -> Self {
        Self {
            active_connections: 0,
            total_requests: 0,
            failed_requests: 0,
            avg_response_ms: 0.0,
            last_checked: None,
        }
    }
}

/// Result of routing a request to an instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub instance_id: String,
    pub contract_id: String,
    pub rpc_endpoint: String,
    pub algorithm_used: BalancingAlgorithm,
    pub session_affinity: bool,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub algorithm: BalancingAlgorithm,
    pub health_check_interval_secs: u64,
    pub session_ttl_secs: u64,
    pub max_retries: u32,
    pub unhealthy_threshold: u32, // consecutive failures before marking unhealthy
    pub healthy_threshold: u32,   // consecutive successes before marking healthy again
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            algorithm: BalancingAlgorithm::RoundRobin,
            health_check_interval_secs: 30,
            session_ttl_secs: 300,
            max_retries: 3,
            unhealthy_threshold: 3,
            healthy_threshold: 2,
        }
    }
}

/// Error types for the load balancer
#[derive(Debug, thiserror::Error)]
pub enum LoadBalancerError {
    #[error("No healthy instances available")]
    NoHealthyInstances,

    #[error("Instance '{0}' not found")]
    InstanceNotFound(String),

    #[error("All instances exhausted after {0} retries")]
    AllInstancesExhausted(u32),

    #[error("Session '{0}' expired or not found")]
    SessionNotFound(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
