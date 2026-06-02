use super::RoutingAlgorithm;
use crate::instance::ContractInstance;
use crate::types::{LoadBalancerError, Region};
use parking_lot::RwLock;
use std::sync::Arc;

/// Routes to the nearest geographic region, falling back to round-robin
pub struct GeographicAlgorithm {
    preferred_region: RwLock<Option<Region>>,
}

impl GeographicAlgorithm {
    pub fn new(preferred_region: Option<Region>) -> Self {
        Self {
            preferred_region: RwLock::new(preferred_region),
        }
    }

    pub fn set_region(&self, region: Region) {
        *self.preferred_region.write() = Some(region);
    }
}

impl RoutingAlgorithm for GeographicAlgorithm {
    fn select(
        &self,
        instances: &[Arc<ContractInstance>],
    ) -> Result<Arc<ContractInstance>, LoadBalancerError> {
        let preferred = self.preferred_region.read().clone();

        // Try preferred region first
        if let Some(region) = preferred {
            let regional: Vec<_> = instances
                .iter()
                .filter(|i| i.is_available() && i.region == region)
                .collect();

            if !regional.is_empty() {
                // Pick the least loaded within the region
                return regional
                    .iter()
                    .min_by(|a, b| {
                        a.load_score()
                            .partial_cmp(&b.load_score())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|i| Arc::clone(i))
                    .ok_or(LoadBalancerError::NoHealthyInstances);
            }
        }

        // Fall back to any available instance (least loaded)
        instances
            .iter()
            .filter(|i| i.is_available())
            .min_by(|a, b| {
                a.load_score()
                    .partial_cmp(&b.load_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(Arc::clone)
            .ok_or(LoadBalancerError::NoHealthyInstances)
    }

    fn name(&self) -> &'static str {
        "geographic"
    }
}
