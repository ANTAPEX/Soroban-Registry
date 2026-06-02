use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use shared::{
    Contract, ContractSearchParams, ContractVersion, CreateContractVersionRequest, PaginatedResponse, PublishRequest, Publisher,
    SemVer, VerifyRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    breaking_changes::{diff_abi, has_breaking_changes, resolve_abi},
    state::AppState,
};

fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

fn map_json_rejection(err: JsonRejection) -> ApiError {
    ApiError::bad_request("InvalidRequest", format!("Invalid JSON payload: {}", err.body_text()))
}

fn map_query_rejection(err: QueryRejection) -> ApiError {
    ApiError::bad_request("InvalidQuery", format!("Invalid query parameters: {}", err.body_text()))
}

#[utoipa::path(
    get,
    path = "/health",
    summary = "Health check",
    description = "Returns service uptime and database connectivity status.",
    tag = "Health",
    responses(
        (status = 200, description = "Service healthy", body = Value),
        (status = 503, description = "Service degraded", body = Value)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let uptime = state.started_at.elapsed().as_secs();
    let now = chrono::Utc::now().to_rfc3339();

    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    if db_ok {
        tracing::info!(uptime_secs = uptime, "health check passed");
        (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "version": "0.1.0",
                "timestamp": now,
                "uptime_secs": uptime
            })),
        )
    } else {
        tracing::warn!(uptime_secs = uptime, "health check degraded — db unreachable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "degraded",
                "version": "0.1.0",
                "timestamp": now,
                "uptime_secs": uptime
            })),
        )
    }
}

#[utoipa::path(
    get,
    path = "/stats",
    summary = "Registry statistics",
    description = "Returns aggregate counts and lightweight API version usage counters.",
    tag = "System",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Stats response", body = crate::openapi::StatsResponse),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_stats(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let total_contracts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count contracts", err))?;

    let verified_contracts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE is_verified = true")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count verified contracts", err))?;

    let total_publishers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM publishers")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count publishers", err))?;

    Ok(Json(json!({
        "total_contracts": total_contracts,
        "verified_contracts": verified_contracts,
        "total_publishers": total_publishers,
        "api_versions": state.api_version_metrics.snapshot(),
    })))
}

/// List and search contracts
#[utoipa::path(
    get,
    path = "/contracts",
    summary = "List contracts",
    description = "Lists contracts with filtering, pagination and sorting.",
    tag = "Contracts",
    params(shared::ContractSearchParams),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Paginated contract list", body = crate::openapi::PaginatedContractsResponse),
        (status = 400, description = "Invalid query", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn list_contracts(
    State(state): State<AppState>,
    params: Result<Query<ContractSearchParams>, QueryRejection>,
) -> axum::response::Response {
    let Query(params) = match params {
        Ok(q) => q,
        Err(err) => return map_query_rejection(err).into_response(),
    };
    
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).max(0) * limit;

    let sort_by = params.sort_by.clone().unwrap_or_else(|| {
        if params.query.is_some() {
            shared::SortBy::Relevance
        } else {
            shared::SortBy::CreatedAt
        }
    });
    let sort_order = params.sort_order.clone().unwrap_or(shared::SortOrder::Desc);

    // Build dynamic query with aggregations
    let mut query = String::from(
        "SELECT c.*
         FROM contracts c
         LEFT JOIN contract_interactions ci ON c.id = ci.contract_id
         LEFT JOIN contract_versions cv ON c.id = cv.contract_id
         WHERE 1=1"
    );
    let mut count_query = String::from("SELECT COUNT(*) FROM contracts WHERE 1=1");

    if let Some(ref q) = params.query {
        let search_clause = format!(
            " AND (c.name ILIKE '%{}%' OR c.description ILIKE '%{}%')",
            q, q
        );
        query.push_str(&search_clause);
        count_query.push_str(&search_clause);
    }

    if let Some(verified) = params.verified_only {
        if verified {
            query.push_str(" AND c.is_verified = true");
            count_query.push_str(" AND is_verified = true");
        }
    }

    if let Some(ref category) = params.category {
        let category_clause = format!(" AND c.category = '{}'", category);
        query.push_str(&category_clause);
        count_query.push_str(&category_clause);
    }

    query.push_str(" GROUP BY c.id");

    // Sorting logic using aggregations in ORDER BY
    let order_by = match sort_by {
        shared::SortBy::CreatedAt => "c.created_at".to_string(),
        shared::SortBy::UpdatedAt => "c.updated_at".to_string(),
        shared::SortBy::Popularity | shared::SortBy::Interactions => "COUNT(DISTINCT ci.id)".to_string(),
        shared::SortBy::Deployments => "COUNT(DISTINCT cv.id)".to_string(),
        shared::SortBy::Relevance => {
            if let Some(ref q) = params.query {
                format!(
                    "CASE WHEN c.name ILIKE '{}' THEN 0 
                          WHEN c.name ILIKE '%{}%' THEN 1 
                          ELSE 2 END",
                    q, q
                )
            } else {
                "c.created_at".to_string()
            }
        }
    };

    let direction = if sort_order == shared::SortOrder::Asc { "ASC" } else { "DESC" };
    
    query.push_str(&format!(
        " ORDER BY {} {}, c.id DESC LIMIT {} OFFSET {}",
        order_by, direction, limit, offset
    ));

    let contracts: Vec<Contract> = match sqlx::query_as(&query)
        .fetch_all(&state.db)
        .await
    {
        Ok(rows) => rows,
        Err(err) => return db_internal_error("list contracts", err).into_response(),
    };

    let total: i64 = match sqlx::query_scalar(&count_query)
        .fetch_one(&state.db)
        .await
    {
        Ok(v) => v,
        Err(err) => return db_internal_error("count filtered contracts", err).into_response(),
    };

    (
        StatusCode::OK,
        Json(PaginatedResponse::new(contracts, total, page, limit)),
    ).into_response()
}

