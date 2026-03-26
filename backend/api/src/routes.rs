#[cfg(feature = "openapi")]
use crate::openapi;
use crate::{
    ab_test_handlers, auth, auth_handlers, batch_verify_handlers, breaking_changes,
    canary_handlers, compatibility_testing_handlers, custom_metrics_handlers, deprecation_handlers,
    handlers, metrics_handler, migration_handlers, performance_handlers, resource_handlers,
    simulation_handlers, state::AppState,
};
use axum::{
    middleware,
    routing::{get, patch, post},
    Router,
};
#[cfg(feature = "openapi")]
use utoipa::OpenApi;
#[cfg(feature = "openapi")]
use utoipa_swagger_ui::SwaggerUi;

pub fn observability_routes() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler::metrics_endpoint))
}

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/challenge", get(auth_handlers::get_challenge))
        .route("/api/auth/verify", post(auth_handlers::verify_challenge))
}

pub fn contract_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts",
            get(handlers::list_contracts).post(handlers::publish_contract),
        )
        .route(
            "/api/contracts/trending",
            get(handlers::get_trending_contracts),
        )
        .route("/api/contracts/graph", get(handlers::get_contract_graph))
        .route("/api/contracts/:id", get(handlers::get_contract))
        .route(
            "/api/contracts/:id/metadata",
            patch(handlers::update_contract_metadata),
        )
        .route(
            "/api/contracts/:id/publisher",
            patch(handlers::change_contract_publisher),
        )
        .route(
            "/api/contracts/:id/status",
            patch(handlers::update_contract_status),
        )
        .route(
            "/api/contracts/:id/audit-log",
            get(handlers::get_contract_audit_log),
        )
        .route("/api/contracts/:id/abi", get(handlers::get_contract_abi))
        .route(
            "/api/contracts/:id/openapi.yaml",
            get(handlers::get_contract_openapi_yaml),
        )
        .route(
            "/api/contracts/:id/openapi.json",
            get(handlers::get_contract_openapi_json),
        )
        .route(
            "/api/contracts/:id/versions",
            get(handlers::get_contract_versions).post(handlers::create_contract_version),
        )
        .route(
            "/api/contracts/:id/changelog",
            get(handlers::get_contract_changelog),
        )
        .route(
            "/contracts/:id/changelog",
            get(handlers::get_contract_changelog),
        )
        .route(
            "/api/contracts/breaking-changes",
            get(breaking_changes::get_breaking_changes),
        )
        .route(
            "/api/contracts/:id/interactions",
            get(handlers::get_contract_interactions).post(handlers::post_contract_interaction),
        )
        .route(
            "/api/contracts/:id/interactions/batch",
            post(handlers::post_contract_interactions_batch),
        )
        .route(
            "/api/contracts/:id/deprecation-info",
            get(deprecation_handlers::get_deprecation_info),
        )
        .route(
            "/api/contracts/:id/deprecate",
            post(deprecation_handlers::deprecate_contract),
        )
        .route(
            "/api/contracts/:id/state/:key",
            get(handlers::get_contract_state)
                .put(handlers::update_contract_state)
                .post(handlers::update_contract_state),
        )
        .route(
            "/api/contracts/:id/analytics",
            get(handlers::get_contract_analytics),
        )
        .route(
            "/api/contracts/:id/trust-score",
            get(handlers::get_trust_score),
        )
        .route(
            "/api/contracts/:id/dependencies",
            get(handlers::get_contract_dependencies),
        )
        .route(
            "/api/contracts/:id/dependents",
            get(handlers::get_contract_dependents),
        )
        .route(
            "/api/contracts/:id/impact",
            get(handlers::get_impact_analysis),
        )
        .route("/api/contracts/verify", post(handlers::verify_contract))
        .route("/api/contracts/:id/verifications", get(handlers::get_contract_verifications))
        .route("/api/verifications/:id", get(handlers::get_verification))
        .route(
            "/api/contracts/batch-verify",
            post(batch_verify_handlers::batch_verify_contracts),
        )
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
            "/api/contracts/:id/resources",
            get(resource_handlers::get_contract_resources),
        )
        .route(
            "/api/contracts/:id/metrics/batch",
            post(custom_metrics_handlers::record_metrics_batch),
        )
        .route(
            "/api/contracts/:id/metrics/catalog",
            get(custom_metrics_handlers::get_metric_catalog),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix",
            get(compatibility_testing_handlers::get_compatibility_matrix),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/test",
            post(compatibility_testing_handlers::run_compatibility_test),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/history",
            get(compatibility_testing_handlers::get_compatibility_history),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/notifications",
            get(compatibility_testing_handlers::get_compatibility_notifications),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/notifications/read",
            post(compatibility_testing_handlers::mark_notifications_read),
        )
        .route(
            "/api/contracts/:id/deployments/status",
            get(handlers::get_deployment_status),
        )
        .route(
            "/api/contracts/:id/deployment-status",
            get(handlers::get_deployment_status),
        )
        .route("/api/deployments/green", post(handlers::deploy_green))
        .route(
            "/api/contracts/:id/deploy-green",
            post(handlers::deploy_green),
        )
        .route(
            "/api/contracts/simulate-deploy",
            post(simulation_handlers::simulate_deploy),
        )
}

