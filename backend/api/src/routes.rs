use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    handlers, metrics_handler, custom_metrics_handlers,
    handlers, metrics_handler, breaking_changes,
    changelog_handlers,
    deprecation_handlers,
    state::AppState,
};

pub fn observability_routes() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler::metrics_endpoint))
}

pub fn contract_routes() -> Router<AppState> {
    Router::new()
        .route("/api/contracts", get(handlers::list_contracts))
        .route("/api/contracts", post(handlers::publish_contract))
        .route("/api/contracts/trending", get(handlers::get_trending_contracts))
        .route("/api/contracts/graph", get(handlers::get_contract_graph))
        .route("/api/contracts/:id", get(handlers::get_contract))
        .route("/api/contracts/:id/abi", get(handlers::get_contract_abi))
        .route("/api/contracts/:id/versions", get(handlers::get_contract_versions).post(handlers::create_contract_version))
        .route("/api/contracts/breaking-changes", get(breaking_changes::get_breaking_changes))
        .route("/api/contracts/:id/versions", get(handlers::get_contract_versions))
<<<<<<< feature/issue-46-add-contract-interaction-history-tracking
        .route(
            "/api/contracts/:id/interactions",
            get(handlers::get_contract_interactions).post(handlers::post_contract_interaction),
        )
        .route(
            "/api/contracts/:id/interactions/batch",
            post(handlers::post_contract_interactions_batch),
        )
=======
        .route("/api/contracts/:id/deprecation-info", get(deprecation_handlers::get_deprecation_info))
        .route("/api/contracts/:id/deprecate", post(deprecation_handlers::deprecate_contract))
>>>>>>> main
        .route("/api/contracts/:id/state/:key", get(handlers::get_contract_state).post(handlers::update_contract_state))
        .route("/api/contracts/:id/analytics", get(handlers::get_contract_analytics))
        .route("/api/contracts/:id/trust-score", get(handlers::get_trust_score))
        .route("/api/contracts/:id/dependencies", get(handlers::get_contract_dependencies))
        .route("/api/contracts/:id/dependents", get(handlers::get_contract_dependents))
        .route("/api/contracts/verify", post(handlers::verify_contract))
        .route(
            "/api/contracts/:id/performance",
            get(handlers::get_contract_performance),
        )
        .route(
            "/api/contracts/:id/metrics",
            get(custom_metrics_handlers::get_contract_metrics)
                .post(custom_metrics_handlers::record_contract_metric),
        )
        .route(
            "/api/contracts/:id/metrics/batch",
            post(custom_metrics_handlers::record_metrics_batch),
        )
        .route(
            "/api/contracts/:id/metrics/catalog",
            get(custom_metrics_handlers::get_metric_catalog),
        )
        // .route(
        //     "/api/contracts/:id/compatibility",
        //     get(compatibility_handlers::get_contract_compatibility)
        //         .post(compatibility_handlers::add_contract_compatibility),
        // )
        // .route(
        //     "/api/contracts/:id/compatibility/export",
        //     get(compatibility_handlers::export_contract_compatibility),
        // )
        .route("/api/contracts/:id/deployments/status", get(handlers::get_deployment_status))
        .route("/api/deployments/green", post(handlers::deploy_green))
        .route(
            "/api/contracts/:id/changelog",
            get(changelog_handlers::get_contract_changelog)
                .post(changelog_handlers::generate_changelog),
        )
}

pub fn publisher_routes() -> Router<AppState> {
    Router::new()
        .route("/api/publishers", post(handlers::create_publisher))
        .route("/api/publishers/:id", get(handlers::get_publisher))
        .route(
            "/api/publishers/:id/contracts",
            get(handlers::get_publisher_contracts),
        )
}

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/stats", get(handlers::get_stats))
}



pub fn migration_routes() -> Router<AppState> {
    Router::new()
}

pub fn canary_routes() -> Router<AppState> { Router::new() }
pub fn ab_test_routes() -> Router<AppState> { Router::new() }
pub fn performance_routes() -> Router<AppState> { Router::new() }
