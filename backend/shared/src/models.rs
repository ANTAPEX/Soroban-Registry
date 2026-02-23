use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════
// EXISTING REGISTRY TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Represents a smart contract in the registry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Contract {
    pub id: Uuid,
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_id: Uuid,
    pub network: Network,
    pub is_verified: bool,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub is_maintenance: bool,
    /// Groups rows that represent the same logical contract across networks (Issue #43)
    #[serde(default)]
    pub logical_id: Option<Uuid>,
    /// Per-network config: { "mainnet": { contract_id, is_verified, min_version, max_version }, ... }
    #[serde(default)]
    pub network_configs: Option<serde_json::Value>,
}

/// Response for GET /contracts/:id with optional network-specific slice (Issue #43)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractGetResponse {
    #[serde(flatten)]
    pub contract: Contract,
    /// When ?network= is set, the requested network
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_network: Option<Network>,
    /// When ?network= is set, that network's config slice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_config: Option<NetworkConfig>,
}

/// Per-network config: address, verified status, min/max version (Issue #43)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub contract_id: String,
    pub is_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_version: Option<String>,
}

/// Network where the contract is deployed
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "network_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Futurenet => write!(f, "futurenet"),
        }
    }
}

/// Upgrade strategy for contract upgrades
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "upgrade_strategy_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UpgradeStrategy {
    Proxy,
    Uups,
    DataMigration,
    ShadowContract,
}

/// Contract version information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractVersion {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_schema: Option<serde_json::Value>,
}

/// Verification status and details
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Verification {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub status: VerificationStatus,
    pub source_code: Option<String>,
    pub build_params: Option<serde_json::Value>,
    pub compiler_version: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Verification status enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "verification_status", rename_all = "lowercase")]
pub enum VerificationStatus {
    Pending,
    Verified,
    Failed,
}

/// Contract maturity level - indicates stability and production readiness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaturityLevel {
    Experimental,
    Beta,
    Stable,
    Production,
}

/// Publisher/developer information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Publisher {
    pub id: Uuid,
    pub stellar_address: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub github_url: Option<String>,
    pub website: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Contract interaction statistics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractStats {
    pub contract_id: Uuid,
    pub total_deployments: i64,
    pub total_interactions: i64,
    pub unique_users: i64,
    pub last_interaction: Option<DateTime<Utc>>,
}

/// GraphNode (minimal contract info for graph rendering)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub network: Network,
    pub is_verified: bool,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

/// Graph edge (dependency relationship)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub dependency_type: String,
}

/// Full graph response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Request to publish a new contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub network: Network,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub source_url: Option<String>,
    pub publisher_address: String,
    // Dependencies (new field)
    #[serde(default)]
    pub dependencies: Vec<DependencyDeclaration>,
}

/// Request to create a new contract version with ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContractVersionRequest {
    pub contract_id: String,
    pub version: String,
    pub wasm_hash: String,
    pub abi: serde_json::Value,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Deprecation management (issue #65)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeprecationStatus {
    Active,
    Deprecated,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationInfo {
    pub contract_id: String,
    pub status: DeprecationStatus,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub retirement_at: Option<DateTime<Utc>>,
    pub replacement_contract_id: Option<String>,
    pub migration_guide_url: Option<String>,
    pub notes: Option<String>,
    pub days_remaining: Option<i64>,
    pub dependents_notified: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecateContractRequest {
    pub retirement_at: DateTime<Utc>,
    pub replacement_contract_id: Option<String>,
    pub migration_guide_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeprecationNotification {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub deprecated_contract_id: Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
}
/// Dependency declaration in publish request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyDeclaration {
    pub name: String,
    pub version_constraint: String,
}

/// Contract dependency record (database row)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractDependency {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub dependency_name: String,
    pub dependency_contract_id: Option<Uuid>,
    pub version_constraint: String,
    pub created_at: DateTime<Utc>,
}

/// Tracks migration scripts between contract versions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MigrationScript {
    pub id: Uuid,
    pub from_version: Uuid,
    pub to_version: Uuid,
    pub script_path: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
}

