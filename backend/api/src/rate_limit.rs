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
//! ## Write-endpoint protection
//!
//! Mutation endpoints (POST, PUT, PATCH, DELETE) consume more server resources
//! and can cause irreversible state changes.  They are subject to a separate,
//! tighter quota:
//!
//! - Anonymous write limit: 100 req/hour  (`RATE_LIMIT_WRITE_ANON_PER_HOUR`)
//! - Authenticated write limit: 300 req/hour (`RATE_LIMIT_WRITE_AUTH_PER_HOUR`)
//!
//! ## Exempt paths
//!
//! Health probes (`/health*`), the Prometheus metrics scrape endpoint
//! (`/metrics`), and internal admin flows (`/api/admin/*`) bypass the rate
//! limiter so monitoring systems and operators are never blocked.
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

// Read limits — 1,000 requests per hour per client (issue #609).
const DEFAULT_ANON_LIMIT: u32 = 1_000;
const DEFAULT_AUTH_LIMIT: u32 = 1_000;

// Write limits — tighter quota for mutation endpoints.
const DEFAULT_WRITE_ANON_LIMIT: u32 = 100;
const DEFAULT_WRITE_AUTH_LIMIT: u32 = 300;

const DEFAULT_WINDOW_SECONDS: u64 = 3_600; // 1 hour
#[allow(dead_code)]
const DEFAULT_CONTRACTS_PAGE_SIZE: u32 = 50;
#[allow(dead_code)]
const MAX_CONTRACTS_PAGE_SIZE: u32 = 1000;
#[allow(dead_code)]
const ENDPOINT_LIMIT_ENV_PREFIX: &str = "RATE_LIMIT_ENDPOINT_";

/// How often the background task sweeps for expired buckets.
const EVICTION_INTERVAL: Duration = Duration::from_secs(5 * 60); // every 5 minutes

const HEADER_RATE_LIMIT_LIMIT: HeaderName = HeaderName::from_static("x-ratelimit-limit");
const HEADER_RATE_LIMIT_REMAINING: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
const HEADER_RATE_LIMIT_RESET: HeaderName = HeaderName::from_static("x-ratelimit-reset");

/// Paths that bypass rate limiting entirely.
///
/// Health probes must not be rate-limited so that load balancers and
/// orchestrators always get a response.  The Prometheus scrape endpoint
/// `/metrics` and internal admin APIs are also exempt.
fn is_exempt_path(path: &str) -> bool {
    path.starts_with("/health")
        || path == "/metrics"
        || path.starts_with("/api/admin/")
}

#[derive(Clone)]
pub struct RateLimitState {
    config: std::sync::Arc<RateLimitConfig>,
    /// Shared bucket map — protected by a *tokio* Mutex so it is async-safe.
    buckets: std::sync::Arc<Mutex<HashMap<BucketKey, BucketState>>>,
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
                    state
                        .timestamps
                        .back()
                        .map(|last_seen| {
                            now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                        })
                        .unwrap_or(false)
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

    async fn check_request(&self, key: BucketKey, limit: u32) -> RateLimitDecision {
        let now = Instant::now();

        // tokio::sync::Mutex::lock() never poisons — no .expect() needed.
        let mut buckets = self.buckets.lock().await;

        let bucket = buckets.entry(key).or_insert_with(|| BucketState {
            timestamps: VecDeque::new(),
        });

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

        let reset_seconds = bucket
            .timestamps
            .front()
            .and_then(|oldest| oldest.checked_add(self.config.window))
            .map(|expiry| ceil_duration_to_seconds(expiry.saturating_duration_since(now)).max(1))
            .unwrap_or_else(|| ceil_duration_to_seconds(self.config.window).max(1));

        if (bucket.timestamps.len() as u32) >= limit {
            return RateLimitDecision {
                allowed: false,
                limit,
                remaining: 0,
                reset_seconds,
            };
        }

        bucket.timestamps.push_back(now);
        let remaining = limit.saturating_sub(bucket.timestamps.len() as u32);

        RateLimitDecision {
            allowed: true,
            limit,
            remaining,
            reset_seconds,
        }
    }

