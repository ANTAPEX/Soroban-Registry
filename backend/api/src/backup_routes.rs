use axum::{routing::get, routing::post, Router};

use crate::{backup_handlers, state::AppState};

pub fn backup_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/backups",
            post(backup_handlers::create_backup).get(backup_handlers::list_backups),
        )
        .route(
            "/api/contracts/:id/backups/restore",
            post(backup_handlers::restore_backup),
        )
        .route(
            "/api/contracts/:id/backups/:date/verify",
            post(backup_handlers::verify_backup),
        )
        .route(
            "/api/contracts/:id/backups/stats",
            get(backup_handlers::get_backup_stats),
        )
        // Disaster Recovery Routes
        .route(
            "/api/contracts/:id/disaster-recovery-plan",
            post(backup_handlers::create_disaster_recovery_plan)
                .get(backup_handlers::get_disaster_recovery_plan),
        )
        .route(
            "/api/contracts/:id/disaster-recovery/execute",
            post(backup_handlers::execute_recovery),
        )
}
