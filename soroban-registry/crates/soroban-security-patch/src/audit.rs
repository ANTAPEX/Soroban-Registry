use chrono::Utc;

use crate::types::{AuditAction, AuditEntry, SecurityPatchError};

// ---------------------------------------------------------------------------
// Audit trail
// ---------------------------------------------------------------------------

/// Maintains an append-only audit trail for the security patch lifecycle.
#[derive(Debug, Default)]
pub struct AuditTrail {
    entries: Vec<AuditEntry>,
}

impl AuditTrail {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record a new audit entry.
    pub fn record(
        &mut self,
        patch_id: &str,
        contract_id: Option<String>,
        action: AuditAction,
        performed_by: &str,
        details: Option<String>,
    ) -> &AuditEntry {
        let entry = AuditEntry {
            entry_id: uuid::Uuid::new_v4().to_string(),
            patch_id: patch_id.to_string(),
            contract_id,
            action,
            performed_by: performed_by.to_string(),
            timestamp: Utc::now(),
            details,
        };

        self.entries.push(entry);
        self.entries.last().expect("just pushed")
    }

    /// Retrieve all entries for a given patch.
    pub fn entries_for_patch(&self, patch_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.patch_id == patch_id)
            .collect()
    }

    /// Retrieve all entries for a given contract.
    pub fn entries_for_contract(&self, contract_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.contract_id.as_deref() == Some(contract_id))
            .collect()
    }

    /// Retrieve entries filtered by action type.
    pub fn entries_by_action(&self, action: &AuditAction) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| &e.action == action)
            .collect()
    }

    /// Check whether a specific patch has been applied to a specific contract.
    pub fn is_patch_applied(&self, patch_id: &str, contract_id: &str) -> bool {
        self.entries.iter().any(|e| {
            e.patch_id == patch_id
                && e.contract_id.as_deref() == Some(contract_id)
                && e.action == AuditAction::PatchApplied
        })
    }

    /// Get a time-ordered history of a patch's lifecycle.
    pub fn patch_timeline(&self, patch_id: &str) -> Vec<&AuditEntry> {
        let mut timeline: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.patch_id == patch_id)
            .collect();
        timeline.sort_by_key(|e| e.timestamp);
        timeline
    }

    /// Count how many contracts have had a specific patch applied.
    pub fn application_count(&self, patch_id: &str) -> usize {
        self.entries
            .iter()
            .filter(|e| e.patch_id == patch_id && e.action == AuditAction::PatchApplied)
            .count()
    }

    /// Total number of audit entries.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Export the full audit trail as a JSON string.
    pub fn export_json(&self) -> Result<String, SecurityPatchError> {
        serde_json::to_string_pretty(&self.entries)
            .map_err(|e| SecurityPatchError::SerializationError(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_retrieve() {
        let mut trail = AuditTrail::new();
        trail.record(
            "patch-1",
            None,
            AuditAction::PatchCreated,
            "admin",
            Some("First patch".into()),
        );

        assert_eq!(trail.count(), 1);
        let entries = trail.entries_for_patch("patch-1");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, AuditAction::PatchCreated);
    }

    #[test]
    fn test_is_patch_applied() {
        let mut trail = AuditTrail::new();

        trail.record(
            "patch-1",
            Some("C1".into()),
            AuditAction::PatchApplied,
            "operator",
            None,
        );

        assert!(trail.is_patch_applied("patch-1", "C1"));
        assert!(!trail.is_patch_applied("patch-1", "C2"));
        assert!(!trail.is_patch_applied("patch-2", "C1"));
    }

    #[test]
    fn test_application_count() {
        let mut trail = AuditTrail::new();

        trail.record(
            "patch-1",
            Some("C1".into()),
            AuditAction::PatchApplied,
            "op",
            None,
        );
        trail.record(
            "patch-1",
            Some("C2".into()),
            AuditAction::PatchApplied,
            "op",
            None,
        );
        trail.record(
            "patch-1",
            Some("C3".into()),
            AuditAction::NotificationSent,
            "op",
            None,
        );

        assert_eq!(trail.application_count("patch-1"), 2);
    }

    #[test]
    fn test_entries_for_contract() {
        let mut trail = AuditTrail::new();
        trail.record(
            "p1",
            Some("C1".into()),
            AuditAction::PatchApplied,
            "op",
            None,
        );
        trail.record(
            "p2",
            Some("C1".into()),
            AuditAction::NotificationSent,
            "op",
            None,
        );
        trail.record(
            "p1",
            Some("C2".into()),
            AuditAction::PatchApplied,
            "op",
            None,
        );

        let c1 = trail.entries_for_contract("C1");
        assert_eq!(c1.len(), 2);
    }

    #[test]
    fn test_entries_by_action() {
        let mut trail = AuditTrail::new();
        trail.record("p1", None, AuditAction::PatchCreated, "admin", None);
        trail.record("p1", None, AuditAction::PatchValidated, "admin", None);
        trail.record("p2", None, AuditAction::PatchCreated, "admin", None);

        let created = trail.entries_by_action(&AuditAction::PatchCreated);
        assert_eq!(created.len(), 2);
    }

    #[test]
    fn test_patch_timeline() {
        let mut trail = AuditTrail::new();
        trail.record("p1", None, AuditAction::PatchCreated, "admin", None);
        trail.record("p1", None, AuditAction::PatchValidated, "admin", None);
        trail.record(
            "p1",
            Some("C1".into()),
            AuditAction::PatchApplied,
            "op",
            None,
        );

        let timeline = trail.patch_timeline("p1");
        assert_eq!(timeline.len(), 3);
        // Ensure chronological order
        for i in 1..timeline.len() {
            assert!(timeline[i].timestamp >= timeline[i - 1].timestamp);
        }
    }

    #[test]
    fn test_export_json() {
        let mut trail = AuditTrail::new();
        trail.record("p1", None, AuditAction::PatchCreated, "admin", None);

        let json = trail.export_json().unwrap();
        assert!(json.contains("patch_created"));
        assert!(json.contains("p1"));
    }
}