    fn select_limit_and_key<B>(&self, request: &Request<B>) -> (u32, BucketKey) {
        let method = request.method();
        let path = request.uri().path();
        let query = request.uri().query();
        let is_write = is_write_method(method);

        if let Some(token) = extract_auth_token(request) {
            let limit = if is_write {
                self.config.write_auth_limit
            } else {
                self.config.auth_limit
            };
            let bucket_suffix = if is_write { "write" } else { "read" };
            return (
                limit,
                BucketKey {
                    client_key: format!("auth:{bucket_suffix}:{token}"),
                },
            );
        }

        let ip = extract_client_ip(request);
        if is_write {
            return (
                self.config.write_anonymous_limit,
                BucketKey {
                    client_key: format!("anon:write:{ip}"),
                },
            );
        }

        let base_limit = self.config.anonymous_limit;
        let limit = if let Some(page_size) =
            contracts_page_size_rate_limit(method, path, query)
        {
            scale_limit_by_page_size(base_limit, page_size)
        } else {
            base_limit
        };

        (
            limit,
            BucketKey {
                client_key: format!("anon:read:{ip}"),
            },
        )
    }
}

struct RateLimitConfig {
    anonymous_limit: u32,
    auth_limit: u32,
    write_anonymous_limit: u32,
    write_auth_limit: u32,
    window: Duration,
}

impl RateLimitConfig {
    fn from_env() -> Self {
        // Read limits — configurable per-IP; defaults to 1000 req/hour (issue #609).
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
        // Write limits — stricter quota for mutation endpoints.
        let write_anonymous_limit = env_u32(
            "RATE_LIMIT_WRITE_ANON_PER_HOUR",
            DEFAULT_WRITE_ANON_LIMIT,
        );
        let write_auth_limit = env_u32(
            "RATE_LIMIT_WRITE_AUTH_PER_HOUR",
            DEFAULT_WRITE_AUTH_LIMIT,
        );
        let window_seconds = env_u64("RATE_LIMIT_WINDOW_SECONDS", DEFAULT_WINDOW_SECONDS).max(1);

        tracing::info!(
            anonymous_limit,
            auth_limit,
            write_anonymous_limit,
            write_auth_limit,
            window_seconds,
            "Rate limiter configured"
        );

        Self {
            anonymous_limit,
            auth_limit,
            write_anonymous_limit,
            write_auth_limit,
            window: Duration::from_secs(window_seconds),
        }
    }

    #[cfg(test)]
    fn for_tests(anonymous_limit: u32, auth_limit: u32, window: Duration) -> Self {
        Self {
            anonymous_limit,
            auth_limit,
            write_anonymous_limit: anonymous_limit / 10,
            write_auth_limit: auth_limit / 3,
            window,
        }
    }

