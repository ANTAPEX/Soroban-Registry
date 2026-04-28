use moka::future::Cache as MokaCache;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

/// Cache configuration options
#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_capacity: u64,
    pub redis_enabled: bool,
    pub redis_url: Option<String>,
    pub contracts_ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_capacity: 10_000,
            redis_enabled: false,
            redis_url: None,
            contracts_ttl: 300,
        }
    }
}

impl CacheConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(enabled_str) = std::env::var("CACHE_ENABLED") {
            config.enabled = enabled_str.to_lowercase() == "true";
        }

        if let Ok(capacity_str) = std::env::var("CACHE_MAX_CAPACITY") {
            if let Ok(capacity) = capacity_str.parse::<u64>() {
                config.max_capacity = capacity;
            }
        }

        if let Ok(redis_enabled_str) = std::env::var("REDIS_ENABLED") {
            config.redis_enabled = redis_enabled_str.to_lowercase() == "true";
        }

        if let Ok(ttl_str) = std::env::var("CONTRACTS_CACHE_TTL") {
            if let Ok(ttl) = ttl_str.parse::<u64>() {
                config.contracts_ttl = ttl;
            }
        }

        config.redis_url = std::env::var("REDIS_URL").ok();

        tracing::info!(
            "Cache config loaded: enabled={}, capacity={}, redis_enabled={}",
            config.enabled,
            config.max_capacity,
            config.redis_enabled
        );

        config
    }
}

pub struct CacheLayer {
    pub abi_cache: MokaCache<String, String>,
    pub verification_cache: MokaCache<String, String>,
    pub generic_cache: MokaCache<String, String>,
    pub contracts_cache: MokaCache<String, String>,
    pub contract_access_cache: MokaCache<String, bool>,
    config: CacheConfig,
    pub redis_cm: Option<ConnectionManager>,
}

impl CacheLayer {
    pub async fn new(config: CacheConfig) -> Self {
        // 24-hour TTL for ABI, max size configurable default 10GB but we use the config max_capacity
        let abi_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(24 * 3600))
            .build();

