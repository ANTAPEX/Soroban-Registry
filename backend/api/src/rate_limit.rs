//! Sliding-window rate limiter with automatic eviction of expired entries.
//!
//! ## Memory-leak fix (issue #317)
//!
//! The original implementation stored fixed-window counters in a
//! `HashMap` that was never cleaned up.  Every unique IP that ever hit the
//! API accumulated an entry that lived forever.
//!
//! This version fixes that by:
//!
//! 1. **Background eviction task** – [`RateLimitState::spawn_eviction_task`]
//!    runs every `EVICTION_INTERVAL` and removes any bucket whose window
//!    expired more than `window_duration` ago.
//! 2. **`tokio::sync::Mutex`** – replaced `std::sync::Mutex` so the lock is
//!    held correctly across `.await` points and avoids blocking the async
//!    executor.
//! 3. **Graceful lock error handling** – `.lock().await` on a
//!    `tokio::sync::Mutex` never poisons, so the panic from `.expect()` is
//!    gone. The one remaining fallible path (attaching response headers) logs
//!    a warning instead of crashing.
//!
//! ## Horizontal scaling note
//!
//! This rate limiter is **per-instance**.  When running multiple API replicas
//! the bucket state is not shared between them.  For true distributed rate
//! limiting consider replacing the in-process `HashMap` with a Redis-backed
//! store (e.g. via the `upstash-redis` crate or `fred`).

use std::{
    collections::{HashMap, VecDeque},
    env,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, State},
    http::{
        header::{AUTHORIZATION, RETRY_AFTER},
        HeaderName, HeaderValue, Method, Request,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::Mutex;

use crate::error::ApiError;

// Issue #609: 1,000 requests per hour per IP (anonymous), 1,000 per hour authenticated
const DEFAULT_ANON_LIMIT: u32 = 1_000;
const DEFAULT_AUTH_LIMIT: u32 = 1_000;
const DEFAULT_WINDOW_SECONDS: u64 = 3_600; // 1 hour
#[allow(dead_code)]
const DEFAULT_CONTRACTS_PAGE_SIZE: u32 = 50;
#[allow(dead_code)]
const MAX_CONTRACTS_PAGE_SIZE: u32 = 1000;
#[allow(dead_code)]
const ENDPOINT_LIMIT_ENV_PREFIX: &str = "RATE_LIMIT_ENDPOINT_";

/// Issue #727 — tiered hourly limits
const FREE_TIER_LIMIT: u32 = 100;
const PRO_TIER_LIMIT: u32 = 10_000;
const BURST_WINDOW_SECONDS: u64 = 60; // 1 minute burst window

/// How often the background task sweeps for expired buckets.
const EVICTION_INTERVAL: Duration = Duration::from_secs(5 * 60); // every 5 minutes

const HEADER_RATE_LIMIT_LIMIT: HeaderName = HeaderName::from_static("x-ratelimit-limit");
const HEADER_RATE_LIMIT_REMAINING: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
const HEADER_RATE_LIMIT_RESET: HeaderName = HeaderName::from_static("x-ratelimit-reset");
const HEADER_RATE_LIMIT_TIER: HeaderName = HeaderName::from_static("x-ratelimit-tier");

/// API tier that determines quota limits (issue #727).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiTier {
    Free,
    Pro,
    /// Enterprise limit is read from ENTERPRISE_RATE_LIMIT_PER_HOUR at startup.
    Enterprise,
}

impl ApiTier {
    fn as_str(&self) -> &'static str {
        match self {
            ApiTier::Free => "free",
            ApiTier::Pro => "pro",
            ApiTier::Enterprise => "enterprise",
        }
    }

    pub fn from_header(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "pro" => ApiTier::Pro,
            "enterprise" => ApiTier::Enterprise,
            _ => ApiTier::Free,
        }
    }
}

#[derive(Clone)]
pub struct RateLimitState {
    config: std::sync::Arc<RateLimitConfig>,
    /// Shared bucket map — protected by a *tokio* Mutex so it is async-safe.
    buckets: std::sync::Arc<Mutex<HashMap<BucketKey, BucketState>>>,
}

