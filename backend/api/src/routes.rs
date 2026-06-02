use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::{
    api_versioning::{self, ApiVersion, ApiVersionMiddlewareState, ApiVersionMetrics},
    breaking_changes, deprecation_handlers, handlers,
    metrics_handler,
    state::AppState,
    webhooks,
};

pub fn observability_routes() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler::metrics_endpoint))
}

pub fn health_routes() -> Router<AppState> {
    Router::new().route("/health", get(handlers::health_check))
}

pub fn api_router(metrics: Arc<ApiVersionMetrics>) -> Router<AppState> {
    let v1 = api_v1_routes().layer(middleware::from_fn_with_state(
        ApiVersionMiddlewareState {
            metrics: Arc::clone(&metrics),
            version: ApiVersion::V1,
            deprecated_alias: false,
            sunset: None,
        },
        api_versioning::version_middleware,
    ));

    let v2 = api_v2_routes().layer(middleware::from_fn_with_state(
        ApiVersionMiddlewareState {
            metrics: Arc::clone(&metrics),
            version: ApiVersion::V2,
            deprecated_alias: false,
            sunset: None,
        },
        api_versioning::version_middleware,
    ));

    let alias = api_v1_routes().layer(middleware::from_fn_with_state(
        ApiVersionMiddlewareState {
            metrics,
            version: ApiVersion::V1,
            deprecated_alias: true,
            sunset: Some("Wed, 31 Dec 2026 00:00:00 GMT"),
        },
        api_versioning::version_middleware,
    ));

    Router::new()
        .nest("/api/v1", v1)
        .nest("/api/v2", v2)
        .nest("/api", alias)
}

fn api_v1_routes() -> Router<AppState> {
    Router::new()
        .merge(contract_routes())
        .merge(publisher_routes())
        .merge(webhook_routes())
        .route("/stats", get(handlers::get_stats))
}

fn api_v2_routes() -> Router<AppState> {
    Router::new()
        .merge(contract_routes())
        .merge(publisher_routes())
        .merge(webhook_routes())
        .route("/stats", get(handlers::get_stats))
}

fn contract_routes() -> Router<AppState> {
    Router::new()
        .route("/contracts", get(handlers::list_contracts))
        .route("/contracts", post(handlers::publish_contract))
        .route("/contracts/trending", get(handlers::get_trending_contracts))
        .route("/contracts/graph", get(handlers::get_contract_graph))
        .route("/contracts/breaking-changes", get(breaking_changes::get_breaking_changes))
        .route("/contracts/verify", post(handlers::verify_contract))
        .route("/contracts/:id", get(handlers::get_contract))
        .route("/contracts/:id/abi", get(handlers::get_contract_abi))
        .route(
            "/contracts/:id/versions",
            get(handlers::get_contract_versions).post(handlers::create_contract_version),
        )
        .route(
            "/contracts/:id/deprecation-info",
            get(deprecation_handlers::get_deprecation_info),
        )
        .route(
            "/contracts/:id/deprecate",
            post(deprecation_handlers::deprecate_contract),
        )
        .route(
            "/contracts/:id/state/:key",
            get(handlers::get_contract_state).post(handlers::update_contract_state),
        )
        .route("/contracts/:id/analytics", get(handlers::get_contract_analytics))
        .route("/contracts/:id/trust-score", get(handlers::get_trust_score))
        .route(
            "/contracts/:id/dependencies",
            get(handlers::get_contract_dependencies),
        )
        .route(
            "/contracts/:id/dependents",
            get(handlers::get_contract_dependents),
        )
        .route(
            "/contracts/:id/performance",
            get(handlers::get_contract_performance),
        )
        .route(
            "/contracts/:id/deployments/status",
            get(handlers::get_deployment_status),
        )
        .route("/deployments/green", post(handlers::deploy_green))
}

fn publisher_routes() -> Router<AppState> {
    Router::new()
        .route("/publishers", post(handlers::create_publisher))
        .route("/publishers/:id", get(handlers::get_publisher))
        .route("/publishers/:id/contracts", get(handlers::get_publisher_contracts))
}

fn webhook_routes() -> Router<AppState> {
    Router::new()
        .route("/webhooks", post(webhooks::create_webhook).get(webhooks::list_webhooks))
        .route("/webhooks/deliveries", get(webhooks::list_deliveries))
        .route("/webhooks/dead-letter", get(webhooks::list_dead_letters))
        .route(
            "/webhooks/dead-letter/:id/retry",
            post(webhooks::retry_dead_letter),
        )
        .route("/webhooks/:id", get(webhooks::get_webhook))
}

pub fn migration_routes() -> Router<AppState> {
    Router::new()
}

pub fn canary_routes() -> Router<AppState> {
    Router::new()
}
pub fn ab_test_routes() -> Router<AppState> {
    Router::new()
}
pub fn performance_routes() -> Router<AppState> {
    Router::new()
}
