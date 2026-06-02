use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use shared::models::{
    CreateMigrationRequest, Migration, MigrationStatus, PaginatedResponse,
    UpdateMigrationStatusRequest,
};
use uuid::Uuid;

use super::db_internal_error;
use crate::error::ApiError;
use crate::state::AppState;

/// Create a new migration
pub async fn create_migration(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateMigrationRequest>,
) -> Result<Json<Migration>, ApiError> {
    let migration: Migration = sqlx::query_as(
        "INSERT INTO migrations (contract_id, wasm_hash, status)
        VALUES ($1, $2, 'pending')
        RETURNING id, contract_id, status, wasm_hash, log_output, created_at, updated_at",
    )
    .bind(&payload.contract_id)
    .bind(&payload.wasm_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("create migration", e))?;

    Ok(Json(migration))
}

/// Update a migration status
pub async fn update_migration(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateMigrationStatusRequest>,
) -> Result<Json<Migration>, ApiError> {
    let migration: Migration = sqlx::query_as(
        "UPDATE migrations
        SET status = $1, log_output = COALESCE($2, log_output)
        WHERE id = $3
        RETURNING id, contract_id, status, wasm_hash, log_output, created_at, updated_at",
    )
    .bind(payload.status)
    .bind(payload.log_output)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("update migration", e))?;

    Ok(Json(migration))
}

/// Get all migrations
pub async fn get_migrations(
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<Migration>>, ApiError> {
    // For simplicity, we'll just return the last 50 migrations
    let migrations: Vec<Migration> = sqlx::query_as(
        "SELECT id, contract_id, status, wasm_hash, log_output, created_at, updated_at
        FROM migrations
        ORDER BY created_at DESC
        LIMIT 50",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("get migrations", e))?;

    let total = migrations.len() as i64; // In a real app we'd do a count query
    let response = PaginatedResponse::new(migrations, total, 1, 50);

    Ok(Json(response))
}

/// Get a specific migration
pub async fn get_migration(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Migration>, ApiError> {
    let migration: Migration = sqlx::query_as(
        "SELECT id, contract_id, status, wasm_hash, log_output, created_at, updated_at
        FROM migrations
        WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_internal_error("get migration", e))?
    .ok_or(ApiError::not_found(
        "MigrationNotFound",
        "Migration not found",
    ))?;

    Ok(Json(migration))
}
