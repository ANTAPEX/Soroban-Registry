use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Severity level of a security patch.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Low-impact issues – informational or best-practice improvements.
    Low,
    /// Medium-impact vulnerabilities – potential for limited exploitation.
    #[default]
    Medium,
    /// High-impact vulnerabilities – significant risk of exploitation.
    High,
    /// Critical vulnerabilities – active or imminent exploitation risk.
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

// ---------------------------------------------------------------------------
// Patch status
// ---------------------------------------------------------------------------

/// Lifecycle status of a security patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PatchStatus {
    /// Patch has been drafted but not yet validated.
    #[default]
    Draft,
    /// Patch is undergoing validation / testing.
    Validating,
    /// Patch has been validated and is ready for distribution.
    Validated,
    /// Patch is currently being rolled out in stages.
    RollingOut,
    /// Patch rollout is complete.
    Applied,
    /// Patch was rejected during validation.
    Rejected,
    /// Patch rollout was rolled back due to issues.
    RolledBack,
}

impl fmt::Display for PatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "DRAFT"),
            Self::Validating => write!(f, "VALIDATING"),
            Self::Validated => write!(f, "VALIDATED"),
            Self::RollingOut => write!(f, "ROLLING_OUT"),
            Self::Applied => write!(f, "APPLIED"),
            Self::Rejected => write!(f, "REJECTED"),
            Self::RolledBack => write!(f, "ROLLED_BACK"),
        }
    }
}

// ---------------------------------------------------------------------------
// Rollout stage
// ---------------------------------------------------------------------------

/// Stage in a staged-rollout pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolloutStage {
    /// Canary deployment — small percentage of contracts.
    Canary,
    /// Early adopter cohort.
    EarlyAdopter,
    /// General availability — all remaining contracts.
    GeneralAvailability,
}

impl fmt::Display for RolloutStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canary => write!(f, "CANARY"),
            Self::EarlyAdopter => write!(f, "EARLY_ADOPTER"),
            Self::GeneralAvailability => write!(f, "GENERAL_AVAILABILITY"),
        }
    }
}

// ---------------------------------------------------------------------------
// Notification status
// ---------------------------------------------------------------------------

/// Status of a vulnerability notification sent to a contract owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    /// Notification has been queued for delivery.
    Pending,
    /// Notification was delivered successfully.
    Delivered,
    /// Notification delivery failed.
    Failed,
    /// Contract owner acknowledged the notification.
    Acknowledged,
}

impl fmt::Display for NotificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Delivered => write!(f, "DELIVERED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Acknowledged => write!(f, "ACKNOWLEDGED"),
        }
    }
}

// ---------------------------------------------------------------------------
// Patch version
// ---------------------------------------------------------------------------

/// Semantic version for a security patch release.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatchVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PatchVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Bump the major version (resets minor + patch to 0).
    pub fn bump_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
        }
    }

    /// Bump the minor version (resets patch to 0).
    pub fn bump_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    /// Bump the patch version.
    pub fn bump_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
        }
    }
}

impl fmt::Display for PatchVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for PatchVersion {
    fn default() -> Self {
        Self::new(0, 1, 0)
    }
}

// ---------------------------------------------------------------------------
// Notification record
// ---------------------------------------------------------------------------

/// Record of a notification sent to a contract about a security patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRecord {
    /// Unique notification identifier.
    pub notification_id: String,
    /// ID of the patch the notification pertains to.
    pub patch_id: String,
    /// Contract ID that was notified.
    pub contract_id: String,
    /// Current status of the notification.
    pub status: NotificationStatus,
    /// Timestamp when the notification was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp of the most recent status update.
    pub updated_at: DateTime<Utc>,
    /// Number of delivery attempts.
    pub attempt_count: u32,
    /// Optional error message from the last failed delivery attempt.
    pub last_error: Option<String>,
}

// ---------------------------------------------------------------------------
// Audit entry
// ---------------------------------------------------------------------------

/// An entry in the patch audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique audit entry identifier.
    pub entry_id: String,
    /// ID of the patch.
    pub patch_id: String,
    /// Contract to which the patch was applied (if applicable).
    pub contract_id: Option<String>,
    /// Action that was performed.
    pub action: AuditAction,
    /// Actor who performed the action (address / operator ID).
    pub performed_by: String,
    /// Timestamp of the action.
    pub timestamp: DateTime<Utc>,
    /// Additional details / human-readable notes.
    pub details: Option<String>,
}

/// Auditable actions within the patch lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    PatchCreated,
    PatchValidated,
    PatchRejected,
    RolloutStarted,
    RolloutStageCompleted,
    PatchApplied,
    PatchRolledBack,
    NotificationSent,
    NotificationAcknowledged,
    VersionBumped,
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PatchCreated => write!(f, "PATCH_CREATED"),
            Self::PatchValidated => write!(f, "PATCH_VALIDATED"),
            Self::PatchRejected => write!(f, "PATCH_REJECTED"),
            Self::RolloutStarted => write!(f, "ROLLOUT_STARTED"),
            Self::RolloutStageCompleted => write!(f, "ROLLOUT_STAGE_COMPLETED"),
            Self::PatchApplied => write!(f, "PATCH_APPLIED"),
            Self::PatchRolledBack => write!(f, "PATCH_ROLLED_BACK"),
            Self::NotificationSent => write!(f, "NOTIFICATION_SENT"),
            Self::NotificationAcknowledged => write!(f, "NOTIFICATION_ACKNOWLEDGED"),
            Self::VersionBumped => write!(f, "VERSION_BUMPED"),
        }
    }
}

// ---------------------------------------------------------------------------
// Rollout plan
// ---------------------------------------------------------------------------

/// Configuration for a staged rollout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutPlan {
    /// Percentage of contracts to include in the canary stage (0–100).
    pub canary_percentage: u8,
    /// Percentage of contracts for early-adopter stage (0–100, cumulative with canary).
    pub early_adopter_percentage: u8,
    /// Minimum soak time (in seconds) before advancing to the next stage.
    pub soak_time_secs: u64,
    /// Maximum acceptable failure rate before auto-rollback (0.0 – 1.0).
    pub max_failure_rate: f64,
    /// Whether to require manual approval before advancing stages.
    pub require_approval: bool,
}

impl Default for RolloutPlan {
    fn default() -> Self {
        Self {
            canary_percentage: 5,
            early_adopter_percentage: 25,
            soak_time_secs: 3600, // 1 hour
            max_failure_rate: 0.01,
            require_approval: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Domain errors for the security patch system.
#[derive(Debug, thiserror::Error)]
pub enum SecurityPatchError {
    #[error("Patch '{0}' not found")]
    PatchNotFound(String),

    #[error("Invalid patch transition from {from} to {to}")]
    InvalidTransition { from: PatchStatus, to: PatchStatus },

    #[error("Patch validation failed: {0}")]
    ValidationFailed(String),

    #[error("Integrity check failed – expected hash {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("Rollout failed at stage {stage}: {reason}")]
    RolloutFailed { stage: RolloutStage, reason: String },

    #[error("No vulnerable contracts found for patch '{0}'")]
    NoVulnerableContracts(String),

    #[error("Duplicate patch ID: '{0}'")]
    DuplicatePatchId(String),

    #[error("Version conflict: current {current}, proposed {proposed}")]
    VersionConflict { current: String, proposed: String },

    #[error("Distribution error: {0}")]
    DistributionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}
