use serde::{Deserialize, Serialize};

use crate::types::{PatchVersion, SecurityPatchError, Severity};

// ---------------------------------------------------------------------------
// Version registry
// ---------------------------------------------------------------------------

/// A record of a released patch version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    /// The patch ID this version belongs to.
    pub patch_id: String,
    /// The version number.
    pub version: PatchVersion,
    /// Whether this is a major (breaking) release.
    pub is_major: bool,
    /// Severity level at time of release.
    pub severity: Severity,
    /// Release timestamp (RFC 3339).
    pub released_at: String,
    /// Optional release notes.
    pub release_notes: Option<String>,
}

/// Manages versioning for security patches.
#[derive(Debug, Default)]
pub struct VersionManager {
    records: Vec<VersionRecord>,
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Register a new version release for a patch.
    pub fn release_version(
        &mut self,
        patch_id: &str,
        version: PatchVersion,
        severity: Severity,
        release_notes: Option<String>,
    ) -> &VersionRecord {
        let is_major = version.major > 0
            && self
                .latest_version(patch_id)
                .is_none_or(|prev| version.major > prev.major);

        let record = VersionRecord {
            patch_id: patch_id.to_string(),
            version,
            is_major,
            severity,
            released_at: chrono::Utc::now().to_rfc3339(),
            release_notes,
        };

        self.records.push(record);
        self.records.last().expect("just pushed")
    }

    /// Bump the version of a patch based on severity:
    /// - Critical / High → major bump
    /// - Medium → minor bump
    /// - Low → patch bump
    pub fn bump_for_severity(
        &mut self,
        patch_id: &str,
        severity: Severity,
        release_notes: Option<String>,
    ) -> &VersionRecord {
        let current = self.latest_version(patch_id).cloned().unwrap_or_default();

        let next = match severity {
            Severity::Critical | Severity::High => current.bump_major(),
            Severity::Medium => current.bump_minor(),
            Severity::Low => current.bump_patch(),
        };

        self.release_version(patch_id, next, severity, release_notes)
    }

    /// Get the latest version for a given patch.
    pub fn latest_version(&self, patch_id: &str) -> Option<&PatchVersion> {
        self.records
            .iter()
            .rev()
            .find(|r| r.patch_id == patch_id)
            .map(|r| &r.version)
    }

    /// Get the full release history for a patch, in chronological order.
    pub fn release_history(&self, patch_id: &str) -> Vec<&VersionRecord> {
        self.records
            .iter()
            .filter(|r| r.patch_id == patch_id)
            .collect()
    }

    /// Verify that a proposed version is strictly newer than the current one.
    pub fn verify_version_order(
        &self,
        patch_id: &str,
        proposed: &PatchVersion,
    ) -> Result<(), SecurityPatchError> {
        if let Some(current) = self.latest_version(patch_id) {
            let is_newer = proposed.major > current.major
                || (proposed.major == current.major && proposed.minor > current.minor)
                || (proposed.major == current.major
                    && proposed.minor == current.minor
                    && proposed.patch > current.patch);

            if !is_newer {
                return Err(SecurityPatchError::VersionConflict {
                    current: current.to_string(),
                    proposed: proposed.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Total number of version records.
    pub fn count(&self) -> usize {
        self.records.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_release_version() {
        let mut vm = VersionManager::new();
        let record = vm.release_version(
            "patch-1",
            PatchVersion::new(1, 0, 0),
            Severity::Critical,
            Some("Initial release".into()),
        );

        assert_eq!(record.patch_id, "patch-1");
        assert!(record.is_major);
        assert_eq!(record.version, PatchVersion::new(1, 0, 0));
    }

    #[test]
    fn test_bump_for_severity_critical() {
        let mut vm = VersionManager::new();

        // Seed with v0.1.0
        vm.release_version("p1", PatchVersion::new(0, 1, 0), Severity::Low, None);

        // Critical bump → major
        let record = vm.bump_for_severity("p1", Severity::Critical, None);
        assert_eq!(record.version, PatchVersion::new(1, 0, 0));
        assert!(record.is_major);
    }

    #[test]
    fn test_bump_for_severity_medium() {
        let mut vm = VersionManager::new();
        vm.release_version("p1", PatchVersion::new(1, 0, 0), Severity::High, None);

        let record = vm.bump_for_severity("p1", Severity::Medium, None);
        assert_eq!(record.version, PatchVersion::new(1, 1, 0));
        assert!(!record.is_major);
    }

    #[test]
    fn test_bump_for_severity_low() {
        let mut vm = VersionManager::new();
        vm.release_version("p1", PatchVersion::new(1, 2, 0), Severity::Medium, None);

        let record = vm.bump_for_severity("p1", Severity::Low, None);
        assert_eq!(record.version, PatchVersion::new(1, 2, 1));
    }

    #[test]
    fn test_latest_version() {
        let mut vm = VersionManager::new();
        vm.release_version("p1", PatchVersion::new(1, 0, 0), Severity::High, None);
        vm.release_version("p1", PatchVersion::new(2, 0, 0), Severity::Critical, None);

        let latest = vm.latest_version("p1").unwrap();
        assert_eq!(*latest, PatchVersion::new(2, 0, 0));
    }

    #[test]
    fn test_release_history() {
        let mut vm = VersionManager::new();
        vm.release_version("p1", PatchVersion::new(1, 0, 0), Severity::High, None);
        vm.release_version("p1", PatchVersion::new(1, 1, 0), Severity::Medium, None);
        vm.release_version("p2", PatchVersion::new(1, 0, 0), Severity::Low, None);

        let history = vm.release_history("p1");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_verify_version_order_valid() {
        let mut vm = VersionManager::new();
        vm.release_version("p1", PatchVersion::new(1, 0, 0), Severity::High, None);

        let result = vm.verify_version_order("p1", &PatchVersion::new(1, 0, 1));
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_version_order_conflict() {
        let mut vm = VersionManager::new();
        vm.release_version("p1", PatchVersion::new(2, 0, 0), Severity::Critical, None);

        let result = vm.verify_version_order("p1", &PatchVersion::new(1, 0, 0));
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_version_order_no_existing() {
        let vm = VersionManager::new();
        // No existing version → any proposed version is valid
        let result = vm.verify_version_order("p-new", &PatchVersion::new(0, 1, 0));
        assert!(result.is_ok());
    }
}