    #[cfg(test)]
    fn for_tests_with_write(
        anonymous_limit: u32,
        auth_limit: u32,
        write_anonymous_limit: u32,
        write_auth_limit: u32,
        window: Duration,
    ) -> Self {
        Self {
            anonymous_limit,
            auth_limit,
            write_anonymous_limit,
            write_auth_limit,
            window,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
struct BucketKey {
    client_key: String,
}

struct BucketState {
    timestamps: VecDeque<Instant>,
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
    // Health probes, metrics scrapes, and admin paths bypass rate limiting.
    if is_exempt_path(request.uri().path()) {
        return next.run(request).await;
    }

    // Extract request metadata before awaiting to avoid borrowing `request` across `.await`.
    let (limit, key) = rate_limiter.select_limit_and_key(&request);
    let decision = rate_limiter.check_request(key, limit).await;

    if !decision.allowed {
        let is_write = is_write_method(request.method());
        let detail = if is_write {
            "Write quota exhausted. Reduce request frequency or wait for the window to reset."
        } else {
            "Too many requests. Please retry after the indicated time."
        };
        let mut response = ApiError::rate_limited(detail)
            .with_details(serde_json::json!({
                "retry_after_seconds": decision.reset_seconds,
                "limit_type": if is_write { "write" } else { "read" }
            }))
            .into_response();
        attach_rate_limit_headers(&mut response, &decision);
        response.headers_mut().insert(
            RETRY_AFTER,
            HeaderValue::from_str(&decision.reset_seconds.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1")),
        );
        return response;
    }

    let mut response = next.run(request).await;
    attach_rate_limit_headers(&mut response, &decision);
    response
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
        let (limit, key) = state.select_limit_and_key(&req);
        state.check_request(key, limit).await;

        // Confirm one bucket exists
        assert_eq!(state.buckets.lock().await.len(), 1);

        // Wait for more than two window lengths so the bucket qualifies for eviction
        tokio::time::sleep(window.saturating_mul(3)).await;

        // Run eviction manually (same logic as the background task)
        {
            let now = Instant::now();
            let mut map = state.buckets.lock().await;
            map.retain(|_, s| {
                s.timestamps
                    .back()
                    .map(|last_seen| {
                        now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                    })
                    .unwrap_or(false)
            });
        }

        assert_eq!(state.buckets.lock().await.len(), 0);
    }

    // ── Write endpoint protection tests ──────────────────────────────────────

    fn test_app_with_write(
        read_limit: u32,
        write_limit: u32,
        window: Duration,
    ) -> Router<()> {
        let limiter = RateLimitState::new(RateLimitConfig::for_tests_with_write(
            read_limit,
            read_limit,
            write_limit,
            write_limit,
            window,
        ));

        Router::new()
            .route("/read", get(|| async { "read" }))
            .route("/write", post(|| async { "written" }))
            .route("/health", get(|| async { "ok" }))
            .route("/health/ready", get(|| async { "ok" }))
            .route("/metrics", get(|| async { "# metrics" }))
            .route("/api/admin/audit-logs", get(|| async { "[]" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ))
    }

    #[tokio::test]
    async fn write_requests_use_tighter_limit_than_reads() {
        // Read limit: 100, write limit: 2. After 2 POSTs the third is blocked.
        // GETs should still be allowed up to the read limit.
        let app = test_app_with_write(100, 2, Duration::from_secs(60));
        let ip = "198.51.100.99";

        // First two POSTs succeed.
        for _ in 0..2 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/write")
                    .method("POST")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "first two write requests should not be rate limited"
            );
        }

        // Third POST is blocked.
        let limited = call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(limited.headers().contains_key(RETRY_AFTER));

        // GET requests are still within their separate read quota.
        let read_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_ne!(read_response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn write_limit_response_body_contains_limit_type_write() {
        let app = test_app_with_write(100, 1, Duration::from_secs(60));
        let ip = "203.0.113.55";

        // Use up the single write slot.
        call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        let response = call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body =
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["details"]["limit_type"], "write");
    }

    #[tokio::test]
    async fn health_endpoint_is_not_rate_limited() {
        // Very tight limits — both read and write — to verify health bypasses.
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.1";

        // Send many requests to /health; none should be rate limited.
        for i in 0..20 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/health")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /health should not be rate limited"
            );
        }
    }

    #[tokio::test]
    async fn health_ready_endpoint_is_exempt() {
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.2";

        for i in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/health/ready")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /health/ready should not be rate limited"
            );
        }
    }

    #[tokio::test]
    async fn metrics_endpoint_is_exempt() {
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.3";

        for i in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/metrics")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /metrics should not be rate limited"
            );
        }
    }

    #[tokio::test]
    async fn admin_endpoint_is_exempt_from_rate_limiting() {
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.4";

        for i in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/api/admin/audit-logs")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /api/admin/* should not be rate limited"
            );
        }
    }

    #[test]
    fn is_exempt_path_recognises_health_metrics_and_admin() {
        assert!(is_exempt_path("/health"));
        assert!(is_exempt_path("/health/live"));
        assert!(is_exempt_path("/health/ready"));
        assert!(is_exempt_path("/health/detailed"));
        assert!(is_exempt_path("/metrics"));
        assert!(is_exempt_path("/api/admin/migrations/status"));
        assert!(!is_exempt_path("/api/contracts"));
        assert!(!is_exempt_path("/api/contracts/verify"));
        assert!(!is_exempt_path("/api/publishers"));
    }
}