#[cfg(not(feature = "openapi"))]
pub fn openapi_routes() -> Router<AppState> {
    Router::new()
}

#[cfg(feature = "openapi")]
pub fn openapi_routes() -> Router<AppState> {
    Router::new().merge(
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
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

pub fn health_monitor_routes() -> Router<AppState> {
    Router::new().route(
        "/api/health-monitor/status",
        get(crate::health_monitor::get_health_monitor_status),
    )
}

pub fn migration_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/migrations/status",
            get(migration_handlers::get_migration_status),
        )
        .route(
            "/api/admin/migrations/register",
            post(migration_handlers::register_migration),
        )
        .route(
            "/api/admin/migrations/validate",
            get(migration_handlers::validate_migrations),
        )
        .route(
            "/api/admin/migrations/lock",
            get(migration_handlers::get_lock_status),
        )
        .route(
            "/api/admin/migrations/:version",
            get(migration_handlers::get_migration_version),
        )
        .route(
            "/api/admin/migrations/:version/rollback",
            post(migration_handlers::rollback_migration),
        )
}

pub fn compatibility_dashboard_routes() -> Router<AppState> {
    Router::new().route(
        "/api/compatibility-dashboard",
        get(compatibility_testing_handlers::get_compatibility_dashboard),
    )
}

pub fn canary_routes() -> Router<AppState> {
    Router::new()
        // Contract-scoped canary endpoints
        .route(
            "/api/contracts/:id/canary",
            get(canary_handlers::list_canaries).post(canary_handlers::create_canary),
        )
        // Canary-specific endpoints
        .route("/api/canary/:canary_id", get(canary_handlers::get_canary))
        .route(
            "/api/canary/:canary_id/advance",
            post(canary_handlers::advance_canary),
        )
        .route(
            "/api/canary/:canary_id/rollback",
            post(canary_handlers::rollback_canary),
        )
        .route(
            "/api/canary/:canary_id/complete",
            post(canary_handlers::complete_canary),
        )
        .route(
            "/api/canary/:canary_id/metrics",
            get(canary_handlers::list_canary_metrics).post(canary_handlers::record_canary_metric),
        )
}

pub fn ab_test_routes() -> Router<AppState> {
    Router::new()
        // Contract-scoped A/B test endpoints
        .route(
            "/api/contracts/:id/ab-tests",
            get(ab_test_handlers::list_ab_tests).post(ab_test_handlers::create_ab_test),
        )
        // A/B test-specific endpoints
        .route("/api/ab-tests/:test_id", get(ab_test_handlers::get_ab_test))
        .route(
            "/api/ab-tests/:test_id/start",
            post(ab_test_handlers::start_ab_test),
        )
        .route(
            "/api/ab-tests/:test_id/stop",
            post(ab_test_handlers::stop_ab_test),
        )
        .route(
            "/api/ab-tests/:test_id/cancel",
            post(ab_test_handlers::cancel_ab_test),
        )
        .route(
            "/api/ab-tests/:test_id/metrics",
            post(ab_test_handlers::record_ab_test_metric),
        )
        .route(
            "/api/ab-tests/:test_id/results",
            get(ab_test_handlers::get_ab_test_results),
        )
}

pub fn performance_routes() -> Router<AppState> {
    Router::new()
        // Contract-scoped performance endpoints
        .route(
            "/api/contracts/:id/perf/metrics",
            get(performance_handlers::list_metrics).post(performance_handlers::record_metric),
        )
        .route(
            "/api/contracts/:id/perf/anomalies",
            get(performance_handlers::list_anomalies),
        )
        .route(
            "/api/contracts/:id/perf/alerts",
            get(performance_handlers::list_alerts),
        )
        .route(
            "/api/contracts/:id/perf/alert-configs",
            get(performance_handlers::list_alert_configs)
                .post(performance_handlers::create_alert_config),
        )
        .route(
            "/api/contracts/:id/perf/trends",
            get(performance_handlers::list_trends),
        )
        .route(
            "/api/contracts/:id/perf/summary",
            get(performance_handlers::get_performance_summary),
        )
        // Alert-specific action endpoints
        .route(
            "/api/perf/alerts/:alert_id/acknowledge",
            post(performance_handlers::acknowledge_alert),
        )
        .route(
            "/api/perf/alerts/:alert_id/resolve",
            post(performance_handlers::resolve_alert),
        )
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/audit-logs", get(handlers::get_all_audit_logs))
        .merge(migration_routes())
        .route_layer(middleware::from_fn(auth::require_admin))
}
