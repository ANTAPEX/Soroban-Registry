#[cfg(feature = "openapi")]
use crate::openapi;
use crate::{
    ab_test_handlers, abi_versioning_handlers,
    ai::handlers as ai_handlers,
    analytics_handlers, archival, auth, auth_handlers, batch_verify_handlers, breaking_changes,
    bulk_operations_handlers, canary_handlers, category_handlers, client_observability_handlers,
    clone_federation_handlers, collaborative_reviews, compatibility_testing_handlers,
    contract_events, contract_stats_handlers, contributor_handlers, custom_metrics_handlers,
    db_pool, dependency_handlers, deprecated_contracts_handlers, deprecation_handlers,
    elasticsearch_handlers, error_logging, feature_flags, formal_verification_handlers,
    formal_verification_integration, gas_estimation_handlers, governance_handlers,
    graph_analysis_handlers, handlers, integrity, interoperability_handlers,
    marketplace::{
        license_handlers as mp_license, metering as mp_metering, pricing_handlers as mp_pricing,
        stripe_handlers as mp_stripe, usdc_handlers as mp_usdc,
    },
    metrics_handler, migration_handlers, mutation_testing_handlers, org_handlers,
    partition_manager, patch_handlers, performance_handlers, plugin_marketplace_handlers,
    publisher_verification_handlers, query_analysis, query_monitor, recommendation_handlers,
    report_handlers, resource_handlers, search_postgres, security_scan_handlers,
    signature_verification, similarity_handlers, simulation_handlers,
    state::AppState,
    state_monitor::handlers as state_monitor_handlers,
    stats, subscription_handlers, v1_contract_handlers, v1_search_handlers, v1_similar_handlers,
    v1_trending_handlers, verification_handlers, websocket, zk_proof_handlers,
};

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

// ── Issue #888: contract signature verification system ───────────────────────

pub fn signature_verification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/signatures/keys",
            post(signature_verification::register_key),
        )
        .route(
            "/api/signatures/keys/:key_id",
            get(signature_verification::get_key),
        )
        .route(
            "/api/signatures/keys/:key_id/rotate",
            post(signature_verification::rotate_key),
        )
        .route(
            "/api/signatures/keys/:key_id/revoke",
            post(signature_verification::revoke_key),
        )
        .route(
            "/api/signatures/keys/:key_id/verify-chain",
            post(signature_verification::verify_chain),
        )
        .route(
            "/api/signatures/revocations",
            get(signature_verification::list_revocations),
        )
        .route(
            "/api/signatures",
            post(signature_verification::store_signature),
        )
        .route(
            "/api/signatures/verify",
            post(signature_verification::verify),
        )
        .route(
            "/api/contracts/:id/signatures",
            get(signature_verification::list_contract_signatures),
        )
        // Application-side query logging & analysis (issue #887)
        .merge(query_analysis_routes())
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

pub fn plugin_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/plugins/marketplace",
            get(plugin_marketplace_handlers::get_marketplace),
        )
        .route(
            "/api/plugins/:name/:version",
            get(plugin_marketplace_handlers::get_plugin_manifest),
        )
}

/// Marketplace Phase 1 — paid contract pricing + Ed25519 license issuance,
/// validation, revocation, and usage metering. Payment-provider integration
/// (Stripe, USDC) lives in later phases and will hang off the same routes.
pub fn marketplace_routes() -> Router<AppState> {
    Router::new()
        // Pricing plans per contract
        .route(
            "/api/contracts/:contract_id/pricing-plans",
            get(mp_pricing::list_plans).post(mp_pricing::create_plan),
        )
        .route(
            "/api/contracts/:contract_id/pricing-plans/:plan_id",
            patch(mp_pricing::update_plan),
        )
        // License issuance + lifecycle
        .route(
            "/api/contracts/:contract_id/licenses",
            post(mp_license::issue_license),
        )
        .route(
            "/api/marketplace/licenses",
            get(mp_license::list_my_licenses),
        )
        .route(
            "/api/marketplace/licenses/validate",
            post(mp_license::validate_license),
        )
        .route(
            "/api/marketplace/licenses/:jti/revoke",
            post(mp_license::revoke_license),
        )
        .route(
            "/api/marketplace/license-pubkey",
            get(mp_license::license_pubkey),
        )
        // Usage metering
        .route(
            "/api/marketplace/licenses/:jti/usage",
            get(mp_metering::get_usage).post(mp_metering::record_usage),
        )
        // Phase 2 — Stripe checkout + webhook (idempotent by event id)
        .route(
            "/api/contracts/:contract_id/checkout",
            post(mp_stripe::create_checkout),
        )
        .route("/api/marketplace/stripe/webhook", post(mp_stripe::webhook))
        // Phase 3 — USDC on Stellar: payment intents + confirm
        .route(
            "/api/contracts/:contract_id/usdc-intents",
            post(mp_usdc::create_intent),
        )
        .route(
            "/api/marketplace/usdc/confirm",
            post(mp_usdc::confirm_intent),
        )
        .route(
            "/api/marketplace/usdc-payments/:payment_id",
            get(mp_usdc::get_intent),
        )
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
