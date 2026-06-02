use crate::algorithms::{
    geographic::GeographicAlgorithm, least_loaded::LeastLoadedAlgorithm,
    round_robin::RoundRobinAlgorithm, RoutingAlgorithm,
};
use crate::health::HealthChecker;
use crate::instance::ContractInstance;
use crate::session::SessionManager;
use crate::types::{
    BalancingAlgorithm, HealthStatus, InstanceMetrics, LoadBalancerConfig, LoadBalancerError,
    Region, RouteResult,
};
use anyhow::Result;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{self, Duration};

/// Central load balancer coordinating all instances, algorithms, health, and sessions
pub struct LoadBalancer {
    /// All registered instances keyed by instance ID
    instances: DashMap<String, Arc<ContractInstance>>,
    /// Active routing algorithm (swappable at runtime without lock)
    algorithm: ArcSwap<Box<dyn RoutingAlgorithm>>,
    /// Session affinity manager
    sessions: Arc<SessionManager>,
    /// Health checker
    health_checker: Arc<HealthChecker>,
    /// Current configuration
    config: LoadBalancerConfig,
}

impl LoadBalancer {
    /// Create a new load balancer with the given config
    pub fn new(config: LoadBalancerConfig) -> Arc<Self> {
        let algorithm: Box<dyn RoutingAlgorithm> = match config.algorithm {
            BalancingAlgorithm::RoundRobin => Box::new(RoundRobinAlgorithm::new()),
            BalancingAlgorithm::LeastLoaded => Box::new(LeastLoadedAlgorithm::new()),
            BalancingAlgorithm::Geographic => Box::new(GeographicAlgorithm::new(None)),
        };

        let sessions = Arc::new(SessionManager::new(config.session_ttl_secs));
        let health_checker = Arc::new(HealthChecker::new(config.clone()));

        Arc::new(Self {
            instances: DashMap::new(),
            algorithm: ArcSwap::from_pointee(algorithm),
            sessions,
            health_checker,
            config,
        })
    }

    /// Register a new contract instance with the balancer
    pub fn register_instance(
        &self,
        id: impl Into<String>,
        contract_id: impl Into<String>,
        rpc_endpoint: impl Into<String>,
        region: Region,
        weight: u32,
    ) {
        let instance = ContractInstance::new(id, contract_id, rpc_endpoint, region, weight);
        self.instances.insert(instance.id.clone(), instance);
    }

    /// Remove an instance and evict its sessions
    pub fn remove_instance(&self, id: &str) {
        self.instances.remove(id);
        self.sessions.evict_instance(id);
    }

    /// Route a request — respects session affinity, then falls back to algorithm
    pub fn route(&self, session_key: Option<&str>) -> Result<RouteResult, LoadBalancerError> {
        // Check session affinity first
        if let Some(key) = session_key {
            if let Some(pinned_id) = self.sessions.get(key) {
                if let Some(instance) = self.instances.get(&pinned_id) {
                    if instance.is_available() {
                        instance.increment_connections();
                        return Ok(RouteResult {
                            instance_id: instance.id.clone(),
                            contract_id: instance.contract_id.clone(),
                            rpc_endpoint: instance.rpc_endpoint.clone(),
                            algorithm_used: self.config.algorithm.clone(),
                            session_affinity: true,
                        });
                    }
                    // Pinned instance is unhealthy — evict session and re-route
                    self.sessions.evict_instance(&pinned_id);
                }
            }
        }

        // Collect available instances
        let instances: Vec<Arc<ContractInstance>> = self
            .instances
            .iter()
            .map(|e| Arc::clone(e.value()))
            .collect();

        if instances.is_empty() {
            return Err(LoadBalancerError::NoHealthyInstances);
        }

        // Use the active algorithm to pick an instance
        let algorithm = self.algorithm.load();
        let selected = algorithm.select(&instances)?;
        selected.increment_connections();

        // Pin session if key provided
        if let Some(key) = session_key {
            self.sessions.set(key, selected.id.clone());
        }

        Ok(RouteResult {
            instance_id: selected.id.clone(),
            contract_id: selected.contract_id.clone(),
            rpc_endpoint: selected.rpc_endpoint.clone(),
            algorithm_used: self.config.algorithm.clone(),
            session_affinity: false,
        })
    }

    /// Record outcome of a routed request
    pub fn record_result(&self, instance_id: &str, success: bool, response_ms: f64) {
        if let Some(instance) = self.instances.get(instance_id) {
            if success {
                instance.record_success(response_ms);
            } else {
                instance.record_failure();
                // Auto-mark unhealthy after threshold
                let failures = instance
                    .consecutive_failures
                    .load(std::sync::atomic::Ordering::Relaxed);
                if failures >= self.config.unhealthy_threshold {
                    *instance.health.write() = HealthStatus::Unhealthy;
                    self.sessions.evict_instance(instance_id);
                }
            }
        }
    }

    /// Switch the active algorithm at runtime (no downtime)
    pub fn set_algorithm(&self, algorithm: BalancingAlgorithm) {
        let new_algo: Box<dyn RoutingAlgorithm> = match algorithm {
            BalancingAlgorithm::RoundRobin => Box::new(RoundRobinAlgorithm::new()),
            BalancingAlgorithm::LeastLoaded => Box::new(LeastLoadedAlgorithm::new()),
            BalancingAlgorithm::Geographic => Box::new(GeographicAlgorithm::new(None)),
        };
        self.algorithm.store(Arc::new(new_algo));
    }

    /// Get metrics for all instances
    pub fn metrics(&self) -> Vec<(String, InstanceMetrics)> {
        self.instances
            .iter()
            .map(|e| (e.key().clone(), e.value().metrics()))
            .collect()
    }

    /// Get count of healthy instances
    pub fn healthy_count(&self) -> usize {
        self.instances
            .iter()
            .filter(|e| e.value().is_available())
            .count()
    }

    /// Get total registered instance count
    pub fn total_count(&self) -> usize {
        self.instances.len()
    }

    /// Start background health check loop (call once, runs forever)
    pub async fn start_health_checks(self: Arc<Self>) {
        let interval = Duration::from_secs(self.config.health_check_interval_secs);
        let mut ticker = time::interval(interval);

        loop {
            ticker.tick().await;
            let instances: Vec<Arc<ContractInstance>> = self
                .instances
                .iter()
                .map(|e| Arc::clone(e.value()))
                .collect();

            self.health_checker.check_all(&instances).await;

            // Evict sessions for newly unhealthy instances
            for instance in &instances {
                let health = instance.health.read();
                if *health == HealthStatus::Unhealthy {
                    self.sessions.evict_instance(&instance.id);
                }
            }

            // Purge expired sessions periodically
            self.sessions.purge_expired();
        }
    }
}