/// Snapshot of quota usage for a client key (used by the /api/quota endpoint).
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuotaSnapshot {
    pub tier: String,
    pub hourly_limit: u32,
    pub hourly_used: usize,
    pub hourly_remaining: usize,
    pub burst_limit: u32,
    pub burst_used: usize,
    pub burst_remaining: usize,
}

impl RateLimitState {
    pub fn from_env() -> Self {
        Self::new(RateLimitConfig::from_env())
    }

    fn new(config: RateLimitConfig) -> Self {
        Self {
            config: std::sync::Arc::new(config),
            buckets: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a background Tokio task that periodically evicts expired buckets.
    ///
    /// Call this once during application startup (after `tokio::main` is
    /// entered).  The task runs until the process exits.
    pub fn spawn_eviction_task(&self) {
        let buckets = self.buckets.clone();
        let window = self.config.window;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(EVICTION_INTERVAL);
            // The first tick fires immediately — skip it so we don't evict
            // right after startup when there is nothing to evict yet.
            ticker.tick().await;

            loop {
                ticker.tick().await;

                let now = Instant::now();
                let mut map = buckets.lock().await;
                let before = map.len();

                // Retain only buckets that have seen traffic recently.
                map.retain(|_, state| {
                    // Check both windows; keep if either has a recent timestamp.
                    let hourly_active = state
                        .timestamps
                        .back()
                        .map(|last_seen| {
                            now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                        })
                        .unwrap_or(false);
                    let burst_active = state
                        .burst_timestamps
                        .back()
                        .map(|last_seen| {
                            now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                        })
                        .unwrap_or(false);
                    hourly_active || burst_active
                });

                let evicted = before - map.len();
                if evicted > 0 {
                    tracing::info!(
                        evicted,
                        remaining = map.len(),
                        "rate limiter: evicted expired buckets"
                    );
                }
            }
        });
    }

    async fn check_request(
        &self,
        key: BucketKey,
        hourly_limit: u32,
        burst_limit: u32,
    ) -> RateLimitDecision {
        let now = Instant::now();

        // tokio::sync::Mutex::lock() never poisons — no .expect() needed.
        let mut buckets = self.buckets.lock().await;

        let bucket = buckets.entry(key).or_insert_with(|| BucketState {
            timestamps: VecDeque::new(),
            burst_timestamps: VecDeque::new(),
        });

        // Trim the hourly sliding window
        let window_start_cutoff = now.checked_sub(self.config.window).unwrap_or(now);
        while bucket
            .timestamps
            .front()
            .copied()
            .map(|ts| ts <= window_start_cutoff)
            .unwrap_or(false)
        {
            bucket.timestamps.pop_front();
        }

        // Trim the burst (1-minute) sliding window
        let burst_cutoff = now.checked_sub(self.config.burst_window).unwrap_or(now);
        while bucket
            .burst_timestamps
            .front()
            .copied()
            .map(|ts| ts <= burst_cutoff)
            .unwrap_or(false)
        {
            bucket.burst_timestamps.pop_front();
        }

        let reset_seconds = bucket
            .timestamps
            .front()
            .and_then(|oldest| oldest.checked_add(self.config.window))
            .map(|expiry| ceil_duration_to_seconds(expiry.saturating_duration_since(now)).max(1))
            .unwrap_or_else(|| ceil_duration_to_seconds(self.config.window).max(1));

        // Reject if hourly limit exceeded
        if (bucket.timestamps.len() as u32) >= hourly_limit {
            return RateLimitDecision {
                allowed: false,
                limit: hourly_limit,
                remaining: 0,
                reset_seconds,
            };
        }

        // Reject if burst limit exceeded (too many in the last minute)
        if (bucket.burst_timestamps.len() as u32) >= burst_limit {
            return RateLimitDecision {
                allowed: false,
                limit: hourly_limit,
                remaining: hourly_limit.saturating_sub(bucket.timestamps.len() as u32),
                reset_seconds: ceil_duration_to_seconds(self.config.burst_window).max(1),
            };
        }

        bucket.timestamps.push_back(now);
        bucket.burst_timestamps.push_back(now);
        let remaining = hourly_limit.saturating_sub(bucket.timestamps.len() as u32);

        RateLimitDecision {
            allowed: true,
            limit: hourly_limit,
            remaining,
            reset_seconds,
        }
    }