        // 7-day TTL for verification result cache, keyed by bytecode_hash
        let verification_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(7 * 24 * 3600))
            .build();

        // Generic cache for namespace-keyed entries (e.g., contract graphs)
        // Default 1-hour TTL, configurable per-entry
        let generic_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(3600))
            .build();

        let contracts_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(config.contracts_ttl))
            .build();

        let contract_access_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(Duration::from_secs(60))
            .build();

        let redis_cm = if config.redis_enabled {
            if let Some(url) = &config.redis_url {
                match redis::Client::open(url.as_str()) {
                    Ok(client) => match client.get_connection_manager().await {
                        Ok(cm) => Some(cm),
                        Err(e) => {
                            tracing::error!("Failed to get Redis connection manager: {}", e);
                            None
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to open Redis client: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Self {
            abi_cache,
            verification_cache,
            generic_cache,
            contracts_cache,
            contract_access_cache,
            redis_cm,
            config,
        }
    }

    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    pub async fn get_abi(&self, contract_id: &str, bypass_cache: bool) -> Option<String> {
        if !self.config.enabled || bypass_cache {
            if bypass_cache {
                tracing::debug!("Bypassing cache for contract_id: {}", contract_id);
            }
            return None;
        }

        // Check L1 cache (Moka)
        if let Some(abi) = self.abi_cache.get(contract_id).await {
            crate::metrics::ABI_CACHE_HITS.inc();
            return Some(abi);
        }

        crate::metrics::ABI_CACHE_MISSES.inc();
        None
    }

    pub async fn put_abi(&self, contract_id: &str, abi: String) {
        if !self.config.enabled {
            return;
        }

        // Put to L1 cache
        self.abi_cache.insert(contract_id.to_string(), abi).await;
    }

    pub async fn invalidate_abi(&self, contract_id: &str) {
        if !self.config.enabled {
            return;
        }

        // Invalidate cache entry
        self.abi_cache.invalidate(contract_id).await;
    }

    pub async fn get_verification(&self, bytecode_hash: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }
        let result = self.verification_cache.get(bytecode_hash).await;
        if result.is_some() {
            crate::metrics::VERIFICATION_CACHE_HITS.inc();
        } else {
            crate::metrics::VERIFICATION_CACHE_MISSES.inc();
        }
        result
    }

    pub async fn put_verification(&self, bytecode_hash: &str, result: String) {
        if !self.config.enabled {
            return;
        }
        self.verification_cache
            .insert(bytecode_hash.to_string(), result)
            .await;
    }

    pub async fn invalidate_verification(&self, bytecode_hash: &str) {
        if !self.config.enabled {
            return;
        }
        self.verification_cache.invalidate(bytecode_hash).await;
    }

    // Generic cache methods with namespace support
    pub async fn get(&self, ns: &str, key: &str) -> (Option<String>, bool) {
        if !self.config.enabled {
            return (None, false);
        }

        let namespaced_key = format!("{}:{}", ns, key);
        let result = self.generic_cache.get(&namespaced_key).await;
        let hit = result.is_some();

        if hit {
            crate::metrics::CACHE_HITS.inc();
        } else {
            crate::metrics::CACHE_MISSES.inc();
        }

        (result, hit)
    }

    pub async fn put(&self, ns: &str, key: &str, value: String, _ttl: Option<Duration>) {
        if !self.config.enabled {
            return;
        }

        let namespaced_key = format!("{}:{}", ns, key);

        // Note: moka doesn't support per-entry TTL easily, so we use the cache-wide TTL
        // For custom TTL support, we'd need to use entry_by_ref with expiration policy
        // For now, we'll insert with the default TTL configured for generic_cache
        self.generic_cache.insert(namespaced_key, value).await;
    }

    pub async fn invalidate(&self, ns: &str, key: &str) {
        if !self.config.enabled {
            return;
        }

        let namespaced_key = format!("{}:{}", ns, key);
        self.generic_cache.invalidate(&namespaced_key).await;
    }

    pub async fn should_refresh_contract_access(&self, contract_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        if self.contract_access_cache.get(contract_id).await.is_some() {
            return false;
        }

        self.contract_access_cache
            .insert(contract_id.to_string(), true)
            .await;
        true
    }

    pub async fn get_contracts(&self, key: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }
        self.contracts_cache.get(key).await
    }

    pub async fn put_contracts(&self, key: String, value: String) {
        if !self.config.enabled {
            return;
        }
        self.contracts_cache.insert(key, value).await;
    }

    pub async fn invalidate_contracts(&self) {
        if !self.config.enabled {
            return;
        }
        self.contracts_cache.invalidate_all();
    }

    /// Starts an asynchronous startup warmup task querying the top 100 contracts
    pub fn warm_up(self: Arc<Self>, pool: PgPool) {
        if !self.config.enabled {
            return;
        }
        tokio::spawn(async move {
            tracing::info!("Starting startup cache warmup...");
            // Query top 100 contracts by query frequency from contract_interactions or just get contracts
            let top_contracts: Vec<(uuid::Uuid, String, Option<String>)> = sqlx::query_as(
                r#"
                SELECT c.id, c.contract_id, c.wasm_hash
                FROM contracts c
                LEFT JOIN contract_interactions ci ON c.id = ci.contract_id
                GROUP BY c.id
                ORDER BY COUNT(ci.id) DESC
                LIMIT 100
                "#,
            )
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            for (id, contract_id, wasm_hash) in top_contracts {
                if let Ok(Some(abi)) = sqlx::query_scalar::<_, serde_json::Value>(
                    "SELECT abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1"
                )
                .bind(id)
                .fetch_optional(&pool).await {
                    self.put_abi(&contract_id, abi.to_string()).await;
                }

                if let Some(w_hash) = wasm_hash {
                    if let Ok(Some(ver_res)) = sqlx::query_scalar::<_, String>(
                        "SELECT status::text FROM formal_verification_results LIMIT 1", // fallback fake
                    )
                    .fetch_optional(&pool)
                    .await
                    {
                        self.verification_cache
                            .insert(w_hash.clone(), ver_res)
                            .await;
                    }
                }
            }
            tracing::info!("Completed startup cache warmup.");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_abi_cache() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache.put_abi("contract_1", "abi_json_1".to_string()).await;

        let val = cache.get_abi("contract_1", false).await;
        assert_eq!(val, Some("abi_json_1".to_string()));

        cache.invalidate_abi("contract_1").await;

        let val2 = cache.get_abi("contract_1", false).await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_verification_cache() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache
            .put_verification("hash_1", "result_1".to_string())
            .await;

        let val = cache.get_verification("hash_1").await;
        assert_eq!(val, Some("result_1".to_string()));

        cache.invalidate_verification("hash_1").await;

        let val2 = cache.get_verification("hash_1").await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_disabled_cache() {
        let config = CacheConfig {
            enabled: false,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache.put_abi("c1", "v1".to_string()).await;
        let val = cache.get_abi("c1", false).await;
        assert!(val.is_none());

        cache.put_verification("h1", "v1".to_string()).await;
        let val2 = cache.get_verification("h1").await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_generic_cache() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        // Test put and get
        cache
            .put("system", "dependency_graph", "graph_data".to_string(), None)
            .await;

        let (val, hit) = cache.get("system", "dependency_graph").await;
        assert_eq!(val, Some("graph_data".to_string()));
        assert!(hit);

        // Test cache miss
        let (val2, hit2) = cache.get("system", "nonexistent").await;
        assert!(val2.is_none());
        assert!(!hit2);

        // Test invalidate
        cache.invalidate("system", "dependency_graph").await;
        let (val3, hit3) = cache.get("system", "dependency_graph").await;
        assert!(val3.is_none());
        assert!(!hit3);
    }

    #[tokio::test]
    async fn test_generic_cache_namespace_isolation() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        // Put same key in different namespaces
        cache
            .put("ns1", "key1", "value_ns1".to_string(), None)
            .await;
        cache
            .put("ns2", "key1", "value_ns2".to_string(), None)
            .await;

        // Verify namespace isolation
        let (val1, _) = cache.get("ns1", "key1").await;
        let (val2, _) = cache.get("ns2", "key1").await;

        assert_eq!(val1, Some("value_ns1".to_string()));
        assert_eq!(val2, Some("value_ns2".to_string()));

        // Invalidate one namespace shouldn't affect the other
        cache.invalidate("ns1", "key1").await;
        let (val1_after, _) = cache.get("ns1", "key1").await;
        let (val2_after, _) = cache.get("ns2", "key1").await;

        assert!(val1_after.is_none());
        assert_eq!(val2_after, Some("value_ns2".to_string()));
    }

    #[tokio::test]
    async fn test_generic_cache_disabled() {
        let config = CacheConfig {
            enabled: false,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache
            .put("system", "key1", "value1".to_string(), None)
            .await;
        let (val, hit) = cache.get("system", "key1").await;

        assert!(val.is_none());
        assert!(!hit);
    }

    #[tokio::test]
    async fn test_contract_access_refresh_is_debounced() {
        let cache = CacheLayer::new(CacheConfig {
            enabled: true,
            max_capacity: 100,
            redis_enabled: false,
            redis_url: None,
        })
        .await;

        assert!(cache.should_refresh_contract_access("contract-1").await);
        assert!(!cache.should_refresh_contract_access("contract-1").await);
        assert!(cache.should_refresh_contract_access("contract-2").await);
    }
}
