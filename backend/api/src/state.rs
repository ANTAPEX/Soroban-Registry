use crate::ai::service::AIService;
use crate::auth::AuthManager;
use crate::cache::{CacheConfig, CacheLayer};
use crate::{api_versioning::ApiVersionMetrics, webhooks::{WebhookDispatcher, WebhookStore}};
use prometheus::Registry;
use sqlx::PgPool;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{info, warn};

use serde_json::Value;
use shared::models::Network;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractEventVisibility {
    Public,
    Private,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum RealtimeEvent {
    ContractDeployed {
        contract_id: String,
        contract_name: String,
        publisher: String,
        version: String,
        timestamp: String,
        network: Network,
    },
    ContractUpdated {
        contract_id: String,
        update_type: String,
        details: Value,
        timestamp: String,
    },
    CicdPipeline {
        contract_id: String,
        status: String,
        steps_completed: u32,
        total_steps: u32,
        timestamp: String,
    },
    VersionCreated {
        contract_id: String,
        version: String,
        network: Network,
        timestamp: String,
    },
    MetadataUpdated {
        contract_id: String,
        timestamp: String,
        changes: Value,
        visibility: ContractEventVisibility,
    },
    StatusUpdated {
        contract_id: String,
        status: String,
        timestamp: String,
        is_verified: bool,
        details: Option<Value>,
        visibility: ContractEventVisibility,
    },
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub started_at: Instant,
    pub cache: Arc<CacheLayer>,
    pub contract_events: Arc<ContractEventHub>,
    pub registry: Registry,
    pub api_version_metrics: Arc<ApiVersionMetrics>,
    pub webhook_store: Arc<WebhookStore>,
    pub webhook_dispatcher: WebhookDispatcher,
}

impl AppState {
    pub async fn new(
        db: PgPool,
        registry: Registry,
        job_engine: Arc<soroban_batch::engine::JobEngine>,
        is_shutting_down: Arc<AtomicBool>,
        rate_limit_state: Arc<RateLimitState>,
        ai_service: Option<Arc<AIService>>,
        event_broadcaster: broadcast::Sender<RealtimeEvent>,
        db_breaker: Arc<crate::db_resilience::CircuitBreaker>,
        db_queue: Arc<crate::db_resilience::DbQueue>,
        feature_flags: Arc<FeatureFlagManager>,
    ) -> Result<Self, shared::error::RegistryError> {
        let config = CacheConfig::from_env();
        let api_version_metrics = Arc::new(ApiVersionMetrics::new());
        let webhook_store = Arc::new(WebhookStore::default());
        let webhook_dispatcher = WebhookDispatcher::spawn(Arc::clone(&webhook_store));
        Self {
            db,
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(config).await),
            contract_events,
            registry,
            api_version_metrics,
            webhook_store,
            webhook_dispatcher,
        }
    }
}
