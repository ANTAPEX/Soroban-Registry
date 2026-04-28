#![warn(unused_imports)]

mod ai;
mod state_monitor;
mod search_postgres;
mod stats;

mod ab_test_handlers;
mod abi_versioning_handlers;
mod aggregation;
mod analytics;
mod analytics_handlers;
mod auth;
mod auth_handlers;
mod backup_handlers;
mod backup_routes;
mod batch_verify_handlers;
mod breaking_changes;
mod cache;
mod canary_handlers;
mod collaborative_reviews;
mod compatibility_testing_handlers;
mod contract_events;
mod contributor_handlers;
mod custom_metrics_handlers;
mod db_monitoring;
mod deprecation_handlers;
mod disaster_recovery_models;
mod error;
mod error_logging;
mod formal_verification;
mod formal_verification_handlers;
mod gas_estimation_handlers;
mod governance_handlers;
mod graph_analysis;
mod graph_analysis_handlers;
mod graphql;
mod handlers;
mod health_monitor;
#[cfg(test)]
mod health_tests;
mod incident_handlers;
mod incident_routes;
mod metrics;
mod metrics_handler;
mod migration_handlers;
mod models;
mod multisig_handlers;
mod multisig_routes;
mod mutation_testing_handlers;
mod notification_handlers;
mod notification_routes;
mod onchain_verification;
#[cfg(feature = "openapi")]
mod openapi;
mod org_handlers;
mod pagination;
mod patch_handlers;
mod performance_handlers;
mod plugin_marketplace_handlers;
mod post_incident_handlers;
mod post_incident_routes;
mod publisher_verification_handlers;
mod quota_handlers;
mod recommendation_handlers;
mod release_notes_handlers;
mod release_notes_routes;
mod request_tracing;
mod resource_handlers;
mod resource_tracking;
mod routes;
mod search_client;
mod security_scan_handlers;
mod similarity_handlers;
mod simulation;
mod simulation_handlers;
mod subscription_handlers;
mod type_safety;
mod usage_counter;
mod validation;
mod verification_handlers;
mod webhook_delivery;
mod websocket;
mod zk_proof_handlers;

use anyhow::Result;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware;
use axum::response::Response;
use dotenv::dotenv;
use prometheus::Registry;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use crate::ai::service::AIService;
use crate::state_monitor::StateMonitorService;

