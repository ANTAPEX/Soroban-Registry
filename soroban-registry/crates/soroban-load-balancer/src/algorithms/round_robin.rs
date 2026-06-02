use super::RoutingAlgorithm;
use crate::instance::ContractInstance;
use crate::types::LoadBalancerError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Distributes requests evenly across all healthy instances in order
pub struct RoundRobinAlgorithm {
    counter: AtomicUsize,
}

impl RoundRobinAlgorithm {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobinAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingAlgorithm for RoundRobinAlgorithm {
    fn select(
        &self,
        instances: &[Arc<ContractInstance>],
    ) -> Result<Arc<ContractInstance>, LoadBalancerError> {
        let available: Vec<_> = instances.iter().filter(|i| i.is_available()).collect();

        if available.is_empty() {
            return Err(LoadBalancerError::NoHealthyInstances);
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % available.len();
        Ok(Arc::clone(available[idx]))
    }

    fn name(&self) -> &'static str {
        "round_robin"
    }
}