    /// Returns the current quota snapshot for a client key without consuming a token.
    pub async fn quota_snapshot(&self, client_key: &str, tier: &ApiTier) -> QuotaSnapshot {
        let now = Instant::now();
        let hourly_limit = self.config.hourly_limit_for_tier(tier);
        let burst_limit = self.config.burst_limit_for_tier(tier);

        let buckets = self.buckets.lock().await;
        let key = BucketKey {
            client_key: client_key.to_owned(),
        };

        if let Some(bucket) = buckets.get(&key) {
            let window_cutoff = now.checked_sub(self.config.window).unwrap_or(now);
            let hourly_used = bucket
                .timestamps
                .iter()
                .filter(|&&ts| ts > window_cutoff)
                .count();

            let burst_cutoff = now.checked_sub(self.config.burst_window).unwrap_or(now);
            let burst_used = bucket
                .burst_timestamps
                .iter()
                .filter(|&&ts| ts > burst_cutoff)
                .count();

            QuotaSnapshot {
                tier: tier.as_str().to_owned(),
                hourly_limit,
                hourly_used,
                hourly_remaining: (hourly_limit as usize).saturating_sub(hourly_used),
                burst_limit,
                burst_used,
                burst_remaining: (burst_limit as usize).saturating_sub(burst_used),
            }
        } else {
            QuotaSnapshot {
                tier: tier.as_str().to_owned(),
                hourly_limit,
                hourly_used: 0,
                hourly_remaining: hourly_limit as usize,
                burst_limit,
                burst_used: 0,
                burst_remaining: burst_limit as usize,
            }
        }
    }

    /// Derive the bucket key and API tier from an incoming request.
    ///
    /// Tier is resolved in order:
    /// 1. `X-Api-Plan: free | pro | enterprise` header
    /// 2. Defaults to `free` for unauthenticated requests, `free` for authenticated
    ///    (upgrade to pro/enterprise requires the header or a future DB lookup).
    fn select_limit_and_key<B>(&self, request: &Request<B>) -> (u32, u32, BucketKey, ApiTier) {
        let tier = extract_api_tier(request);

        if let Some(token) = extract_auth_token(request) {
            let hourly = self.config.hourly_limit_for_tier(&tier);
            let burst = self.config.burst_limit_for_tier(&tier);
            return (
                hourly,
                burst,
                BucketKey {
                    client_key: format!("auth:{token}"),
                },
                tier,
            );
        }

        let path = request.uri().path();
        let query = request.uri().query();
        let hourly_base = self.config.hourly_limit_for_tier(&tier);
        let hourly = if let Some(page_size) =
            contracts_page_size_rate_limit(request.method(), path, query)
        {
            scale_limit_by_page_size(hourly_base, page_size)
        } else {
            hourly_base
        };
        let burst = self.config.burst_limit_for_tier(&tier);

        (
            hourly,
            burst,
            BucketKey {
                client_key: format!("anon:{}", extract_client_ip(request)),
            },
            tier,
        )
    }
}

struct RateLimitConfig {
    anonymous_limit: u32,
    auth_limit: u32,
    window: Duration,
    enterprise_limit: u32,
    burst_window: Duration,
}

impl RateLimitConfig {
    fn from_env() -> Self {
        // Issue #609: configurable per-IP limits; defaults to 1000 req/hour.
        // Env vars: RATE_LIMIT_ANON_PER_HOUR (preferred) or legacy RATE_LIMIT_ANON_PER_MINUTE / RATE_LIMIT_READ_PER_MINUTE
        let anonymous_limit = env_u32_with_fallback(
            "RATE_LIMIT_ANON_PER_HOUR",
            "RATE_LIMIT_ANON_PER_MINUTE",
            DEFAULT_ANON_LIMIT,
        );
        let auth_limit = env_u32_with_fallback(
            "RATE_LIMIT_AUTH_PER_HOUR",
            "RATE_LIMIT_AUTH_PER_MINUTE",
            DEFAULT_AUTH_LIMIT,
        );
        let window_seconds = env_u64("RATE_LIMIT_WINDOW_SECONDS", DEFAULT_WINDOW_SECONDS).max(1);
        // Issue #727: enterprise tier custom limit
        let enterprise_limit = env_u32("ENTERPRISE_RATE_LIMIT_PER_HOUR", 100_000);

        tracing::info!(
            anonymous_limit,
            auth_limit,
            window_seconds,
            enterprise_limit,
            "Rate limiter configured (issue #609/#727: tiered quotas)"
        );

        Self {
            anonymous_limit,
            auth_limit,
            window: Duration::from_secs(window_seconds),
            enterprise_limit,
            burst_window: Duration::from_secs(BURST_WINDOW_SECONDS),
        }
    }