async fn track_in_flight_middleware(
    State(state): State<AppState>,
    req: Request,
    next: middleware::Next,
) -> Result<Response, ApiError> {
    if state.is_shutting_down.load(Ordering::Relaxed) {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            "Service is shutting down and temporarily unavailable",
        ));
    }
    api::metrics::HTTP_IN_FLIGHT.inc();
    let res = next.run(req).await;
    api::metrics::HTTP_IN_FLIGHT.dec();
    Ok(res)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load and validate configuration (#768)
    let config = config::load_config()?;

    // Initialize structured JSON tracing (ELK/Splunk compatible)
    request_tracing::init_json_tracing();

    let logical_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let max_pool_size = std::env::var("DB_MAX_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or((logical_cores * 2).max(10) as u32);

    tracing::info!(
        max_pool_size = max_pool_size,
        logical_cores = logical_cores,
        "Initializing database connection pool"
    );

    let pool = PgPoolOptions::new()
        .max_connections(max_pool_size)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect_with(
            config
                .database_url
                .parse::<sqlx::postgres::PgConnectOptions>()?,
        )
        .await?;

    // Run migrations (skip if SKIP_MIGRATIONS=true, useful when migrations were applied manually)
    let skip_migrations = std::env::var("SKIP_MIGRATIONS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if skip_migrations {
        tracing::info!("Skipping automatic migrations (SKIP_MIGRATIONS=true)");
    } else {
        sqlx::migrate!("../../database/migrations")
            .run(&pool)
            .await?;
    }

    tracing::info!("Database connected and migrations applied");

    // Check migration versioning state on startup (Issue #252)
    migration_handlers::check_migrations_on_startup(&pool).await;

    // Spawn the hourly analytics aggregation background task
    aggregation::spawn_aggregation_task(pool.clone());

    // Create prometheus registry for metrics
    let registry = Registry::new();
    if let Err(e) = api::metrics::register_all(&registry) {
        tracing::error!("Failed to register metrics: {}", e);
    }

    // Job engine omitted: optional dependency; add soroban_batch and uncomment to enable.
    // let (job_engine, job_rx) = soroban_batch::engine::JobEngine::new();
    // let job_engine = Arc::new(job_engine);
    // tokio::spawn(async move { job_engine.clone().run_worker(job_rx).await });

    // Create app state
    let is_shutting_down = Arc::new(AtomicBool::new(false));
    // Job engine: initialize for background batch processing
    let (job_engine, job_rx) = soroban_batch::engine::JobEngine::new();
    let job_engine = Arc::new(job_engine);
    let je = job_engine.clone();
    tokio::spawn(async move { je.run_worker(job_rx).await });

    // Issue #727: create rate limiter before AppState so it can be shared
    let rate_limit_state = std::sync::Arc::new(RateLimitState::from_env());
    rate_limit_state.spawn_eviction_task();

    // Initialize AI service (optional - graceful if not configured)
    let ai_service = match AIService::from_env() {
        Ok(service) => {
            tracing::info!("AI service initialized successfully");
            Some(Arc::new(service))
        }
        Err(e) => {
            tracing::warn!("AI service not initialized: {}", e);
            None
        }
    };

    // Create event broadcaster for real-time updates
    let (event_broadcaster, _) = broadcast::channel(100);

    // Create app state
    let is_shutting_down = Arc::new(AtomicBool::new(false));
    // Job engine: initialize for background batch processing
    let (job_engine, job_rx) = soroban_batch::engine::JobEngine::new();
    let job_engine = Arc::new(job_engine);
    let je = job_engine.clone();
    tokio::spawn(async move { je.run_worker(job_rx).await });

    let state = AppState::new(
        pool.clone(),
        registry,
        job_engine,
        is_shutting_down.clone(),
        rate_limit_state.clone(),
        ai_service.clone(),
        event_broadcaster.clone(),
    )
    .await?;

    // Initialize state monitor service (optional)
    if let Some(monitor) = state.state_monitor.clone() {
        // Spawn state monitor background task
        tokio::spawn(async move {
            if let Err(e) = monitor.run().await {
                tracing::error!("State monitor error: {}", e);
            }
        });
    }

    // Initialize state monitor service (optional)
    let state_monitor = match crate::state_monitor::StateMonitorService::new(
        pool.clone(),
        event_broadcaster.clone(),
    ) {
        Ok(service) => {
            info!("State monitor service initialized");
            let monitor = Arc::new(service);
            state.state_monitor = Some(monitor.clone());
            
            // Spawn state monitor background task
            let monitor_clone = monitor.clone();
            tokio::spawn(async move {
                if let Err(e) = monitor_clone.run().await {
                    error!("State monitor error: {}", e);
                }
            });
            
            Some(monitor)
        }
        Err(e) => {
            warn!("State monitor service not initialized: {}", e);
            None
        }
    };

    // Initialize GraphQL schema
    let schema = graphql::schema::build_schema(state.clone());

    // Spawn webhook delivery background task
    webhook_delivery::spawn_webhook_delivery_task(pool.clone());

    // Spawn the background DB and cache monitoring task
    db_monitoring::spawn_db_monitoring_task(pool.clone(), state.cache.clone());

    // Spawn the health monitor background task (Issue #333)
    let hm_state = state.clone();
    let hm_status = state.health_monitor_status.clone();
    tokio::spawn(async move {
        health_monitor::run_health_monitor(hm_state, hm_status).await;
    });

    let network_state = state.clone();
    tokio::spawn(async move {
        handlers::run_network_catalog_refresh(network_state).await;
    });

    // Warm up the cache
    state.cache.clone().warm_up(pool.clone());

    let allowed_origins = std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:3000,https://soroban-registry.vercel.app".to_string()
    });

    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(HeaderValue::from_str(s).expect("Invalid allowed origin"))
            }
        })
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            crate::request_tracing::X_REQUEST_ID.clone(),
            crate::request_tracing::X_CORRELATION_ID.clone(),
        ])
        .expose_headers([
            crate::request_tracing::X_REQUEST_ID.clone(),
            crate::request_tracing::X_CORRELATION_ID.clone(),
        ]);

    // Build router
    let app = routes::application_routes(schema)
        .fallback(handlers::route_not_found)
        .layer(middleware::from_fn(
            validation::payload_size::payload_size_validation_middleware,
        ))
        .layer(middleware::from_fn(
            validation::enhanced_extractors::validation_failure_tracking_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            track_in_flight_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            (*rate_limit_state).clone(),
            rate_limit::rate_limit_middleware,
        ))
        .layer(cors)
        .layer(middleware::from_fn(request_tracing::tracing_middleware))
        .with_state(state.clone());

    // Start server (port configurable via PORT env var, default 3001)
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        tracing::info!(
            "SIGTERM/SIGINT received. Failing health checks and stopping new requests..."
        );
        let _ = tx.send(()).await;
    });

    let server_task = tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!("Server error: {}", e);
        }
    });

    if let Some(()) = rx.recv().await {
        is_shutting_down.store(true, Ordering::SeqCst);
        let initial_in_flight = crate::metrics::HTTP_IN_FLIGHT.get();
        tracing::info!(
            "Graceful shutdown initiated. In-flight requests: {}",
            initial_in_flight
        );

        let timeout_secs = std::env::var("SHUTDOWN_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .unwrap_or(30);

        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);

        let mut success = false;
        loop {
            let in_flight = crate::metrics::HTTP_IN_FLIGHT.get();
            if in_flight == 0 {
                tracing::info!(
                    "All in-flight requests completed in {}ms. In-flight: 0",
                    start_time.elapsed().as_millis()
                );
                success = true;
                break;
            }
            if start_time.elapsed() > timeout_duration {
                tracing::error!(
                    "Graceful shutdown timeout ({}s) reached. {} requests still in-flight.",
                    timeout_secs,
                    in_flight
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        tracing::info!("Closing database connections cleanly...");
        pool.close().await;

        let shutdown_duration = start_time.elapsed();
        tracing::info!(
            "Shutdown complete. Duration: {}ms",
            shutdown_duration.as_millis()
        );

        if success {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    } else {
        let _ = server_task.await;
        tracing::info!("Closing database connections cleanly...");
        pool.close().await;
        tracing::info!("Shutdown complete");
    }

    Ok(())
}
