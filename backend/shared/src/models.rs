use chrono::{DateTime, Utc};
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
    #[serde(default)]
    pub maturity: MaturityLevel,
}

/// Contract maturity level
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "maturity_level", rename_all = "lowercase")]
pub enum MaturityLevel {
    #[default]
    Alpha,
    Beta,
    Stable,
    Mature,
    Legacy,
}

/// Network where the contract is deployed
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "network_type", rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
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
}

/// Request to verify a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub contract_id: String,
    pub source_code: String,
    pub build_params: serde_json::Value,
    pub compiler_version: String,
}

/// Search/filter parameters for contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSearchParams {
    pub query: Option<String>,
    pub network: Option<Network>,
    pub verified_only: Option<bool>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub maturity: Option<MaturityLevel>,
    pub page: Option<i64>,
    #[serde(alias = "page_size")]
    pub limit: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "deployment_environment", rename_all = "lowercase")]
pub enum DeploymentEnvironment {
    Blue,
    Green,
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
// MULTI-SIGNATURE DEPLOYMENT TYPES  (issue #47)
// ═══════════════════════════════════════════════════════════════════════════

/// Lifecycle of a multi-sig deployment proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "proposal_status", rename_all = "lowercase")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Executed,
    Expired,
    Rejected,
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ProposalStatus::Pending => "pending",
            ProposalStatus::Approved => "approved",
            ProposalStatus::Executed => "executed",
            ProposalStatus::Expired => "expired",
            ProposalStatus::Rejected => "rejected",
        };
        write!(f, "{}", s)
    }
}

/// A multi-sig policy defining signers and required threshold
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MultisigPolicy {
    pub id: Uuid,
    pub name: String,
    /// Minimum number of signatures required (M in M-of-N)
    pub threshold: i32,
    /// Stellar addresses authorised to sign proposals using this policy
    pub signer_addresses: Vec<String>,
    /// How long (seconds) a proposal under this policy stays valid
    pub expiry_seconds: i32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

/// A pending (or resolved) deployment proposal
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeployProposal {
    pub id: Uuid,
    pub contract_name: String,
    pub contract_id: String,
    pub wasm_hash: String,
    pub network: Network,
    pub description: Option<String>,
    pub policy_id: Uuid,
    pub status: ProposalStatus,
    pub expires_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    pub proposer: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single signature on a deployment proposal
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProposalSignature {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub signer_address: String,
    pub signature_data: Option<String>,
    pub signed_at: DateTime<Utc>,
}

// ── Request / Response DTOs ───────────────────────────────────────────────

/// POST /api/multisig/policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    /// M-of-N threshold (must be ≥ 1 and ≤ number of signers)
    pub threshold: i32,
    /// Comma-separated list acceptable; server always stores as Vec<String>
    pub signer_addresses: Vec<String>,
    /// Seconds until unsigned proposals expire (default: 86400 = 24 h)
    pub expiry_seconds: Option<i32>,
    pub created_by: String,
}

/// POST /api/contracts/deploy-proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposalRequest {
    pub contract_name: String,
    pub contract_id: String,
    pub wasm_hash: String,
    pub network: Network,
    pub description: Option<String>,
    pub policy_id: Uuid,
    pub proposer: String,
}

/// POST /api/contracts/{id}/sign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignProposalRequest {
    pub signer_address: String,
    /// Optional raw signature bytes (hex-encoded) for off-chain validation
    pub signature_data: Option<String>,
}