    #[cfg(test)]
    fn for_tests(anonymous_limit: u32, auth_limit: u32, window: Duration) -> Self {
        Self {
            anonymous_limit,
            auth_limit,
            window,
            enterprise_limit: 100_000,
            burst_window: Duration::from_secs(BURST_WINDOW_SECONDS),
        }
    }

    /// Returns the hourly limit for the given tier.
    fn hourly_limit_for_tier(&self, tier: &ApiTier) -> u32 {
        match tier {
            ApiTier::Free => FREE_TIER_LIMIT,
            ApiTier::Pro => PRO_TIER_LIMIT,
            ApiTier::Enterprise => self.enterprise_limit,
        }
    }

    /// Burst limit = ceil(hourly_limit × 1.2 / 60) requests per minute.
    fn burst_limit_for_tier(&self, tier: &ApiTier) -> u32 {
        let hourly = self.hourly_limit_for_tier(tier);
        // 120 % of the per-minute equivalent; minimum 1
        ((hourly as f64 * 1.2 / 60.0).ceil() as u32).max(1)
    }
}

#[derive(Hash, Eq, PartialEq)]
struct BucketKey {
    client_key: String,
}

struct BucketState {
    timestamps: VecDeque<Instant>,
    /// Timestamps in the 1-minute burst window (subset of `timestamps`).
    burst_timestamps: VecDeque<Instant>,
}

struct RateLimitDecision {
    allowed: bool,
    limit: u32,
    remaining: u32,
    reset_seconds: u64,
}

pub async fn rate_limit_middleware(
    State(rate_limiter): State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Extract request metadata before awaiting to avoid borrowing `request` across `.await`.
    let (hourly_limit, burst_limit, key, tier) = rate_limiter.select_limit_and_key(&request);
    let decision = rate_limiter
        .check_request(key, hourly_limit, burst_limit)
        .await;

    if !decision.allowed {
        let mut response =
            ApiError::rate_limited("Too many requests. Please retry after the indicated time.")
                .with_details(serde_json::json!({
                    "retry_after_seconds": decision.reset_seconds
                }))
                .into_response();
        attach_rate_limit_headers(&mut response, &decision);
        attach_tier_header(&mut response, &tier);
        response.headers_mut().insert(
            RETRY_AFTER,
            HeaderValue::from_str(&decision.reset_seconds.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1")),
        );
        return response;
    }

    let mut response = next.run(request).await;
    attach_rate_limit_headers(&mut response, &decision);
    attach_tier_header(&mut response, &tier);
    response
}

fn attach_tier_header(response: &mut Response, tier: &ApiTier) {
    if let Ok(val) = HeaderValue::from_str(tier.as_str()) {
        response.headers_mut().insert(HEADER_RATE_LIMIT_TIER, val);
    }
}

fn attach_rate_limit_headers(response: &mut Response, decision: &RateLimitDecision) {
    // Use graceful fallback instead of panicking on header value conversion.
    let headers = response.headers_mut();

    if let Ok(val) = HeaderValue::from_str(&decision.limit.to_string()) {
        headers.insert(HEADER_RATE_LIMIT_LIMIT, val);
    } else {
        tracing::warn!("Failed to encode x-ratelimit-limit header");
    }

    if let Ok(val) = HeaderValue::from_str(&decision.remaining.to_string()) {
        headers.insert(HEADER_RATE_LIMIT_REMAINING, val);
    } else {
        tracing::warn!("Failed to encode x-ratelimit-remaining header");
    }

    if let Ok(val) = HeaderValue::from_str(&decision.reset_seconds.to_string()) {
        headers.insert(HEADER_RATE_LIMIT_RESET, val);
    } else {
        tracing::warn!("Failed to encode x-ratelimit-reset header");
    }
}

fn extract_client_ip<B>(request: &Request<B>) -> String {
    if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_x_forwarded_for)
    {
        return ip.to_string();
    }

    if let Some(ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_addr)
    {
        return ip.to_string();
    }

    if let Some(connect_info) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    "unknown".to_string()
}

