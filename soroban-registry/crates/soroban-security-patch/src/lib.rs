//! # soroban-security-patch
//!
//! A framework for creating, validating, distributing, and applying security
//! patches to Soroban smart contracts.
//!
//! ## Features
//!
//! - **Patch management** – create, validate, and track security patches with
//!   SHA-256 integrity verification.
//! - **Severity levels** – Critical, High, Medium, and Low classifications
//!   that drive version bumps and notification priority.
//! - **Distribution** – notify vulnerable contracts with delivery tracking
//!   and acknowledgement.
//! - **Versioning** – semantic version management with severity-based bumping
//!   and release history.
//! - **Staged rollout** – canary → early-adopter → GA deployment pipeline
//!   with failure-rate gating and manual approval gates.
//! - **Audit trail** – append-only log of every action in the patch lifecycle.

pub mod audit;
pub mod distribution;
pub mod patch;
pub mod rollout;
pub mod types;
pub mod versioning;

// Re-export the primary public API.
pub use audit::AuditTrail;
pub use distribution::DistributionManager;
pub use patch::{PatchManager, SecurityPatch};
pub use rollout::RolloutEngine;
pub use types::{
    AuditAction, AuditEntry, NotificationRecord, NotificationStatus, PatchStatus, PatchVersion,
    RolloutPlan, RolloutStage, SecurityPatchError, Severity,
};
pub use versioning::VersionManager;
