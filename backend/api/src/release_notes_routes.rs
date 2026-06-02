use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{release_notes_handlers, state::AppState};

pub fn release_notes_routes() -> Router<AppState> {
    Router::new()
        // List all release notes for a contract
        .route(
            "/api/contracts/:id/release-notes",
            get(release_notes_handlers::list_release_notes),
        )
        // Auto-generate release notes for a version
        .route(
            "/api/contracts/:id/release-notes/generate",
            post(release_notes_handlers::generate_release_notes),
        )
        // Get release notes for a specific version
        .route(
            "/api/contracts/:id/release-notes/:version",
            get(release_notes_handlers::get_release_notes),
        )
        // Edit draft release notes
        .route(
            "/api/contracts/:id/release-notes/:version",
            put(release_notes_handlers::update_release_notes),
        )
        // Publish (finalize) release notes
        .route(
            "/api/contracts/:id/release-notes/:version/publish",
            post(release_notes_handlers::publish_release_notes),
        )
}
