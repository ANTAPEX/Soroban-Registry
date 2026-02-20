use axum::{routing::get, routing::post, Router};

use crate::{state::AppState, trust_handlers};

pub fn trust_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/trust-score",
            get(trust_handlers::get_trust_score),
        )
        .route(
            "/api/contracts/:id/trust-score/calculate",
            post(trust_handlers::calculate_trust_score),
        )
        .route(
            "/api/trust-score/weights",
            get(trust_handlers::get_score_weights),
        )
}
