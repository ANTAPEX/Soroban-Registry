use axum::{routing::get, routing::post, Router};

use crate::{post_incident_handlers, state::AppState};

pub fn post_incident_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/post-incident-reports",
            post(post_incident_handlers::create_post_incident_report),
        )
        .route(
            "/api/post-incident-reports/:id",
            get(post_incident_handlers::get_post_incident_report),
        )
        .route(
            "/api/contracts/:id/post-incident-reports",
            get(post_incident_handlers::get_contract_post_incident_reports),
        )
        .route(
            "/api/post-incident-reports/:report_id/action-items",
            post(post_incident_handlers::create_action_item)
                .get(post_incident_handlers::get_action_items_for_report),
        )
        .route(
            "/api/action-items/:id/status/:status",
            post(post_incident_handlers::update_action_item_status),
        )
}