/// Rich response combining a proposal with its signatures and policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalWithSignatures {
    pub proposal: DeployProposal,
    pub policy: MultisigPolicy,
    pub signatures: Vec<ProposalSignature>,
    /// How many more signatures are needed to reach the threshold
    pub signatures_needed: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMigrationStatusRequest {
    pub status: MigrationStatus,
    pub log_output: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// MAINTENANCE MODE
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MaintenanceWindow {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub message: String,
    pub started_at: DateTime<Utc>,
    pub scheduled_end_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMaintenanceRequest {
    pub message: String,
    pub scheduled_end_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceStatusResponse {
    pub is_maintenance: bool,
    pub current_window: Option<MaintenanceWindow>,
}

// ═══════════════════════════════════════════════════════════════════════════
// MATURITY LEVELS
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MaturityChange {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub from_level: Option<MaturityLevel>,
    pub to_level: MaturityLevel,
    pub reason: Option<String>,
    pub changed_by: Uuid,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMaturityRequest {
    pub maturity: MaturityLevel,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaturityRequirements {
    pub level: MaturityLevel,
    pub criteria: Vec<MaturityCriterion>,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaturityCriterion {
    pub name: String,
    pub required: bool,
    pub met: bool,
    pub description: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// COST ESTIMATION
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimateRequest {
    pub method_name: String,
    pub invocations: Option<i64>,
    pub storage_growth_kb: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub method_name: String,
    pub gas_cost: i64,
    pub storage_cost: i64,
    pub bandwidth_cost: i64,
    pub total_stroops: i64,
    pub total_xlm: f64,
    pub invocations: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCostEstimate {
    pub estimates: Vec<CostEstimate>,
    pub total_stroops: i64,
    pub total_xlm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimization {
    pub current_cost: i64,
    pub optimized_cost: i64,
    pub savings_percent: f64,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostForecast {
    pub daily_cost_xlm: f64,
    pub monthly_cost_xlm: f64,
    pub yearly_cost_xlm: f64,
    pub usage_pattern: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// BACKUP SYSTEM
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractBackup {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub backup_date: chrono::NaiveDate,
    pub wasm_hash: String,
    pub metadata: serde_json::Value,
    pub state_snapshot: Option<serde_json::Value>,
    pub storage_size_bytes: i64,
    pub verified: bool,
    pub primary_region: String,
    pub backup_regions: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBackupRequest {
    pub include_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreBackupRequest {
    pub backup_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoration {
    pub id: Uuid,
    pub backup_id: Uuid,
    pub restored_by: Uuid,
    pub restore_duration_ms: i32,
    pub success: bool,
    pub error_message: Option<String>,
    pub restored_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════════════════
// GOVERNANCE FRAMEWORK
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "governance_model", rename_all = "snake_case")]
pub enum GovernanceModel {
    TokenWeighted,
    Quadratic,
    Multisig,
    Timelock,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "proposal_status", rename_all = "lowercase")]
pub enum ProposalStatus {
    Pending,
    Active,
    Passed,
    Rejected,
    Executed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "vote_choice", rename_all = "lowercase")]
pub enum VoteChoice {
    For,
    Against,
    Abstain,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GovernanceProposal {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub title: String,
    pub description: String,
    pub governance_model: GovernanceModel,
    pub proposer: Uuid,
    pub status: ProposalStatus,
    pub voting_starts_at: DateTime<Utc>,
    pub voting_ends_at: DateTime<Utc>,
    pub execution_delay_hours: Option<i32>,
    pub quorum_required: i32,
    pub approval_threshold: i32,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposalRequest {
    pub title: String,
    pub description: String,
    pub governance_model: GovernanceModel,
    pub voting_duration_hours: i32,
    pub execution_delay_hours: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GovernanceVote {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub voter: Uuid,
    pub vote_choice: VoteChoice,
    pub voting_power: i64,
    pub delegated_from: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastVoteRequest {
    pub vote_choice: VoteChoice,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VoteDelegation {
    pub id: Uuid,
    pub delegator: Uuid,
    pub delegate: Uuid,
    pub contract_id: Option<Uuid>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalResults {
    pub proposal: GovernanceProposal,
    pub votes_for: i64,
    pub votes_against: i64,
    pub votes_abstain: i64,
    pub total_votes: i64,
    pub quorum_met: bool,
    pub approved: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// TRUST SCORING
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrustScore {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub score: i32,
    pub verified_points: i32,
    pub audit_points: i32,
    pub usage_points: i32,
    pub age_points: i32,
    pub vulnerability_penalty: i32,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreBreakdown {
    pub score: i32,
    pub tier: String,
    pub factors: Vec<TrustFactor>,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFactor {
    pub name: String,
    pub points: i32,
    pub max_points: i32,
    pub weight: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreWeights {
    pub verified: f64,
    pub audit: f64,
    pub usage: f64,
    pub age: f64,
    pub vulnerability: f64,
}

impl Default for TrustScoreWeights {
    fn default() -> Self {
        Self {
            verified: 0.30,
            audit: 0.25,
            usage: 0.20,
            age: 0.15,
            vulnerability: 0.10,
        }
    }
}

impl std::fmt::Display for DeploymentEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
         match self {
             DeploymentEnvironment::Blue => write!(f, "blue"),
             DeploymentEnvironment::Green => write!(f, "green"),
         }
    }
}
