pub mod frameworks;
pub mod audit;
pub mod reports;
pub mod remediation;
pub mod certification;

pub use frameworks::*;
pub use audit::*;
pub use reports::*;
pub use remediation::*;
pub use certification::*;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents a compliance check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResult {
    pub check_id: String,
    pub framework: ComplianceFramework,
    pub passed: bool,
    pub severity: Severity,
    pub message: String,
    pub details: Option<String>,
}

/// Compliance frameworks supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComplianceFramework {
    #[serde(rename = "gdpr")]
    GDPR,
    #[serde(rename = "soc2")]
    SOC2,
    #[serde(rename = "hipaa")]
    HIPAA,
    #[serde(rename = "iso27001")]
    ISO27001,
    #[serde(rename = "pci_dss")]
    PCIDSS,
    #[serde(rename = "custom")]
    Custom(String),
}

impl std::fmt::Display for ComplianceFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GDPR => write!(f, "GDPR"),
            Self::SOC2 => write!(f, "SOC2"),
            Self::HIPAA => write!(f, "HIPAA"),
            Self::ISO27001 => write!(f, "ISO 27001"),
            Self::PCIDSS => write!(f, "PCI DSS"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Severity levels for compliance issues
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "info")]
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "Critical"),
            Self::High => write!(f, "High"),
            Self::Medium => write!(f, "Medium"),
            Self::Low => write!(f, "Low"),
            Self::Info => write!(f, "Info"),
        }
    }
}

/// Compliance status for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub contract_id: Uuid,
    pub framework: ComplianceFramework,
    pub overall_compliant: bool,
    pub last_checked: DateTime<Utc>,
    pub gaps_count: usize,
    pub satisfied_requirements: usize,
    pub total_requirements: usize,
    pub compliance_percentage: f64,
}

/// Requirement for compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub id: String,
    pub framework: ComplianceFramework,
    pub title: String,
    pub description: String,
    pub category: String,
    pub mandatory: bool,
    pub check_fn: String, // Name of the check function
}

/// Compliance gap identified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceGap {
    pub id: String,
    pub requirement: ComplianceRequirement,
    pub severity: Severity,
    pub impact: String,
    pub current_state: String,
    pub desired_state: String,
}

/// Remediation advice for a gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationAdvice {
    pub gap_id: String,
    pub steps: Vec<RemediationStep>,
    pub estimated_effort: EffortLevel,
    pub priority: Priority,
    pub resources_needed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationStep {
    pub step_number: usize,
    pub action: String,
    pub description: String,
    pub tools: Vec<String>,
    pub success_criteria: String,
}

/// Effort level for remediation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EffortLevel {
    #[serde(rename = "trivial")]
    Trivial,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "very_high")]
    VeryHigh,
}

/// Priority for remediation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}
