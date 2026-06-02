use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::types::{PatchStatus, PatchVersion, SecurityPatchError, Severity};

// ---------------------------------------------------------------------------
// Security patch
// ---------------------------------------------------------------------------

/// A security patch targeting one or more Soroban contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPatch {
    /// Unique identifier for the patch (UUID v4).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Detailed description of the vulnerability and the fix.
    pub description: String,
    /// Severity level.
    pub severity: Severity,
    /// Current lifecycle status.
    pub status: PatchStatus,
    /// Semantic version of this patch release.
    pub version: PatchVersion,
    /// SHA-256 hash of the patch payload for integrity verification.
    pub payload_hash: String,
    /// The raw patch payload (e.g. WASM diff, bytecode, or migration script).
    pub payload: Vec<u8>,
    /// List of contract IDs known to be affected.
    pub affected_contracts: Vec<String>,
    /// CVE or internal advisory identifier (optional).
    pub advisory_id: Option<String>,
    /// Who created the patch.
    pub created_by: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Last-modified timestamp.
    pub updated_at: String,
    /// Validation results (populated after validation passes).
    pub validation_results: Vec<ValidationResult>,
}

/// Result of a single validation check executed against a patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub check_name: String,
    pub passed: bool,
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// Patch manager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of security patches.
#[derive(Debug, Default)]
pub struct PatchManager {
    patches: Vec<SecurityPatch>,
}

impl PatchManager {
    pub fn new() -> Self {
        Self {
            patches: Vec::new(),
        }
    }

    // ----- Creation --------------------------------------------------------

