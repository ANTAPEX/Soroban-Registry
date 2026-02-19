use axum::{
    routing::{get, post},
    Router,
};

use crate::{handlers, compliance_handlers, state::AppState};

/// Contract-related routes
pub fn contract_routes() -> Router<AppState> {
    Router::new()
        .route("/api/contracts", get(handlers::list_contracts))
        .route("/api/contracts", post(handlers::publish_contract))
        .route("/api/contracts/:id", get(handlers::get_contract))
        .route("/api/contracts/:id/versions", get(handlers::get_contract_versions))
        .route("/api/contracts/verify", post(handlers::verify_contract))
}

/// Publisher-related routes
pub fn publisher_routes() -> Router<AppState> {
    Router::new()
        .route("/api/publishers", post(handlers::create_publisher))
        .route("/api/publishers/:id", get(handlers::get_publisher))
        .route("/api/publishers/:id/contracts", get(handlers::get_publisher_contracts))
}

/// Compliance-related routes
pub fn compliance_routes() -> Router<AppState> {
    Router::new()
        .route("/api/compliance/frameworks", get(compliance_handlers::get_frameworks))
        .route("/api/compliance/audit", post(compliance_handlers::audit_contract))
        .route("/api/compliance/:contract_id/:framework/report", get(compliance_handlers::generate_report))
        .route("/api/compliance/:contract_id/:framework/gaps", get(compliance_handlers::identify_gaps))
        .route("/api/compliance/:contract_id/:framework/eligible", get(compliance_handlers::check_eligibility))
}

/// Health check routes
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/stats", get(handlers::get_stats))
}
