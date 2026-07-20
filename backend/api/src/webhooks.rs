use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, RwLock};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    ContractVerified,
    ContractDeployed,
    ContractUpdated,
}

impl WebhookEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WebhookEventType::ContractVerified => "contract_verified",
            WebhookEventType::ContractDeployed => "contract_deployed",
            WebhookEventType::ContractUpdated => "contract_updated",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub target_url: String,
    pub subscribed_events: Vec<WebhookEventType>,
    pub contract_id: Option<String>,
    pub secret: Vec<u8>,
    pub active: bool,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebhookEndpointPublic {
    pub id: Uuid,
    pub target_url: String,
    pub subscribed_events: Vec<WebhookEventType>,
    pub contract_id: Option<String>,
    pub active: bool,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
}

impl From<&WebhookEndpoint> for WebhookEndpointPublic {
    fn from(value: &WebhookEndpoint) -> Self {
        Self {
            id: value.id,
            target_url: value.target_url.clone(),
            subscribed_events: value.subscribed_events.clone(),
            contract_id: value.contract_id.clone(),
            active: value.active,
            retry_count: value.retry_count,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebhookEndpointCreated {
    pub webhook: WebhookEndpointPublic,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: Value,
    pub status_code: Option<u16>,
    pub response_body: Option<String>,
    pub success: bool,
    pub retry_attempts: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebhookDeadLetter {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: Value,
    pub last_status_code: Option<u16>,
    pub last_response_body: Option<String>,
    pub retry_attempts: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Default)]
pub struct WebhookStore {
    endpoints: RwLock<HashMap<Uuid, WebhookEndpoint>>,
    deliveries: RwLock<Vec<WebhookDelivery>>,
    dead_letters: RwLock<HashMap<Uuid, WebhookDeadLetter>>,
}

impl WebhookStore {
    pub async fn insert_endpoint(&self, endpoint: WebhookEndpoint) {
        self.endpoints.write().await.insert(endpoint.id, endpoint);
    }

    pub async fn get_endpoint(&self, id: Uuid) -> Option<WebhookEndpoint> {
        self.endpoints.read().await.get(&id).cloned()
    }

    pub async fn list_endpoints(&self) -> Vec<WebhookEndpoint> {
        self.endpoints.read().await.values().cloned().collect()
    }

    pub async fn record_delivery(&self, delivery: WebhookDelivery) {
        self.deliveries.write().await.push(delivery);
    }

    pub async fn list_deliveries(&self) -> Vec<WebhookDelivery> {
        self.deliveries.read().await.clone()
    }

    pub async fn insert_dead_letter(&self, record: WebhookDeadLetter) {
        self.dead_letters.write().await.insert(record.id, record);
    }

    pub async fn list_dead_letters(&self) -> Vec<WebhookDeadLetter> {
        self.dead_letters.read().await.values().cloned().collect()
    }

    pub async fn get_dead_letter(&self, id: Uuid) -> Option<WebhookDeadLetter> {
        self.dead_letters.read().await.get(&id).cloned()
    }

    pub async fn remove_dead_letter(&self, id: Uuid) {
        self.dead_letters.write().await.remove(&id);
    }
}

#[derive(Debug, Clone)]
pub struct WebhookDispatcher {
    tx: mpsc::Sender<WebhookEvent>,
}

#[derive(Debug, Clone)]
struct WebhookEvent {
    event_type: WebhookEventType,
    contract_id: Option<String>,
    payload: Value,
}

impl WebhookDispatcher {
    pub fn spawn(store: Arc<WebhookStore>) -> Self {
        let (tx, rx) = mpsc::channel(512);
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client build");

        tokio::spawn(dispatch_loop(rx, store, client));
        Self { tx }
    }

    pub async fn emit(
        &self,
        event_type: WebhookEventType,
        contract_id: Option<String>,
        payload: Value,
    ) {
        let _ = self
            .tx
            .send(WebhookEvent {
                event_type,
                contract_id,
                payload,
            })
            .await;
    }
}

async fn dispatch_loop(mut rx: mpsc::Receiver<WebhookEvent>, store: Arc<WebhookStore>, client: Client) {
    while let Some(event) = rx.recv().await {
        let endpoints = store.list_endpoints().await;
        for endpoint in endpoints {
            if !endpoint.active {
                continue;
            }
            if !endpoint.subscribed_events.contains(&event.event_type) {
                continue;
            }
            if let Some(ref endpoint_contract_id) = endpoint.contract_id {
                if event.contract_id.as_deref() != Some(endpoint_contract_id.as_str()) {
                    continue;
                }
            }

            let store = Arc::clone(&store);
            let client = client.clone();
            tokio::spawn(async move {
                deliver_with_retries(client, store, endpoint, event).await;
            });
        }
    }
}

async fn deliver_with_retries(
    client: Client,
    store: Arc<WebhookStore>,
    endpoint: WebhookEndpoint,
    event: WebhookEvent,
) {
    let mut attempt: u32 = 0;
    let mut last_status: Option<u16> = None;
    let mut last_body: Option<String> = None;

    loop {
        let (status, body, should_retry) =
            deliver_once(&client, &endpoint, &event, attempt).await;

        attempt = attempt.saturating_add(1);
        last_status = status;
        last_body = body.clone();

        let success = matches!(status, Some(code) if (200..300).contains(&code));
        store
            .record_delivery(WebhookDelivery {
                id: Uuid::new_v4(),
                webhook_id: endpoint.id,
                event_type: event.event_type.clone(),
                payload: event.payload.clone(),
                status_code: status,
                response_body: body,
                success,
                retry_attempts: attempt.saturating_sub(1),
                timestamp: Utc::now(),
            })
            .await;

        if success {
            return;
        }

        if attempt > endpoint.retry_count || !should_retry {
            let dead = WebhookDeadLetter {
                id: Uuid::new_v4(),
                webhook_id: endpoint.id,
                event_type: event.event_type.clone(),
                payload: event.payload.clone(),
                last_status_code: last_status,
                last_response_body: last_body,
                retry_attempts: attempt.saturating_sub(1),
                timestamp: Utc::now(),
            };
            store.insert_dead_letter(dead).await;
            return;
        }

        let delay = backoff_delay(attempt);
        tokio::time::sleep(delay).await;
    }
}

async fn deliver_once(
    client: &Client,
    endpoint: &WebhookEndpoint,
    event: &WebhookEvent,
    attempt: u32,
) -> (Option<u16>, Option<String>, bool) {
    let payload = json!({
        "id": Uuid::new_v4(),
        "type": event.event_type.as_str(),
        "timestamp": Utc::now().to_rfc3339(),
        "contract_id": event.contract_id,
        "attempt": attempt,
        "data": event.payload,
    });

    let body_bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(_) => return (None, Some("payload serialize failed".to_string()), false),
    };

    let ts = now_epoch_seconds();
    let sig = sign_payload(&endpoint.secret, ts, &body_bytes);

    let res = client
        .post(&endpoint.target_url)
        .header("content-type", "application/json")
        .header("x-webhook-timestamp", ts.to_string())
        .header("x-webhook-signature", sig)
        .body(body_bytes)
        .send()
        .await;

    match res {
        Ok(r) => {
            let status = r.status().as_u16();
            let body = r.text().await.ok();
            let should_retry = status >= 500;
            (Some(status), body, should_retry)
        }
        Err(e) => (None, Some(e.to_string()), true),
    }
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as i64
}

fn sign_payload(secret: &[u8], timestamp: i64, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

fn backoff_delay(attempt: u32) -> Duration {
    let seconds = 1u64.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
    Duration::from_secs(seconds.min(30))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWebhookRequest {
    pub target_url: String,
    pub subscribed_events: Vec<WebhookEventType>,
    pub contract_id: Option<String>,
    pub secret: Option<String>,
    pub retry_count: Option<u32>,
}

#[utoipa::path(
    post,
    path = "/webhooks",
    summary = "Register webhook",
    description = "Registers a webhook endpoint. If `secret` is omitted, a random base64 secret is generated and returned once.",
    tag = "Webhooks",
    request_body = CreateWebhookRequest,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 201, description = "Webhook created", body = WebhookEndpointCreated),
        (status = 400, description = "Invalid request", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn create_webhook(
    State(state): State<AppState>,
    Json(payload): Json<CreateWebhookRequest>,
) -> impl IntoResponse {
    let mut secret_bytes = vec![0u8; 32];
    let secret = match payload.secret {
        Some(s) => s,
        None => {
            rand::thread_rng().fill_bytes(&mut secret_bytes);
            BASE64.encode(secret_bytes)
        }
    };

    let decoded = match BASE64.decode(secret.as_bytes()) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "InvalidSecret",
                    "message": "secret must be base64",
                    "code": 400,
                    "timestamp": Utc::now().to_rfc3339(),
                    "correlation_id": Uuid::new_v4().to_string()
                })),
            )
                .into_response()
        }
    };

    let endpoint = WebhookEndpoint {
        id: Uuid::new_v4(),
        target_url: payload.target_url,
        subscribed_events: payload.subscribed_events,
        contract_id: payload.contract_id,
        secret: decoded,
        active: true,
        retry_count: payload.retry_count.unwrap_or(5).clamp(0, 20),
        created_at: Utc::now(),
    };

    state.webhook_store.insert_endpoint(endpoint.clone()).await;

    (
        StatusCode::CREATED,
        Json(WebhookEndpointCreated {
            webhook: WebhookEndpointPublic::from(&endpoint),
            secret,
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct ListWebhooksQuery {
    pub event: Option<WebhookEventType>,
    pub contract_id: Option<String>,
    pub active: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/webhooks",
    summary = "List webhooks",
    description = "Lists webhook endpoints with optional filtering.",
    tag = "Webhooks",
    params(
        ("event" = Option<WebhookEventType>, Query, description = "Filter by event type"),
        ("contract_id" = Option<String>, Query, description = "Filter by contract id"),
        ("active" = Option<bool>, Query, description = "Filter by active flag")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Webhooks", body = [WebhookEndpointPublic]),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn list_webhooks(
    State(state): State<AppState>,
    Query(query): Query<ListWebhooksQuery>,
) -> impl IntoResponse {
    let mut endpoints: Vec<WebhookEndpointPublic> = state
        .webhook_store
        .list_endpoints()
        .await
        .iter()
        .filter(|e| match query.active {
            Some(active) => e.active == active,
            None => true,
        })
        .filter(|e| match query.contract_id.as_deref() {
            Some(contract_id) => e.contract_id.as_deref() == Some(contract_id),
            None => true,
        })
        .filter(|e| match query.event.as_ref() {
            Some(ev) => e.subscribed_events.contains(ev),
            None => true,
        })
        .map(WebhookEndpointPublic::from)
        .collect();

    endpoints.sort_by_key(|e| e.created_at);
    Json(endpoints).into_response()
}

#[utoipa::path(
    get,
    path = "/webhooks/{id}",
    summary = "Get webhook",
    description = "Returns a webhook endpoint by id.",
    tag = "Webhooks",
    params(
        ("id" = Uuid, Path, description = "Webhook id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Webhook", body = WebhookEndpointPublic),
        (status = 404, description = "Not found", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_webhook(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match state.webhook_store.get_endpoint(id).await {
        Some(endpoint) => Json(WebhookEndpointPublic::from(&endpoint)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"WebhookNotFound","message":"Webhook not found","code":404,"timestamp":Utc::now().to_rfc3339(),"correlation_id":Uuid::new_v4().to_string()})),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListDeliveriesQuery {
    pub webhook_id: Option<Uuid>,
    pub success: Option<bool>,
    pub event: Option<WebhookEventType>,
}

#[utoipa::path(
    get,
    path = "/webhooks/deliveries",
    summary = "Delivery history",
    description = "Lists webhook delivery attempts.",
    tag = "Webhooks",
    params(
        ("webhook_id" = Option<Uuid>, Query, description = "Filter by webhook id"),
        ("success" = Option<bool>, Query, description = "Filter by success flag"),
        ("event" = Option<WebhookEventType>, Query, description = "Filter by event type")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Deliveries", body = [WebhookDelivery]),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn list_deliveries(
    State(state): State<AppState>,
    Query(query): Query<ListDeliveriesQuery>,
) -> impl IntoResponse {
    let mut deliveries = state.webhook_store.list_deliveries().await;
    deliveries.retain(|d| match query.webhook_id {
        Some(id) => d.webhook_id == id,
        None => true,
    });
    deliveries.retain(|d| match query.success {
        Some(success) => d.success == success,
        None => true,
    });
    deliveries.retain(|d| match query.event.as_ref() {
        Some(ev) => &d.event_type == ev,
        None => true,
    });
    deliveries.sort_by_key(|d| d.timestamp);
    Json(deliveries).into_response()
}

#[utoipa::path(
    get,
    path = "/webhooks/dead-letter",
    summary = "Dead letter queue",
    description = "Lists permanently failed webhook deliveries.",
    tag = "Webhooks",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Dead letters", body = [WebhookDeadLetter]),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn list_dead_letters(State(state): State<AppState>) -> impl IntoResponse {
    let mut dead = state.webhook_store.list_dead_letters().await;
    dead.sort_by_key(|d| d.timestamp);
    Json(dead).into_response()
}

#[utoipa::path(
    post,
    path = "/webhooks/dead-letter/{id}/retry",
    summary = "Retry dead letter",
    description = "Requeues a dead letter delivery for retry.",
    tag = "Webhooks",
    params(
        ("id" = Uuid, Path, description = "Dead letter record id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 202, description = "Queued for retry", body = Value),
        (status = 404, description = "Not found", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn retry_dead_letter(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let record = match state.webhook_store.get_dead_letter(id).await {
        Some(r) => r,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error":"DeadLetterNotFound","message":"Dead letter record not found"})),
            )
                .into_response()
        }
    };

    let endpoint = match state.webhook_store.get_endpoint(record.webhook_id).await {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error":"WebhookNotFound","message":"Webhook not found"})),
            )
                .into_response()
        }
    };

    state.webhook_store.remove_dead_letter(id).await;
    state
        .webhook_dispatcher
        .emit(
            record.event_type,
            None,
            json!({ "replayed": true, "payload": record.payload }),
        )
        .await;

    (
        StatusCode::ACCEPTED,
        Json(json!({"queued": true, "webhook_id": endpoint.id})),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::post, Router};
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[tokio::test]
    async fn signs_payload_deterministically() {
        let secret = b"secret";
        let ts = 123;
        let body = br#"{"a":1}"#;
        let sig = sign_payload(secret, ts, body);
        assert_eq!(
            sig,
            "886b2cfb33929d95bb4506427b76d7448911b6c3a99c63c33a2a9ed0e218935b"
        );
    }

    #[tokio::test]
    async fn retries_then_succeeds() {
        let hits = Arc::new(AtomicU64::new(0));
        let hits_clone = Arc::clone(&hits);
        let app = Router::new().route(
            "/hook",
            post(move || {
                let hits = Arc::clone(&hits_clone);
                async move {
                    let n = hits.fetch_add(1, Ordering::Relaxed);
                    if n < 2 {
                        StatusCode::INTERNAL_SERVER_ERROR
                    } else {
                        StatusCode::OK
                    }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service()).await.unwrap();
        });

        let store = Arc::new(WebhookStore::default());
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let endpoint = WebhookEndpoint {
            id: Uuid::new_v4(),
            target_url: format!("http://{}/hook", addr),
            subscribed_events: vec![WebhookEventType::ContractVerified],
            contract_id: None,
            secret: b"secret".to_vec(),
            active: true,
            retry_count: 3,
            created_at: Utc::now(),
        };
        let event = WebhookEvent {
            event_type: WebhookEventType::ContractVerified,
            contract_id: None,
            payload: json!({"ok":true}),
        };

        deliver_with_retries(client, Arc::clone(&store), endpoint, event).await;
        let deliveries = store.list_deliveries().await;
        assert!(deliveries.iter().any(|d| d.success));
        assert!(hits.load(Ordering::Relaxed) >= 3);
    }

    #[tokio::test]
    async fn moves_to_dead_letter_after_retries() {
        let app = Router::new().route("/hook", post(|| async { StatusCode::INTERNAL_SERVER_ERROR }));
        let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service()).await.unwrap();
        });

        let store = Arc::new(WebhookStore::default());
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let endpoint = WebhookEndpoint {
            id: Uuid::new_v4(),
            target_url: format!("http://{}/hook", addr),
            subscribed_events: vec![WebhookEventType::ContractVerified],
            contract_id: None,
            secret: b"secret".to_vec(),
            active: true,
            retry_count: 1,
            created_at: Utc::now(),
        };
        let event = WebhookEvent {
            event_type: WebhookEventType::ContractVerified,
            contract_id: None,
            payload: json!({"ok":true}),
        };

        deliver_with_retries(client, Arc::clone(&store), endpoint, event).await;
        let dead = store.list_dead_letters().await;
        assert_eq!(dead.len(), 1);
    }
}