/// Get a specific contract by ID
#[utoipa::path(
    get,
    path = "/contracts/{id}",
    summary = "Get contract",
    description = "Fetches a contract by UUID.",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Contract", body = Contract),
        (status = 400, description = "Invalid contract id", body = crate::openapi::ErrorBody),
        (status = 404, description = "Not found", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Contract>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("get contract by id", err),
        })?;

    Ok(Json(contract))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/versions",
    summary = "List contract versions",
    description = "Returns versions for a contract, ordered newest-first.",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Contract versions", body = [ContractVersion]),
        (status = 400, description = "Invalid contract id", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ContractVersion>>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let versions: Vec<ContractVersion> = sqlx::query_as(
        "SELECT * FROM contract_versions WHERE contract_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("get contract versions", err))?;

    Ok(Json(versions))
}

#[utoipa::path(
    post,
    path = "/contracts/{id}/versions",
    summary = "Create contract version",
    description = "Creates a new contract version and stores its ABI. Enforces semver major bump on breaking ABI changes.",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract UUID or contract_id")
    ),
    request_body = shared::CreateContractVersionRequest,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Created version", body = ContractVersion),
        (status = 400, description = "Invalid input", body = crate::openapi::ErrorBody),
        (status = 404, description = "Contract not found", body = crate::openapi::ErrorBody),
        (status = 422, description = "Unprocessable request", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn create_contract_version(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<CreateContractVersionRequest>, JsonRejection>,
) -> ApiResult<Json<ContractVersion>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;
    if !req.contract_id.trim().is_empty() && req.contract_id != contract_id {
        return Err(ApiError::bad_request(
            "ContractMismatch",
            "Contract ID in payload does not match path",
        ));
    }

    let new_version = SemVer::parse(&req.version).ok_or_else(|| {
        ApiError::bad_request("InvalidVersion", "Version must be valid semver (e.g. 1.2.3)")
    })?;

    let existing_versions: Vec<String> = sqlx::query_scalar(
        "SELECT version FROM contract_versions WHERE contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch contract versions", err))?;

    if !existing_versions.is_empty() {
        let mut parsed: Vec<SemVer> = Vec::with_capacity(existing_versions.len());
        for version in &existing_versions {
            let parsed_version = SemVer::parse(version).ok_or_else(|| {
                ApiError::unprocessable(
                    "InvalidExistingVersion",
                    format!("Existing version '{}' is not valid semver", version),
                )
            })?;
            parsed.push(parsed_version);
        }
        parsed.sort();
        let latest_version = parsed.last().cloned();

        if let Some(old_version) = latest_version {
            let old_selector = format!("{}@{}", contract_id, old_version);
            let old_abi = resolve_abi(&state, &old_selector).await?;
            let old_spec = crate::type_safety::parser::parse_json_spec(&old_abi, &contract_id)
                .map_err(|e| ApiError::bad_request("InvalidABI", format!("Failed to parse old ABI: {}", e)))?;

            let new_spec = crate::type_safety::parser::parse_json_spec(&req.abi.to_string(), &contract_id)
                .map_err(|e| ApiError::bad_request("InvalidABI", format!("Failed to parse new ABI: {}", e)))?;

            let changes = diff_abi(&old_spec, &new_spec);
            if has_breaking_changes(&changes) && new_version.major == old_version.major {
                return Err(ApiError::unprocessable(
                    "BreakingChangeWithoutMajorBump",
                    format!(
                        "Breaking changes detected; bump major version from {} to {}",
                        old_version, new_version
                    ),
                ));
            }
        }
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin transaction", err))?;

    let version_row: ContractVersion = sqlx::query_as(
        "INSERT INTO contract_versions (contract_id, version, wasm_hash, source_url, commit_hash, release_notes) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING *",
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&req.wasm_hash)
    .bind(&req.source_url)
    .bind(&req.commit_hash)
    .bind(&req.release_notes)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err)
            if db_err.constraint() == Some("contract_versions_contract_id_version_key") =>
        {
            ApiError::unprocessable(
                "VersionAlreadyExists",
                format!("Version '{}' already exists for this contract", req.version),
            )
        }
        _ => db_internal_error("insert contract version", err),
    })?;

    sqlx::query(
        "INSERT INTO contract_abis (contract_id, version, abi) VALUES ($1, $2, $3) \
         ON CONFLICT (contract_id, version) DO UPDATE SET abi = EXCLUDED.abi",
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&req.abi)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("insert contract abi", err))?;

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit contract version", err))?;

    state
        .webhook_dispatcher
        .emit(
            crate::webhooks::WebhookEventType::ContractUpdated,
            Some(contract_id),
            json!({
                "contract_uuid": contract_uuid,
                "version": version_row.version,
                "wasm_hash": version_row.wasm_hash,
            }),
        )
        .await;

    Ok(Json(version_row))
}

