use axum::{
    response::{IntoResponse, Redirect},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use utoipa::{
    openapi::{
        security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
        OpenApi as OpenApiSpec, Server,
    },
    Modify, OpenApi, ToSchema,
};
use utoipa_swagger_ui::{SwaggerUi, Url};

use crate::{api_versioning::ApiVersionMetricsSnapshot, breaking_changes, deprecation_handlers, handlers, webhooks};
use shared::{Contract, ContractVersion, Publisher};

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    pub code: u16,
    pub timestamp: String,
    pub correlation_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedContractsResponse {
    #[serde(rename = "contracts")]
    pub items: Vec<Contract>,
    pub total: i64,
    pub page: i64,
    #[serde(rename = "pages")]
    pub total_pages: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    pub total_contracts: i64,
    pub verified_contracts: i64,
    pub total_publishers: i64,
    pub api_versions: ApiVersionMetricsSnapshot,
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut OpenApiSpec) {
        let Some(components) = openapi.components.as_mut() else {
            return;
        };
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::list_contracts,
        handlers::publish_contract,
        handlers::get_contract,
        handlers::get_contract_versions,
        handlers::create_contract_version,
        handlers::get_trending_contracts,
        handlers::get_contract_graph,
        handlers::get_contract_abi,
        handlers::get_contract_state,
        handlers::update_contract_state,
        handlers::get_contract_analytics,
        handlers::get_trust_score,
        handlers::get_contract_dependencies,
        handlers::get_contract_dependents,
        handlers::verify_contract,
        handlers::get_contract_performance,
        handlers::get_deployment_status,
        handlers::deploy_green,
        breaking_changes::get_breaking_changes,
        deprecation_handlers::get_deprecation_info,
        deprecation_handlers::deprecate_contract,
        handlers::create_publisher,
        handlers::get_publisher,
        handlers::get_publisher_contracts,
        handlers::get_stats,
        webhooks::create_webhook,
        webhooks::list_webhooks,
        webhooks::get_webhook,
        webhooks::list_deliveries,
        webhooks::list_dead_letters,
        webhooks::retry_dead_letter
    ),
    components(
        schemas(
            Contract,
            ContractVersion,
            Publisher,
            shared::ContractSearchParams,
            shared::PublishRequest,
            shared::CreateContractVersionRequest,
            shared::DeprecationInfo,
            shared::DeprecateContractRequest,
            shared::VerifyRequest,
            shared::Network,
            shared::SortBy,
            shared::SortOrder,
            breaking_changes::ChangeSeverity,
            breaking_changes::BreakingChange,
            breaking_changes::BreakingChangeReport,
            breaking_changes::BreakingChangeQuery,
            PaginatedContractsResponse,
            StatsResponse,
            ErrorBody,
            webhooks::WebhookEventType,
            webhooks::CreateWebhookRequest,
            webhooks::WebhookEndpointPublic,
            webhooks::WebhookEndpointCreated,
            webhooks::WebhookDelivery,
            webhooks::WebhookDeadLetter
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Contracts", description = "Contract discovery, publishing, versions and verification"),
        (name = "Publishers", description = "Publisher registration and lookup"),
        (name = "Webhooks", description = "Webhook registration and delivery inspection"),
        (name = "System", description = "System statistics and operational endpoints")
    ),
    security(
        (),
        ("bearerAuth" = [])
    )
)]
pub struct VersionedApiDoc;

fn versioned_spec(server_url: &str, title: &str, version: &str) -> OpenApiSpec {
    let mut api = VersionedApiDoc::openapi();
    api.servers = Some(vec![Server::new(server_url)]);
    api.info.title = title.to_string();
    api.info.version = version.to_string();
    api.info.description = Some(
        "Auth: Bearer tokens are documented for forward-compatibility.\n\nRate limits: responses include `x-ratelimit-*` headers; on 429 you also get `Retry-After`.\n\nChangelog: see CHANGELOG.md.\n\nMigration: docs/migrations/v1-to-v2.md."
            .to_string(),
    );
    api
}

#[derive(OpenApi)]
#[openapi(
    paths(handlers::health_check),
    components(schemas(Value, ErrorBody)),
    tags((name = "Health", description = "Health check endpoints")),
    info(title = "Soroban Registry API (public)", version = "1.0.0")
)]
pub struct PublicDoc;

async fn openapi_v1() -> impl IntoResponse {
    Json(versioned_spec(
        "/api/v1",
        "Soroban Registry API (v1)",
        "1.0.0",
    ))
}

async fn openapi_v2() -> impl IntoResponse {
    Json(versioned_spec(
        "/api/v2",
        "Soroban Registry API (v2)",
        "2.0.0",
    ))
}

async fn openapi_public() -> impl IntoResponse {
    Json(PublicDoc::openapi())
}

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/openapi.json", get(openapi_v1))
        .route("/api/v1/openapi.json", get(openapi_v1))
        .route("/api/v2/openapi.json", get(openapi_v2))
        .route("/openapi.public.json", get(openapi_public))
        .route("/swagger", get(|| async { Redirect::permanent("/api/docs") }))
        .route("/swagger-ui", get(|| async { Redirect::permanent("/api/docs") }))
        .merge(
            SwaggerUi::new("/api/docs").urls(vec![
                Url::new("public", "/openapi.public.json"),
                Url::new("v1", "/api/v1/openapi.json"),
                Url::new("v2", "/api/v2/openapi.json"),
            ]),
        )
}