/// Recursive dependency tree node for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTreeNode {
    pub contract_id: String, // Public key ID
    pub name: String,
    pub current_version: String,
    pub constraint_to_parent: String,
    pub dependencies: Vec<DependencyTreeNode>,
}

/// Request to verify a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub contract_id: String,
    pub source_code: String,
    pub build_params: serde_json::Value,
    pub compiler_version: String,
}

/// Sorting options for contracts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    CreatedAt,
    UpdatedAt,
    Popularity,
    Deployments,
    Interactions,
    Relevance,
}

/// Sorting order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Search/filter parameters for contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSearchParams {
    pub query: Option<String>,
    pub network: Option<Network>,
    /// Multiple networks filter (e.g. ?network=mainnet&network=testnet)
    pub networks: Option<Vec<Network>>,
    pub verified_only: Option<bool>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub maturity: Option<MaturityLevel>,
    pub page: Option<i64>,
    #[serde(alias = "page_size")]
    pub limit: Option<i64>,
    pub sort_by: Option<SortBy>,
    pub sort_order: Option<SortOrder>,
}

/// Pagination params for contract versions (limit/offset style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPaginationParams {
    #[serde(default = "default_version_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_version_limit() -> i64 {
    20
}

/// Paginated version response (limit/offset style per issue #32 spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedVersionResponse {
    pub items: Vec<ContractVersion>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    #[serde(rename = "contracts")]
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    #[serde(rename = "pages")]
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, page: i64, limit: i64) -> Self {
        let total_pages = if limit > 0 {
            (total as f64 / limit as f64).ceil() as i64
        } else {
            0
        };
        Self {
            items,
            total,
            page,
            total_pages,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT INTERACTION HISTORY (Issue #46)
// ═══════════════════════════════════════════════════════════════════════════

/// One contract invocation row (DB)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractInteraction {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub user_address: Option<String>,
    pub interaction_type: String,
    pub transaction_hash: Option<String>,
    pub method: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub return_value: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Response item for GET /api/contracts/:id/interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInteractionResponse {
    pub id: Uuid,
    pub account: Option<String>,
    pub method: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub return_value: Option<serde_json::Value>,
    pub transaction_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Query params for GET /api/contracts/:id/interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionsQueryParams {
    #[serde(default = "default_interactions_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub account: Option<String>,
    pub method: Option<String>,
    pub from_timestamp: Option<String>,
    pub to_timestamp: Option<String>,
}

fn default_interactions_limit() -> i64 {
    50
}

/// Request body for POST /api/contracts/:id/interactions (single)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInteractionRequest {
    pub account: Option<String>,
    pub method: Option<String>,
    pub transaction_hash: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub return_value: Option<serde_json::Value>,
    pub timestamp: Option<DateTime<Utc>>,
}

/// Request body for POST /api/contracts/:id/interactions/batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInteractionBatchRequest {
    pub interactions: Vec<CreateInteractionRequest>,
}

/// Paginated interactions response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionsListResponse {
    pub items: Vec<ContractInteractionResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "migration_status", rename_all = "snake_case")]
pub enum MigrationStatus {
    Pending,
    Success,
    Failed,
    RolledBack,
}

/// Represents a contract state migration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Migration {
    pub id: Uuid,
    pub contract_id: String,
    pub status: MigrationStatus,
    pub wasm_hash: String,
    pub log_output: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new migration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMigrationRequest {
    pub contract_id: String,
    pub wasm_hash: String,
}

/// Request to update a migration's status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMigrationStatusRequest {
    pub status: MigrationStatus,
    pub log_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "deployment_environment", rename_all = "lowercase")]
pub enum DeploymentEnvironment {
    Blue,
    Green,
}