async fn fetch_contract_identity(state: &AppState, id: &str) -> ApiResult<(Uuid, String)> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let row = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, contract_id FROM contracts WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract", err))?;
        return row.ok_or_else(|| ApiError::not_found("ContractNotFound", format!("No contract found with ID: {}", id)));
    }

    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, contract_id FROM contracts WHERE contract_id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch contract", err))?;

    row.ok_or_else(|| ApiError::not_found("ContractNotFound", format!("No contract found with ID: {}", id)))
}

#[utoipa::path(
    post,
    path = "/contracts",
    summary = "Publish contract",
    description = "Registers a contract and its publisher in the registry.",
    tag = "Contracts",
    request_body = shared::PublishRequest,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Created contract", body = Contract),
        (status = 400, description = "Invalid request", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn publish_contract(
    State(state): State<AppState>,
    payload: Result<Json<PublishRequest>, JsonRejection>,
) -> ApiResult<Json<Contract>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    let publisher: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address) VALUES ($1)
         ON CONFLICT (stellar_address) DO UPDATE SET stellar_address = EXCLUDED.stellar_address
         RETURNING *"
    )
    .bind(&req.publisher_address)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("upsert publisher", err))?;

    let wasm_hash = "placeholder_hash".to_string();

    let contract: Contract = sqlx::query_as(
        "INSERT INTO contracts (contract_id, wasm_hash, name, description, publisher_id, network, category, tags)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING *"
    )
    .bind(&req.contract_id)
    .bind(&wasm_hash)
    .bind(&req.name)
    .bind(&req.description)
    .bind(publisher.id)
    .bind(&req.network)
    .bind(&req.category)
    .bind(&req.tags)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create contract", err))?;

    Ok(Json(contract))
}

