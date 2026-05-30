use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::info;

#[derive(Debug, Clone)]
pub struct FeatureFlag {
    pub key: String,
    pub is_enabled: bool,
    pub rollout_percentage: u8,
    pub targeted_users: Vec<Uuid>,
}

#[derive(Default)]
pub struct FeatureFlagMetrics {
    pub evaluations: AtomicU64,
    pub hits: AtomicU64,
}

pub struct FeatureFlagManager {
    flags: RwLock<HashMap<String, FeatureFlag>>,
    metrics: RwLock<HashMap<String, Arc<FeatureFlagMetrics>>>,
}

impl FeatureFlagManager {
    pub fn new() -> Self {
        let mut flags = HashMap::new();
        // Initialize with some safe defaults
        flags.insert(
            "new_dashboard".to_string(),
            FeatureFlag {
                key: "new_dashboard".to_string(),
                is_enabled: false,
                rollout_percentage: 0,
                targeted_users: vec![],
            },
        );

        Self {
            flags: RwLock::new(flags),
            metrics: RwLock::new(HashMap::new()),
        }
    }

    pub async fn is_enabled(&self, key: &str, user_id: Option<Uuid>) -> bool {
        // Record evaluation metrics
        let metrics_arc = {
            let mut metrics_guard = self.metrics.write().await;
            if !metrics_guard.contains_key(key) {
                metrics_guard.insert(key.to_string(), Arc::new(FeatureFlagMetrics::default()));
            }
            metrics_guard.get(key).cloned().unwrap()
        };

        metrics_arc.evaluations.fetch_add(1, Ordering::Relaxed);

        let flags = self.flags.read().await;
        if let Some(flag) = flags.get(key) {
            if !flag.is_enabled {
                return false;
            }

            // Check specific user targeting
            if let Some(uid) = user_id {
                if flag.targeted_users.contains(&uid) {
                    metrics_arc.hits.fetch_add(1, Ordering::Relaxed);
                    return true;
                }
            }

            // Check rollout percentage (deterministic based on user_id or random)
            let enabled = if flag.rollout_percentage >= 100 {
                true
            } else if flag.rollout_percentage == 0 {
                false
            } else if let Some(uid) = user_id {
                // Deterministic rollout based on user id
                let hash = self.hash_uuid(uid);
                (hash % 100) < (flag.rollout_percentage as u64)
            } else {
                // If no user ID is provided, randomly sample
                rand::random::<u8>() % 100 < flag.rollout_percentage
            };

            if enabled {
                metrics_arc.hits.fetch_add(1, Ordering::Relaxed);
            }

            return enabled;
        }

        // Safe default for undefined flags
        false
    }

    pub async fn update_flag(&self, flag: FeatureFlag) {
        info!("Updating feature flag: {}", flag.key);
        let mut flags = self.flags.write().await;
        flags.insert(flag.key.clone(), flag);
    }

    pub async fn get_metrics(&self) -> HashMap<String, (u64, u64)> {
        let metrics = self.metrics.read().await;
        let mut result = HashMap::new();
        for (key, m) in metrics.iter() {
            result.insert(
                key.clone(),
                (
                    m.evaluations.load(Ordering::Relaxed),
                    m.hits.load(Ordering::Relaxed),
                ),
            );
        }
        result
    }

    fn hash_uuid(&self, uid: Uuid) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        uid.hash(&mut hasher);
        hasher.finish()
    }
}
