#![feature(str_as_str)]

mod aggregation;
mod analytics;
mod audit_handlers;
mod audit_routes;
mod backup_handlers;
mod backup_routes;
mod benchmark_engine;
mod benchmark_handlers;
mod benchmark_routes;
mod cache;
mod cache_benchmark;
mod checklist;
mod cost_handlers;
mod cost_routes;
mod detector;
mod error;
mod governance_handlers;
mod governance_routes;
mod handlers;
mod maintenance_handlers;
mod maintenance_middleware;
mod maintenance_routes;
mod maintenance_scheduler;
mod maturity_handlers;
mod maturity_routes;
mod multisig_handlers;
mod multisig_routes;
mod models;
mod rate_limit;
mod routes;
mod scoring;
mod state;
mod trust_handlers;
mod trust_routes;

use anyhow::Result;
use axum::http::{header, HeaderValue, Method};
use axum::{middleware, Router};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::rate_limit::RateLimitState;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("../../database/migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database connected and migrations applied");

    // Spawn the hourly analytics aggregation background task
    aggregation::spawn_aggregation_task(pool.clone());
    
    // Spawn maintenance scheduler
    maintenance_scheduler::spawn_maintenance_scheduler(pool.clone());

    // Create app state
    let state = AppState::new(pool);
    let rate_limit_state = RateLimitState::from_env();

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("https://soroban-registry.vercel.app"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    // Build router
    let app = Router::new()
        .merge(routes::contract_routes())
        .merge(routes::publisher_routes())
        .merge(routes::health_routes())
        .merge(routes::migration_routes())
        .merge(multisig_routes::multisig_routes())
        .merge(audit_routes::security_audit_routes())
        .merge(benchmark_routes::benchmark_routes())
        .merge(maintenance_routes::maintenance_routes())
        .merge(maturity_routes::maturity_routes())
        .merge(cost_routes::cost_routes())
        .merge(backup_routes::backup_routes())
        .merge(governance_routes::governance_routes())
        .merge(trust_routes::trust_routes())
        .fallback(handlers::route_not_found)
        .layer(middleware::from_fn(request_logger))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            maintenance_middleware::maintenance_check,
        ))
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit::rate_limit_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(cors)
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn request_logger(
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(req).await;

    let elapsed = start.elapsed().as_millis();
    let status = response.status().as_u16();

    tracing::info!("{method} {uri} {status} {elapsed}ms");

    response
}