    /// Create a new security patch and register it in the manager.
    ///
    /// The patch is created in `Draft` status. The `payload_hash` is computed
    /// automatically from the supplied `payload`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_patch(
        &mut self,
        title: String,
        description: String,
        severity: Severity,
        payload: Vec<u8>,
        affected_contracts: Vec<String>,
        advisory_id: Option<String>,
        created_by: String,
    ) -> Result<&SecurityPatch, SecurityPatchError> {
        let id = uuid::Uuid::new_v4().to_string();

        // Check for duplicate IDs (extremely unlikely with UUIDs, but defensive).
        if self.patches.iter().any(|p| p.id == id) {
            return Err(SecurityPatchError::DuplicatePatchId(id));
        }

        let payload_hash = compute_hash(&payload);
        let now = Utc::now().to_rfc3339();

        let patch = SecurityPatch {
            id,
            title,
            description,
            severity,
            status: PatchStatus::Draft,
            version: PatchVersion::default(),
            payload_hash,
            payload,
            affected_contracts,
            advisory_id,
            created_by,
            created_at: now.clone(),
            updated_at: now,
            validation_results: Vec::new(),
        };

        self.patches.push(patch);
        Ok(self.patches.last().expect("just pushed"))
    }

    // ----- Retrieval -------------------------------------------------------

    /// Retrieve a patch by its ID.
    pub fn get_patch(&self, patch_id: &str) -> Result<&SecurityPatch, SecurityPatchError> {
        self.patches
            .iter()
            .find(|p| p.id == patch_id)
            .ok_or_else(|| SecurityPatchError::PatchNotFound(patch_id.to_string()))
    }

    /// Retrieve a mutable reference to a patch by its ID.
    pub fn get_patch_mut(
        &mut self,
        patch_id: &str,
    ) -> Result<&mut SecurityPatch, SecurityPatchError> {
        self.patches
            .iter_mut()
            .find(|p| p.id == patch_id)
            .ok_or_else(|| SecurityPatchError::PatchNotFound(patch_id.to_string()))
    }

    /// List all patches, optionally filtered by status.
    pub fn list_patches(&self, status_filter: Option<PatchStatus>) -> Vec<&SecurityPatch> {
        match status_filter {
            Some(status) => self.patches.iter().filter(|p| p.status == status).collect(),
            None => self.patches.iter().collect(),
        }
    }

    /// List all patches for a specific severity.
    pub fn list_patches_by_severity(&self, severity: Severity) -> Vec<&SecurityPatch> {
        self.patches
            .iter()
            .filter(|p| p.severity == severity)
            .collect()
    }

    // ----- Validation ------------------------------------------------------

    /// Validate a patch. Runs a series of checks and transitions the patch to
    /// `Validated` or `Rejected`.
    pub fn validate_patch(&mut self, patch_id: &str) -> Result<bool, SecurityPatchError> {
        let patch = self.get_patch_mut(patch_id)?;

        Self::assert_transition(&patch.status, &PatchStatus::Validating)?;
        patch.status = PatchStatus::Validating;
        patch.updated_at = Utc::now().to_rfc3339();

        let mut results = Vec::new();
        let mut all_passed = true;

        // Check 1: payload is non-empty.
        let payload_ok = !patch.payload.is_empty();
        results.push(ValidationResult {
            check_name: "payload_non_empty".to_string(),
            passed: payload_ok,
            message: if payload_ok {
                None
            } else {
                Some("Patch payload is empty".to_string())
            },
        });
        all_passed &= payload_ok;

        // Check 2: integrity hash matches.
        let hash_ok = compute_hash(&patch.payload) == patch.payload_hash;
        results.push(ValidationResult {
            check_name: "integrity_hash".to_string(),
            passed: hash_ok,
            message: if hash_ok {
                None
            } else {
                Some("Payload hash mismatch".to_string())
            },
        });
        all_passed &= hash_ok;

        // Check 3: at least one affected contract listed.
        let contracts_ok = !patch.affected_contracts.is_empty();
        results.push(ValidationResult {
            check_name: "affected_contracts_listed".to_string(),
            passed: contracts_ok,
            message: if contracts_ok {
                None
            } else {
                Some("No affected contracts specified".to_string())
            },
        });
        all_passed &= contracts_ok;

        // Check 4: title and description are not blank.
        let metadata_ok = !patch.title.trim().is_empty() && !patch.description.trim().is_empty();
        results.push(ValidationResult {
            check_name: "metadata_present".to_string(),
            passed: metadata_ok,
            message: if metadata_ok {
                None
            } else {
                Some("Title or description is missing".to_string())
            },
        });
        all_passed &= metadata_ok;

        patch.validation_results = results;

        if all_passed {
            patch.status = PatchStatus::Validated;
        } else {
            patch.status = PatchStatus::Rejected;
        }
        patch.updated_at = Utc::now().to_rfc3339();

        Ok(all_passed)
    }

    /// Verify the payload integrity of a patch against its stored hash.
    pub fn verify_integrity(&self, patch_id: &str) -> Result<bool, SecurityPatchError> {
        let patch = self.get_patch(patch_id)?;
        let actual = compute_hash(&patch.payload);
        Ok(actual == patch.payload_hash)
    }

    // ----- Status transitions ----------------------------------------------

    /// Transition a patch to a new status (with validation of legal transitions).
    pub fn transition(
        &mut self,
        patch_id: &str,
        new_status: PatchStatus,
    ) -> Result<(), SecurityPatchError> {
        let patch = self.get_patch_mut(patch_id)?;
        Self::assert_transition(&patch.status, &new_status)?;
        patch.status = new_status;
        patch.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    /// Returns the number of registered patches.
    pub fn count(&self) -> usize {
        self.patches.len()
    }

    // ----- Helpers (private) -----------------------------------------------

    /// Assert that a status transition is valid.
    fn assert_transition(from: &PatchStatus, to: &PatchStatus) -> Result<(), SecurityPatchError> {
        let valid = matches!(
            (from, to),
            (PatchStatus::Draft, PatchStatus::Validating)
                | (PatchStatus::Validating, PatchStatus::Validated)
                | (PatchStatus::Validating, PatchStatus::Rejected)
                | (PatchStatus::Validated, PatchStatus::RollingOut)
                | (PatchStatus::RollingOut, PatchStatus::Applied)
                | (PatchStatus::RollingOut, PatchStatus::RolledBack)
        );
        if valid {
            Ok(())
        } else {
            Err(SecurityPatchError::InvalidTransition {
                from: *from,
                to: *to,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Compute the SHA-256 hex digest of `data`.
pub fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manager() -> PatchManager {
        PatchManager::new()
    }

    #[test]
    fn test_create_patch() {
        let mut mgr = sample_manager();
        let patch = mgr
            .create_patch(
                "Fix overflow".into(),
                "Prevents integer overflow in token transfer".into(),
                Severity::Critical,
                b"wasm-bytecode-diff".to_vec(),
                vec!["CONTRACT_A".into(), "CONTRACT_B".into()],
                Some("CVE-2026-0001".into()),
                "admin".into(),
            )
            .unwrap();

        assert_eq!(patch.status, PatchStatus::Draft);
        assert_eq!(patch.severity, Severity::Critical);
        assert_eq!(patch.affected_contracts.len(), 2);
        assert!(!patch.payload_hash.is_empty());
    }

    #[test]
    fn test_validate_patch_success() {
        let mut mgr = sample_manager();
        let patch = mgr
            .create_patch(
                "Fix overflow".into(),
                "Prevents integer overflow".into(),
                Severity::High,
                b"valid-payload".to_vec(),
                vec!["CONTRACT_A".into()],
                None,
                "admin".into(),
            )
            .unwrap();
        let id = patch.id.clone();

        let result = mgr.validate_patch(&id).unwrap();
        assert!(result);
        assert_eq!(mgr.get_patch(&id).unwrap().status, PatchStatus::Validated);
    }

    #[test]
    fn test_validate_patch_failure_empty_payload() {
        let mut mgr = sample_manager();
        let patch = mgr
            .create_patch(
                "Bad patch".into(),
                "Empty payload".into(),
                Severity::Medium,
                Vec::new(), // empty!
                vec!["CONTRACT_X".into()],
                None,
                "admin".into(),
            )
            .unwrap();
        let id = patch.id.clone();

        let result = mgr.validate_patch(&id).unwrap();
        assert!(!result);
        assert_eq!(mgr.get_patch(&id).unwrap().status, PatchStatus::Rejected);
    }

    #[test]
    fn test_verify_integrity() {
        let mut mgr = sample_manager();
        let patch = mgr
            .create_patch(
                "Integrity test".into(),
                "Testing hash verification".into(),
                Severity::Low,
                b"some-data".to_vec(),
                vec!["C1".into()],
                None,
                "admin".into(),
            )
            .unwrap();
        let id = patch.id.clone();

        assert!(mgr.verify_integrity(&id).unwrap());
    }

    #[test]
    fn test_invalid_transition() {
        let mut mgr = sample_manager();
        let patch = mgr
            .create_patch(
                "Trans test".into(),
                "Transition check".into(),
                Severity::Medium,
                b"data".to_vec(),
                vec!["C1".into()],
                None,
                "admin".into(),
            )
            .unwrap();
        let id = patch.id.clone();

        // Draft → Applied should fail
        let err = mgr.transition(&id, PatchStatus::Applied);
        assert!(err.is_err());
    }

    #[test]
    fn test_list_patches_by_severity() {
        let mut mgr = sample_manager();

        mgr.create_patch(
            "P1".into(),
            "D1".into(),
            Severity::Critical,
            b"d1".to_vec(),
            vec!["C1".into()],
            None,
            "admin".into(),
        )
        .unwrap();
        mgr.create_patch(
            "P2".into(),
            "D2".into(),
            Severity::Low,
            b"d2".to_vec(),
            vec!["C2".into()],
            None,
            "admin".into(),
        )
        .unwrap();
        mgr.create_patch(
            "P3".into(),
            "D3".into(),
            Severity::Critical,
            b"d3".to_vec(),
            vec!["C3".into()],
            None,
            "admin".into(),
        )
        .unwrap();

        let critical = mgr.list_patches_by_severity(Severity::Critical);
        assert_eq!(critical.len(), 2);

        let low = mgr.list_patches_by_severity(Severity::Low);
        assert_eq!(low.len(), 1);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let data = b"hello world";
        let h1 = compute_hash(data);
        let h2 = compute_hash(data);
        assert_eq!(h1, h2);
        assert!(!h1.is_empty());
    }
}