#[utoipa::path(
    post,
    path = "/publishers",
    summary = "Create publisher",
    description = "Creates a publisher record.",
    tag = "Publishers",
    request_body = Publisher,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Publisher created", body = Publisher),
        (status = 400, description = "Invalid request", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn create_publisher(
    State(state): State<AppState>,
    payload: Result<Json<Publisher>, JsonRejection>,
) -> ApiResult<Json<Publisher>> {
    let Json(publisher) = payload.map_err(map_json_rejection)?;

    let created: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address, username, email, github_url, website)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *"
    )
    .bind(&publisher.stellar_address)
    .bind(&publisher.username)
    .bind(&publisher.email)
    .bind(&publisher.github_url)
    .bind(&publisher.website)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create publisher", err))?;

    Ok(Json(created))
}

#[utoipa::path(
    get,
    path = "/publishers/{id}",
    summary = "Get publisher",
    description = "Fetches a publisher by UUID.",
    tag = "Publishers",
    params(
        ("id" = String, Path, description = "Publisher UUID")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Publisher", body = Publisher),
        (status = 400, description = "Invalid publisher id", body = crate::openapi::ErrorBody),
        (status = 404, description = "Not found", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_publisher(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Publisher>> {
    let publisher_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidPublisherId",
            format!("Invalid publisher ID format: {}", id),
        )
    })?;

    let publisher: Publisher = sqlx::query_as("SELECT * FROM publishers WHERE id = $1")
        .bind(publisher_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "PublisherNotFound",
                format!("No publisher found with ID: {}", id),
            ),
            _ => db_internal_error("get publisher by id", err),
        })?;

    Ok(Json(publisher))
}

#[utoipa::path(
    get,
    path = "/publishers/{id}/contracts",
    summary = "List publisher contracts",
    description = "Lists contracts published by a publisher.",
    tag = "Publishers",
    params(
        ("id" = String, Path, description = "Publisher UUID")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Publisher contracts", body = [Contract]),
        (status = 400, description = "Invalid publisher id", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_publisher_contracts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<Contract>>> {
    let publisher_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidPublisherId",
            format!("Invalid publisher ID format: {}", id),
        )
    })?;

    let contracts: Vec<Contract> = sqlx::query_as(
        "SELECT * FROM contracts WHERE publisher_id = $1 ORDER BY created_at DESC",
    )
    .bind(publisher_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("get publisher contracts", err))?;

    Ok(Json(contracts))
}

// Stubs for upstream added endpoints
#[utoipa::path(
    get,
    path = "/contracts/{id}/abi",
    summary = "Get contract ABI",
    description = "Returns the contract ABI for the current version (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "ABI response", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_abi() -> impl IntoResponse {
    Json(json!({"abi": null}))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/state/{key}",
    summary = "Get contract state",
    description = "Reads a contract state key (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id"),
        ("key" = String, Path, description = "State key")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "State", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_state() -> impl IntoResponse {
    Json(json!({"state": {}}))
}