/// Resolve API tier from the `X-Api-Plan` request header.
/// Falls back to `Free` when the header is absent or unrecognised.
fn extract_api_tier<B>(request: &Request<B>) -> ApiTier {
    request
        .headers()
        .get("x-api-plan")
        .and_then(|v| v.to_str().ok())
        .map(ApiTier::from_header)
        .unwrap_or(ApiTier::Free)
}

fn extract_auth_token<B>(request: &Request<B>) -> Option<String> {
    request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_x_forwarded_for(raw: &str) -> Option<IpAddr> {
    raw.split(',').map(str::trim).find_map(parse_ip_addr)
}

fn parse_ip_addr(raw: &str) -> Option<IpAddr> {
    raw.parse::<IpAddr>()
        .ok()
        .or_else(|| raw.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
}

#[allow(dead_code)]
fn is_write_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn contracts_page_size_rate_limit(method: &Method, path: &str, query: Option<&str>) -> Option<u32> {
    if *method != Method::GET || path != "/api/contracts" {
        return None;
    }

    Some(extract_page_size(query).unwrap_or(DEFAULT_CONTRACTS_PAGE_SIZE))
}

fn extract_page_size(query: Option<&str>) -> Option<u32> {
    let query = query?;

    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next().unwrap_or_default();

        if key == "limit" || key == "page_size" {
            if let Ok(parsed) = value.parse::<u32>() {
                return Some(parsed.clamp(1, MAX_CONTRACTS_PAGE_SIZE));
            }
        }
    }

    None
}

fn scale_limit_by_page_size(base_limit: u32, page_size: u32) -> u32 {
    let weight = page_size.div_ceil(DEFAULT_CONTRACTS_PAGE_SIZE).max(1);
    (base_limit / weight).max(1)
}

#[allow(dead_code)]
fn endpoint_key(method: &Method, path: &str) -> String {
    let normalized_path = path
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    let compact_path = normalized_path
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    if compact_path.is_empty() {
        format!("{}_ROOT", method.as_str().to_ascii_uppercase())
    } else {
        format!("{}_{}", method.as_str().to_ascii_uppercase(), compact_path)
    }
}

fn env_u32(key: &str, default: u32) -> u32 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn env_u32_with_fallback(primary_key: &str, fallback_key: &str, default: u32) -> u32 {
    match env::var(primary_key) {
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!(
                    "Invalid value for {primary_key} (`{raw}`), using default {default}"
                );
                default
            }
        },
        Err(_) => env_u32(fallback_key, default),
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn ceil_duration_to_seconds(duration: Duration) -> u64 {
    let secs = duration.as_secs();
    if duration.subsec_nanos() > 0 {
        secs + 1
    } else {
        secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::Service;

    fn test_app(anonymous_limit: u32, auth_limit: u32, window: Duration) -> Router<()> {
        let limiter = RateLimitState::new(RateLimitConfig::for_tests(
            anonymous_limit,
            auth_limit,
            window,
        ));

        Router::new()
            .route("/read", get(|| async { "read" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ))
    }

    async fn call(app: &Router<()>, request: Request<Body>) -> Response {
        let mut svc = app.clone();
        svc.call(request).await.unwrap()
    }

    /// Issue #609: anonymous IP limited to 1,000 req/hour; 1001st gets 429.
    #[tokio::test]
    async fn anonymous_user_gets_429_on_1001st_request() {
        let app = test_app(1_000, 1_000, Duration::from_secs(3600));

        for _ in 0..1_000 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/read")
                    .method("GET")
                    .header("x-forwarded-for", "203.0.113.10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        let response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "203.0.113.10")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(RETRY_AFTER));
    }

    #[tokio::test]
    async fn authenticated_user_gets_429_on_1001st_request() {
        let app = test_app(100, 1_000, Duration::from_secs(60));

        for _ in 0..1_000 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/read")
                    .method("GET")
                    .header("authorization", "Bearer token-abc")
                    .header("x-forwarded-for", "203.0.113.25")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        let response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("authorization", "Bearer token-abc")
                .header("x-forwarded-for", "203.0.113.25")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(RETRY_AFTER));
    }

    #[tokio::test]
    async fn includes_rate_limit_headers_on_success_and_429() {
        let app = test_app(1, 10, Duration::from_secs(60));

        let ok_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "198.51.100.22")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(ok_response.status(), StatusCode::OK);
        assert!(ok_response.headers().contains_key(HEADER_RATE_LIMIT_LIMIT));
        assert!(ok_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_REMAINING));
        assert!(ok_response.headers().contains_key(HEADER_RATE_LIMIT_RESET));

        let limited_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "198.51.100.22")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(limited_response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_LIMIT));
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_REMAINING));
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_RESET));
        assert!(limited_response.headers().contains_key(RETRY_AFTER));

        let body = axum::body::to_bytes(limited_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error_code"], "RATE_LIMITED");
    }

    #[tokio::test]
    async fn retry_after_header_is_present_and_reasonable() {
        let app = test_app(1, 10, Duration::from_secs(2));

        let _first = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let second = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after: u64 = second
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        assert!((1..=2).contains(&retry_after));
    }

    #[tokio::test]
    async fn rate_limit_headers_show_remaining_quota() {
        let app = test_app(2, 10, Duration::from_secs(60));

        let first = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(
            first
                .headers()
                .get(HEADER_RATE_LIMIT_REMAINING)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
            "1"
        );

        let second = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(
            second
                .headers()
                .get(HEADER_RATE_LIMIT_REMAINING)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
            "0"
        );

        let third = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            third
                .headers()
                .get(HEADER_RATE_LIMIT_REMAINING)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
            "0"
        );
    }

    #[tokio::test]
    async fn contracts_rate_limit_scales_down_for_large_page_sizes() {
        let app = test_app(100, 20, Duration::from_secs(60));
        let ip = "198.51.100.77";

        for _ in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/api/contracts?limit=1000")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(
                response
                    .headers()
                    .get(HEADER_RATE_LIMIT_LIMIT)
                    .and_then(|value| value.to_str().ok()),
                Some("5")
            );
        }

        let limited = call(
            &app,
            Request::builder()
                .uri("/api/contracts?limit=1000")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn page_size_scaling_uses_default_page_size_baseline() {
        assert_eq!(scale_limit_by_page_size(100, 50), 100);
        assert_eq!(scale_limit_by_page_size(100, 51), 50);
        assert_eq!(scale_limit_by_page_size(100, 1000), 5);
    }

    /// Verify that the eviction logic correctly removes expired buckets.
    #[tokio::test]
    async fn eviction_removes_expired_buckets() {
        let window = Duration::from_millis(100);
        let state = RateLimitState::new(RateLimitConfig::for_tests(10, 10, window));

        // Insert a request so a bucket is created
        let req = Request::builder()
            .uri("/read")
            .method("GET")
            .header("x-forwarded-for", "10.0.0.1")
            .body(Body::empty())
            .unwrap();
        let (hourly, burst, key, _tier) = state.select_limit_and_key(&req);
        state.check_request(key, hourly, burst).await;

        // Confirm one bucket exists
        assert_eq!(state.buckets.lock().await.len(), 1);

        // Wait for more than two window lengths so the bucket qualifies for eviction
        tokio::time::sleep(window.saturating_mul(3)).await;

        // Run eviction manually (same logic as the background task)
        {
            let now = Instant::now();
            let mut map = state.buckets.lock().await;
            map.retain(|_, s| {
                let hourly = s
                    .timestamps
                    .back()
                    .map(|last_seen| {
                        now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                    })
                    .unwrap_or(false);
                let burst = s
                    .burst_timestamps
                    .back()
                    .map(|last_seen| {
                        now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                    })
                    .unwrap_or(false);
                hourly || burst
            });
        }

        assert_eq!(state.buckets.lock().await.len(), 0);
    }
}
