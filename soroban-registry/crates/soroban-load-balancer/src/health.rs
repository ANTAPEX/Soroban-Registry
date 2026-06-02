use crate::instance::ContractInstance;
use crate::types::{HealthStatus, LoadBalancerConfig};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performs periodic health checks on contract instances
pub struct HealthChecker {
    config: LoadBalancerConfig,
    client: reqwest::Client,
}

impl HealthChecker {
    pub fn new(config: LoadBalancerConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Check a single instance and update its health status
    pub async fn check_instance(&self, instance: &Arc<ContractInstance>) -> HealthStatus {
        let start = Instant::now();

        let result = self
            .client
            .post(&instance.rpc_endpoint)
            .header("Content-Type", "application/json")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"getHealth","params":{}}"#)
            .send()
            .await;

        let elapsed_ms = start.elapsed().as_millis() as f64;

        match result {
            Ok(resp) if resp.status().is_success() => {
                let consecutive_ok = instance
                    .consecutive_successes
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1;
                instance
                    .consecutive_failures
                    .store(0, std::sync::atomic::Ordering::Relaxed);

                // Update moving average response time
                {
                    let mut avg = instance.avg_response_ms.write();
                    *avg = (*avg * 0.8) + (elapsed_ms * 0.2);
                }

                // Determine status based on response time and consecutive successes
                let new_status = if elapsed_ms > 5000.0 {
                    HealthStatus::Degraded
                } else if consecutive_ok >= self.config.healthy_threshold {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                };

                *instance.health.write() = new_status.clone();
                new_status
            }
            _ => {
                let consecutive_failures = instance
                    .consecutive_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1;
                instance
                    .consecutive_successes
                    .store(0, std::sync::atomic::Ordering::Relaxed);

                let new_status = if consecutive_failures >= self.config.unhealthy_threshold {
                    HealthStatus::Unhealthy
                } else {
                    HealthStatus::Degraded
                };

                *instance.health.write() = new_status.clone();
                new_status
            }
        }
    }

    /// Run health checks on all instances concurrently
    pub async fn check_all(&self, instances: &[Arc<ContractInstance>]) {
        let futures: Vec<_> = instances.iter().map(|i| self.check_instance(i)).collect();

        futures::future::join_all(futures).await;
    }
}