impl std::fmt::Display for DeploymentEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentEnvironment::Blue => write!(f, "blue"),
            DeploymentEnvironment::Green => write!(f, "green"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "deployment_status", rename_all = "lowercase")]
pub enum DeploymentStatus {
    Active,
    Inactive,
    Testing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractDeployment {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub environment: DeploymentEnvironment,
    pub status: DeploymentStatus,
    pub wasm_hash: String,
    pub deployed_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub health_checks_passed: i32,
    pub health_checks_failed: i32,
    pub last_health_check_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeploymentSwitch {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub from_environment: DeploymentEnvironment,
    pub to_environment: DeploymentEnvironment,
    pub switched_at: DateTime<Utc>,
    pub switched_by: Option<String>,
    pub rollback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "canary_status", rename_all = "snake_case")]
pub enum CanaryStatus {
    Pending,
    Active,
    Paused,
    Completed,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "rollout_stage", rename_all = "snake_case")]
pub enum RolloutStage {
    Stage1,
    Stage2,
    Stage3,
    Stage4,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CanaryRelease {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub from_deployment_id: Option<Uuid>,
    pub to_deployment_id: Uuid,
    pub status: CanaryStatus,
    pub current_stage: RolloutStage,
    pub current_percentage: i32,
    pub target_percentage: i32,
    pub error_rate_threshold: Decimal,
    pub current_error_rate: Option<Decimal>,
    pub total_requests: i32,
    pub error_count: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CanaryMetric {
    pub id: Uuid,
    pub canary_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub requests: i32,
    pub errors: i32,
    pub error_rate: rust_decimal::Decimal,
    pub avg_response_time_ms: Option<Decimal>,
    pub p95_response_time_ms: Option<Decimal>,
    pub p99_response_time_ms: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CanaryUserAssignment {
    pub id: Uuid,
    pub canary_id: Uuid,
    pub user_address: String,
    pub assigned_at: DateTime<Utc>,
    pub notified: bool,
    pub notified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCanaryRequest {
    pub contract_id: String,
    pub to_deployment_id: String,
    pub error_rate_threshold: Option<f64>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceCanaryRequest {
    pub canary_id: String,
    pub target_percentage: Option<i32>,
    pub advanced_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCanaryMetricRequest {
    pub canary_id: String,
    pub requests: i32,
    pub errors: i32,
    pub avg_response_time_ms: Option<f64>,
    pub p95_response_time_ms: Option<f64>,
    pub p99_response_time_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "ab_test_status", rename_all = "snake_case")]
pub enum AbTestStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "variant_type", rename_all = "snake_case")]
pub enum VariantType {
    Control,
    Treatment,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AbTest {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: AbTestStatus,
    pub traffic_split: Decimal,
    pub variant_a_deployment_id: Uuid,
    pub variant_b_deployment_id: Uuid,
    pub primary_metric: String,
    pub hypothesis: Option<String>,
    pub significance_threshold: Decimal,
    pub min_sample_size: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AbTestVariant {
    pub id: Uuid,
    pub test_id: Uuid,
    pub variant_type: VariantType,
    pub deployment_id: Uuid,
    pub traffic_percentage: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AbTestAssignment {
    pub id: Uuid,
    pub test_id: Uuid,
    pub user_address: String,
    pub variant_type: VariantType,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AbTestMetric {
    pub id: Uuid,
    pub test_id: Uuid,
    pub variant_type: VariantType,
    pub metric_name: String,
    pub metric_value: Decimal,
    pub user_address: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AbTestResult {
    pub id: Uuid,
    pub test_id: Uuid,
    pub variant_type: VariantType,
    pub sample_size: i32,
    pub mean_value: Option<Decimal>,
    pub std_deviation: Option<Decimal>,
    pub confidence_interval_lower: Option<Decimal>,
    pub confidence_interval_upper: Option<Decimal>,
    pub p_value: Option<Decimal>,
    pub statistical_significance: Option<Decimal>,
    pub is_winner: bool,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAbTestRequest {
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub traffic_split: Option<f64>,
    pub variant_a_deployment_id: String,
    pub variant_b_deployment_id: String,
    pub primary_metric: String,
    pub hypothesis: Option<String>,
    pub significance_threshold: Option<f64>,
    pub min_sample_size: Option<i32>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordAbTestMetricRequest {
    pub test_id: String,
    pub user_address: Option<String>,
    pub metric_name: String,
    pub metric_value: f64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVariantRequest {
    pub test_id: String,
    pub user_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "metric_type", rename_all = "snake_case")]
pub enum MetricType {
    ExecutionTime,
    MemoryUsage,
    StorageIo,
    GasConsumption,
    ErrorRate,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "alert_severity", rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceMetric {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub function_name: Option<String>,
    pub value: Decimal,
    pub p50: Option<Decimal>,
    pub p95: Option<Decimal>,
    pub p99: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceAnomaly {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub function_name: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub baseline_value: Option<Decimal>,
    pub current_value: Option<Decimal>,
    pub deviation_percent: Option<Decimal>,
    pub severity: AlertSeverity,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceAlert {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub threshold_type: String,
    pub threshold_value: Decimal,
    pub current_value: Decimal,
    pub severity: AlertSeverity,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceTrend {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub function_name: Option<String>,
    pub metric_type: MetricType,
    pub timeframe_start: DateTime<Utc>,
    pub timeframe_end: DateTime<Utc>,
    pub avg_value: Option<Decimal>,
    pub min_value: Option<Decimal>,
    pub max_value: Option<Decimal>,
    pub p50_value: Option<Decimal>,
    pub p95_value: Option<Decimal>,
    pub p99_value: Option<Decimal>,
    pub sample_count: i32,
    pub trend_direction: Option<String>,
    pub change_percent: Option<Decimal>,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceAlertConfig {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub threshold_type: String,
    pub threshold_value: Decimal,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordPerformanceMetricRequest {
    pub contract_id: String,
    pub metric_type: MetricType,
    pub function_name: Option<String>,
    pub value: f64,
    pub p50: Option<f64>,
    pub p95: Option<f64>,
    pub p99: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertConfigRequest {
    pub contract_id: String,
    pub metric_type: MetricType,
    pub threshold_type: String,
    pub threshold_value: f64,
    pub severity: Option<AlertSeverity>,
}

// ────────────────────────────────────────────────────────────────────────────
// Custom contract metrics (issue #89)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "custom_metric_type", rename_all = "snake_case")]
pub enum CustomMetricType {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomMetric {
    pub id: Uuid,
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: CustomMetricType,
    pub value: Decimal,
    pub unit: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub ledger_sequence: Option<i64>,
    pub transaction_hash: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub network: Network,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCustomMetricRequest {
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: CustomMetricType,
    pub value: f64,
    pub unit: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub ledger_sequence: Option<i64>,
    pub transaction_hash: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub network: Option<Network>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomMetricAggregate {
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: CustomMetricType,
    pub bucket_start: DateTime<Utc>,
    pub bucket_end: DateTime<Utc>,
    pub sample_count: i32,
    pub sum_value: Option<Decimal>,
    pub avg_value: Option<Decimal>,
    pub min_value: Option<Decimal>,
    pub max_value: Option<Decimal>,
    pub p50_value: Option<Decimal>,
    pub p95_value: Option<Decimal>,
    pub p99_value: Option<Decimal>,
}

// ────────────────────────────────────────────────────────────────────────────
// Analytics models
// ────────────────────────────────────────────────────────────────────────────

/// Types of analytics events tracked by the system
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "analytics_event_type", rename_all = "snake_case")]
pub enum AnalyticsEventType {
    ContractPublished,
    ContractVerified,
    ContractDeployed,
    VersionCreated,
}

impl std::fmt::Display for AnalyticsEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContractPublished => write!(f, "contract_published"),
            Self::ContractVerified => write!(f, "contract_verified"),
            Self::ContractDeployed => write!(f, "contract_deployed"),
            Self::VersionCreated => write!(f, "version_created"),
        }
    }
}

/// A raw analytics event recorded when a contract lifecycle action occurs
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalyticsEvent {
    pub id: Uuid,
    pub event_type: AnalyticsEventType,
    pub contract_id: Uuid,
    pub user_address: Option<String>,
    pub network: Option<Network>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Pre-computed daily aggregate for a single contract
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DailyAggregate {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub date: chrono::NaiveDate,
    pub deployment_count: i32,
    pub unique_deployers: i32,
    pub verification_count: i32,
    pub publish_count: i32,
    pub version_count: i32,
    pub total_events: i32,
    pub unique_users: i32,
    pub network_breakdown: serde_json::Value,
    pub top_users: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ────────────────────────────────────────────────────────────────────────────
// Analytics API response DTOs
// ────────────────────────────────────────────────────────────────────────────

/// Top-level response for GET /api/contracts/:id/analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAnalyticsResponse {
    pub contract_id: Uuid,
    pub deployments: DeploymentStats,
    pub interactors: InteractorStats,
    pub timeline: Vec<TimelineEntry>,
}

/// Deployment statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStats {
    pub count: i64,
    pub unique_users: i64,
    pub by_network: serde_json::Value,
}

/// Interactor / unique-user statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractorStats {
    pub unique_count: i64,
    pub top_users: Vec<TopUser>,
}

/// A user ranked by interaction count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopUser {
    pub address: String,
    pub count: i64,
}

/// One data-point in the 30-day timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployGreenRequest {
    pub contract_id: String,
    pub wasm_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchDeploymentRequest {
    pub contract_id: String,
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRequest {
    pub contract_id: String,
    pub environment: DeploymentEnvironment,
    pub passed: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// POPULARITY / TRENDING
// ═══════════════════════════════════════════════════════════════════════════

/// Query parameters for the trending contracts endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingParams {
    /// Max results to return (default 10, max 50)
    pub limit: Option<i64>,
    /// Timeframe for trending calculation: "7d", "30d", "90d" (default "7d")
    pub timeframe: Option<String>,
}

/// Response DTO for a trending contract
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrendingContract {
    // Core contract fields
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub network: Network,
    pub is_verified: bool,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    // Popularity metrics
    pub popularity_score: f64,
    pub deployment_count: i64,
    pub interaction_count: i64,
}

// MULTI-SIGNATURE DEPLOYMENT TYPES  (issue #47)
// ═══════════════════════════════════════════════════════════════════════════
// ════════════════════════════════════════════════════════════════════════════
// Audit Log & Version History types
// ════════════════════════════════════════════════════════════════════════════

/// The type of mutation that triggered an audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "audit_action_type", rename_all = "snake_case")]
pub enum AuditActionType {
    ContractPublished,
    MetadataUpdated,
    VerificationChanged,
    PublisherChanged,
    VersionCreated,
    Rollback,
}

impl std::fmt::Display for AuditActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::ContractPublished => "contract_published",
            Self::MetadataUpdated => "metadata_updated",
            Self::VerificationChanged => "verification_changed",
            Self::PublisherChanged => "publisher_changed",
            Self::VersionCreated => "version_created",
            Self::Rollback => "rollback",
        };
        write!(f, "{}", s)
    }
}

/// One immutable row in `contract_audit_log`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractAuditLog {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub action_type: AuditActionType,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub changed_by: String,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: Option<String>,
    pub hash: Option<String>,
    pub signature: Option<String>,
}

/// Full contract state captured at each audited change in `contract_snapshots`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractSnapshot {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version_number: i32,
    pub snapshot_data: serde_json::Value,
    pub audit_log_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// A single field-level change between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub from: serde_json::Value,
    pub to: serde_json::Value,
}

/// Response for GET /api/contracts/:id/versions/:v1/diff/:v2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub contract_id: Uuid,
    pub from_version: i32,
    pub to_version: i32,
    /// Fields present in v2 but not v1
    pub added: Vec<FieldChange>,
    /// Fields present in v1 but not v2
    pub removed: Vec<FieldChange>,
    /// Fields present in both but with different values
    pub modified: Vec<FieldChange>,
}

/// Request body for POST /api/contracts/:id/rollback/:snapshot_id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRequest {
    /// Stellar address (or admin service ID) authorising the rollback
    pub changed_by: String,
}

// Multisig deployment types
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MultisigPolicy {
    pub id: Uuid,
    pub name: String,
    pub threshold: i32,
    pub required_signatures: i32,
    pub signer_addresses: Vec<String>,
    pub expiry_seconds: i32,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeployProposal {
    pub id: Uuid,
    pub contract_name: String,
    pub contract_id: Uuid,
    pub wasm_hash: String,
    pub network: String,
    pub description: Option<String>,
    pub policy_id: Uuid,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub proposer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProposalSignature {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub signer_address: String,
}

/// Paginated response for audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalWithSignatures {
    pub proposal: DeployProposal,
    pub policy: MultisigPolicy,
    pub signatures: Vec<ProposalSignature>,
    pub signatures_needed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogPage {
    pub items: Vec<ContractAuditLog>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
}

// ════════════════════════════════════════════════════════════════════════════
// Config Management types
// ════════════════════════════════════════════════════════════════════════════

/// Represents a contract configuration version in the registry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractConfig {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub environment: String,
    pub version: i32,
    pub config_data: serde_json::Value,
    pub secrets_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// Request to create a new configuration version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCreateRequest {
    pub environment: String,
    pub config_data: serde_json::Value,
    pub secrets_data: Option<serde_json::Value>,
    pub created_by: String,
}

/// Request to rollback to an old configuration version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRollbackRequest {
    pub roll_back_to_version: i32,
    pub created_by: String,
}

/// Response object for returning configurations (without secrets_data when returning publicly)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfigResponse {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub environment: String,
    pub version: i32,
    pub config_data: serde_json::Value,
    pub has_secrets: bool, // Indicator instead of returning actual secrets
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl From<ContractConfig> for ContractConfigResponse {
    fn from(config: ContractConfig) -> Self {
        Self {
            id: config.id,
            contract_id: config.contract_id,
            environment: config.environment,
            version: config.version,
            config_data: config.config_data,
            has_secrets: config.secrets_data.is_some(),
            created_at: config.created_at,
            created_by: config.created_by,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DATA RESIDENCY CONTROLS  (issue #100)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "residency_decision", rename_all = "lowercase")]
pub enum ResidencyDecision {
    Allowed,
    Denied,
}

impl std::fmt::Display for ResidencyDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allowed => write!(f, "allowed"),
            Self::Denied => write!(f, "denied"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResidencyPolicy {
    pub id: Uuid,
    pub contract_id: String,
    pub allowed_regions: Vec<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResidencyAuditLog {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub contract_id: String,
    pub requested_region: String,
    pub decision: ResidencyDecision,
    pub action: String,
    pub requested_by: Option<String>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResidencyViolation {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub contract_id: String,
    pub attempted_region: String,
    pub action: String,
    pub attempted_by: Option<String>,
    pub prevented_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResidencyPolicyRequest {
    pub contract_id: String,
    pub allowed_regions: Vec<String>,
    pub description: Option<String>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResidencyPolicyRequest {
    pub allowed_regions: Option<Vec<String>>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResidencyRequest {
    pub policy_id: Uuid,
    pub contract_id: String,
    pub requested_region: String,
    pub action: String,
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResidencyLogsParams {
    pub contract_id: Option<String>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT EVENT TYPES (issue #44)
// ═══════════════════════════════════════════════════════════════════════════

/// A contract event emitted during execution
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractEvent {
    pub id: Uuid,
    pub contract_id: String,
    pub topic: String,
    pub data: Option<serde_json::Value>,
    pub ledger_sequence: i64,
    pub transaction_hash: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub network: Network,
    pub created_at: DateTime<Utc>,
}

/// Query parameters for searching events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventQueryParams {
    pub topic: Option<String>,
    pub data_pattern: Option<String>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub from_ledger: Option<i64>,
    pub to_ledger: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Request to index a new event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEventRequest {
    pub contract_id: String,
    pub topic: String,
    pub data: Option<serde_json::Value>,
    pub ledger_sequence: i64,
    pub transaction_hash: Option<String>,
    pub network: Network,
}

/// Event statistics for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    pub contract_id: String,
    pub total_events: i64,
    pub unique_topics: i64,
    pub first_event: Option<DateTime<Utc>>,
    pub last_event: Option<DateTime<Utc>>,
    pub events_by_topic: serde_json::Value,
}

/// CSV export response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventExport {
    pub contract_id: String,
    pub events: Vec<ContractEvent>,
    pub exported_at: DateTime<Utc>,
    pub total_count: i64,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT PACKAGE SIGNING (Issue #67)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "signature_status", rename_all = "lowercase")]
pub enum SignatureStatus {
    Valid,
    Revoked,
    Expired,
}

impl std::fmt::Display for SignatureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => write!(f, "valid"),
            Self::Revoked => write!(f, "revoked"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "transparency_entry_type", rename_all = "snake_case")]
pub enum TransparencyEntryType {
    PackageSigned,
    SignatureVerified,
    SignatureRevoked,
    KeyRotated,
}

impl std::fmt::Display for TransparencyEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackageSigned => write!(f, "package_signed"),
            Self::SignatureVerified => write!(f, "signature_verified"),
            Self::SignatureRevoked => write!(f, "signature_revoked"),
            Self::KeyRotated => write!(f, "key_rotated"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PackageSignature {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub signature: String,
    pub signing_address: String,
    pub public_key: String,
    pub algorithm: String,
    pub status: SignatureStatus,
    pub signed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
    pub revoked_by: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignPackageRequest {
    pub contract_id: String,
    pub version: String,
    pub wasm_hash: String,
    pub private_key: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySignatureRequest {
    pub contract_id: String,
    pub version: String,
    pub wasm_hash: String,
    pub signature: String,
    pub signing_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySignatureResponse {
    pub valid: bool,
    pub signature_id: Option<Uuid>,
    pub signing_address: String,
    pub signed_at: Option<DateTime<Utc>>,
    pub status: SignatureStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSignatureRequest {
    pub signature_id: String,
    pub revoked_by: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SignatureRevocation {
    pub id: Uuid,
    pub signature_id: Uuid,
    pub revoked_by: String,
    pub reason: String,
    pub revoked_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SigningKey {
    pub id: Uuid,
    pub publisher_id: Uuid,
    pub public_key: String,
    pub key_fingerprint: String,
    pub algorithm: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSigningKeyRequest {
    pub publisher_id: String,
    pub public_key: String,
    pub algorithm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TransparencyLogEntry {
    pub id: Uuid,
    pub entry_type: TransparencyEntryType,
    pub contract_id: Option<Uuid>,
    pub signature_id: Option<Uuid>,
    pub actor_address: String,
    pub previous_hash: Option<String>,
    pub entry_hash: String,
    pub payload: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfCustodyEntry {
    pub action: String,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
    pub signature_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfCustodyResponse {
    pub contract_id: String,
    pub entries: Vec<ChainOfCustodyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransparencyLogQueryParams {
    pub contract_id: Option<String>,
    pub entry_type: Option<TransparencyEntryType>,
    pub actor_address: Option<String>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ═══════════════════════════════════════════════════════════════════════════
// CHANGELOG & RELEASE HISTORY
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "changelog_change_type", rename_all = "lowercase")]
pub enum ChangelogChangeType {
    Feat,
    Fix,
    Docs,
    Style,
    Refactor,
    Perf,
    Test,
    Build,
    Ci,
    Chore,
    Revert,
    Breaking,
}

impl std::fmt::Display for ChangelogChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Feat => write!(f, "feat"),
            Self::Fix => write!(f, "fix"),
            Self::Docs => write!(f, "docs"),
            Self::Style => write!(f, "style"),
            Self::Refactor => write!(f, "refactor"),
            Self::Perf => write!(f, "perf"),
            Self::Test => write!(f, "test"),
            Self::Build => write!(f, "build"),
            Self::Ci => write!(f, "ci"),
            Self::Chore => write!(f, "chore"),
            Self::Revert => write!(f, "revert"),
            Self::Breaking => write!(f, "breaking"),
        }
    }
}

impl std::str::FromStr for ChangelogChangeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "feat" | "feature" => Ok(Self::Feat),
            "fix" | "bugfix" => Ok(Self::Fix),
            "docs" | "doc" => Ok(Self::Docs),
            "style" => Ok(Self::Style),
            "refactor" => Ok(Self::Refactor),
            "perf" | "performance" => Ok(Self::Perf),
            "test" | "tests" => Ok(Self::Test),
            "build" => Ok(Self::Build),
            "ci" => Ok(Self::Ci),
            "chore" => Ok(Self::Chore),
            "revert" => Ok(Self::Revert),
            "breaking" => Ok(Self::Breaking),
            other => Err(format!("unknown change type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractChangelog {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub title: Option<String>,
    pub release_date: DateTime<Utc>,
    pub is_prerelease: bool,
    pub markdown: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChangelogEntry {
    pub id: Uuid,
    pub changelog_id: Uuid,
    pub change_type: ChangelogChangeType,
    pub scope: Option<String>,
    pub description: String,
    pub commit_hash: Option<String>,
    pub is_breaking: bool,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogReleaseResponse {
    pub version: String,
    pub title: Option<String>,
    pub release_date: DateTime<Utc>,
    pub is_prerelease: bool,
    pub breaking_changes: Vec<ChangelogEntryResponse>,
    pub features: Vec<ChangelogEntryResponse>,
    pub fixes: Vec<ChangelogEntryResponse>,
    pub other: Vec<ChangelogEntryResponse>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntryResponse {
    pub change_type: String,
    pub scope: Option<String>,
    pub description: String,
    pub commit_hash: Option<String>,
    pub is_breaking: bool,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractChangelogResponse {
    pub contract_id: Uuid,
    pub releases: Vec<ChangelogReleaseResponse>,
    pub total_releases: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogQueryParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub include_prereleases: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateChangelogRequest {
    pub contract_id: String,
    pub version: String,
    pub title: Option<String>,
    pub commits: Vec<RawCommit>,
    pub is_prerelease: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCommit {
    pub hash: String,
    pub message: String,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionBumpRecommendation {
    pub current_version: String,
    pub recommended_version: String,
    pub bump_type: String,
    pub has_breaking_changes: bool,
    pub breaking_count: usize,
    pub feature_count: usize,
    pub fix_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "health_status", rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractHealth {
    pub contract_id: Uuid,
    pub status: HealthStatus,
    pub last_activity: DateTime<Utc>,
    pub security_score: i32,
    pub audit_date: Option<DateTime<Utc>>,
    pub total_score: i32,
    pub recommendations: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

// Backup and disaster recovery types
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractBackup {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub backup_date: chrono::NaiveDate,
    pub wasm_hash: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub state_snapshot: Option<serde_json::Value>,
    pub storage_size_bytes: Option<i64>,
    pub primary_region: Option<String>,
    pub backup_regions: Option<Vec<String>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupRestoration {
    pub id: Uuid,
    pub backup_id: Uuid,
    pub restored_by: Uuid,
    pub restore_duration_ms: i32,
    pub success: bool,
    pub restored_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBackupRequest {
    pub include_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreBackupRequest {
    pub backup_date: String,
}
