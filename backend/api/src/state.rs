use crate::cache::{CacheConfig, CacheLayer};
use crate::{api_versioning::ApiVersionMetrics, webhooks::{WebhookDispatcher, WebhookStore}};
use prometheus::Registry;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub started_at: Instant,
    pub cache: Arc<CacheLayer>,
    pub registry: Registry,
    pub api_version_metrics: Arc<ApiVersionMetrics>,
    pub webhook_store: Arc<WebhookStore>,
    pub webhook_dispatcher: WebhookDispatcher,
}

impl AppState {
    pub fn new(db: PgPool, registry: Registry) -> Self {
        let config = CacheConfig::from_env();
        let api_version_metrics = Arc::new(ApiVersionMetrics::new());
        let webhook_store = Arc::new(WebhookStore::default());
        let webhook_dispatcher = WebhookDispatcher::spawn(Arc::clone(&webhook_store));
        Self {
            db,
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(config)),
            registry,
            api_version_metrics,
            webhook_store,
            webhook_dispatcher,
        }
    }
}
