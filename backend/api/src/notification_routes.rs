use axum::{routing::get, routing::post, Router};

use crate::{notification_handlers, state::AppState};

pub fn notification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/notification-templates",
            post(notification_handlers::create_notification_template),
        )
        .route(
            "/api/notification-templates/:name",
            get(notification_handlers::get_notification_template),
        )
        .route(
            "/api/users/:id/notification-preferences",
            post(notification_handlers::create_user_notification_preference)
                .get(notification_handlers::get_user_notification_preferences),
        )
        .route(
            "/api/notifications/send",
            post(notification_handlers::send_notification),
        )
        .route(
            "/api/users/:id/notifications",
            get(notification_handlers::get_user_notifications),
        )
}