#[utoipa::path(
    post,
    path = "/contracts/{id}/state/{key}",
    summary = "Update contract state",
    description = "Updates a contract state key (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id"),
        ("key" = String, Path, description = "State key")
    ),
    request_body = Value,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Update result", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn update_contract_state() -> impl IntoResponse {
    Json(json!({"success": true}))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/analytics",
    summary = "Contract analytics",
    description = "Returns contract analytics (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Analytics response", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_analytics() -> impl IntoResponse {
    Json(json!({"analytics": {}}))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/trust-score",
    summary = "Trust score",
    description = "Returns a trust score for the contract (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Trust score", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_trust_score() -> impl IntoResponse {
    Json(json!({"score": 0}))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/dependencies",
    summary = "Dependencies",
    description = "Returns direct dependencies of the contract (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Dependencies", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_dependencies() -> impl IntoResponse {
    Json(json!({"dependencies": []}))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/dependents",
    summary = "Dependents",
    description = "Returns dependents of the contract (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Dependents", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_dependents() -> impl IntoResponse {
    Json(json!({"dependents": []}))
}

#[utoipa::path(
    get,
    path = "/contracts/graph",
    summary = "Contract graph",
    description = "Returns a dependency graph view (MVP stub).",
    tag = "Contracts",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Graph response", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_graph() -> impl IntoResponse {
    Json(json!({"graph": {}}))
}

#[utoipa::path(
    get,
    path = "/contracts/trending",
    summary = "Trending contracts",
    description = "Returns trending contracts (MVP stub).",
    tag = "Contracts",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Trending response", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_trending_contracts() -> impl IntoResponse {
    Json(json!({"trending": []}))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/deployments/status",
    summary = "Deployment status",
    description = "Returns deployment status (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Status response", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_deployment_status() -> impl IntoResponse {
    Json(json!({"status": "pending"}))
}

#[derive(serde::Deserialize)]
pub struct DeployGreenRequest {
    pub contract_id: Option<String>,
}

#[utoipa::path(
    post,
    path = "/contracts/verify",
    summary = "Verify contract",
    description = "Marks a contract as verified and emits a `contract_verified` webhook event.",
    tag = "Contracts",
    request_body = shared::VerifyRequest,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Verification result", body = Value),
        (status = 400, description = "Invalid request", body = crate::openapi::ErrorBody),
        (status = 404, description = "Contract not found", body = crate::openapi::ErrorBody),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody),
        (status = 500, description = "Server error", body = crate::openapi::ErrorBody)
    )
)]
pub async fn verify_contract(
    State(state): State<AppState>,
    payload: Result<Json<VerifyRequest>, JsonRejection>,
) -> axum::response::Response {
    let Json(req) = match payload {
        Ok(v) => v,
        Err(err) => return map_json_rejection(err).into_response(),
    };

    let contract_uuid: Option<Uuid> = match sqlx::query_scalar::<_, Uuid>(
        "UPDATE contracts SET is_verified = true, updated_at = NOW() WHERE contract_id = $1 RETURNING id",
    )
    .bind(&req.contract_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(v) => v,
        Err(err) => return db_internal_error("verify contract", err).into_response(),
    };

    let Some(contract_uuid) = contract_uuid else {
        return ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with contract_id: {}", req.contract_id),
        )
        .into_response();
    };

    state
        .webhook_dispatcher
        .emit(
            crate::webhooks::WebhookEventType::ContractVerified,
            Some(req.contract_id.clone()),
            json!({
                "contract_uuid": contract_uuid,
                "compiler_version": req.compiler_version,
            }),
        )
        .await;

    Json(json!({"verified": true, "contract_uuid": contract_uuid})).into_response()
}

#[utoipa::path(
    post,
    path = "/deployments/green",
    summary = "Deploy green",
    description = "Triggers a green deployment (MVP stub) and emits a `contract_deployed` webhook event when contract_id is provided.",
    tag = "Contracts",
    request_body = DeployGreenRequest,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Deployment result", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn deploy_green(
    State(state): State<AppState>,
    payload: Option<Json<DeployGreenRequest>>,
) -> impl IntoResponse {
    let contract_id = payload.and_then(|p| p.contract_id);
    if let Some(contract_id) = contract_id {
        state
            .webhook_dispatcher
            .emit(
                crate::webhooks::WebhookEventType::ContractDeployed,
                Some(contract_id),
                json!({"deployment": "green"}),
            )
            .await;
    }

    Json(json!({"deployment_id": ""}))
}

#[utoipa::path(
    get,
    path = "/contracts/{id}/performance",
    summary = "Contract performance",
    description = "Returns performance metrics (MVP stub).",
    tag = "Contracts",
    params(
        ("id" = String, Path, description = "Contract id")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Performance response", body = Value),
        (status = 429, description = "Rate limited", body = crate::openapi::ErrorBody)
    )
)]
pub async fn get_contract_performance() -> impl IntoResponse {
    Json(json!({"performance": {}}))
}

pub async fn route_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({"error": "Route not found"})))
}
