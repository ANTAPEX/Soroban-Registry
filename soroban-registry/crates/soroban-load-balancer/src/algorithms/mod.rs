pub mod geographic;
pub mod least_loaded;
pub mod round_robin;

use crate::instance::ContractInstance;
use crate::types::LoadBalancerError;
use std::sync::Arc;

/// Common trait all routing algorithms must implement
pub trait RoutingAlgorithm: Send + Sync {
    /// Select the next instance to route to
    fn select(
        &self,
        instances: &[Arc<ContractInstance>],
    ) -> Result<Arc<ContractInstance>, LoadBalancerError>;

    /// Name of this algorithm for logging
    fn name(&self) -> &'static str;
}
