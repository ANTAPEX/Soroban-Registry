use crate::activity_feed_handlers;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/activity-feed",
        get(activity_feed_handlers::get_activity_feed),
    )
}
