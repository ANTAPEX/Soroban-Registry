use chrono::Utc;

use crate::types::{NotificationRecord, NotificationStatus, SecurityPatchError, Severity};

// ---------------------------------------------------------------------------
// Distribution manager
// ---------------------------------------------------------------------------

/// Manages the distribution of security patch notifications to affected
/// contract owners.
#[derive(Debug, Default)]
pub struct DistributionManager {
    notifications: Vec<NotificationRecord>,
}

impl DistributionManager {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }

    /// Notify all affected contracts about a security patch.
    ///
    /// Creates a [`NotificationRecord`] for each contract in
    /// `affected_contracts` and attempts delivery. In a production system,
    /// this would integrate with a messaging backend; here we simulate the
    /// result.
    pub fn notify_vulnerable_contracts(
        &mut self,
        patch_id: &str,
        affected_contracts: &[String],
        severity: Severity,
    ) -> Result<Vec<String>, SecurityPatchError> {
        if affected_contracts.is_empty() {
            return Err(SecurityPatchError::NoVulnerableContracts(
                patch_id.to_string(),
            ));
        }

        let mut notification_ids = Vec::new();

        for contract_id in affected_contracts {
            let notification_id = uuid::Uuid::new_v4().to_string();
            let now = Utc::now();

            // Simulate delivery – in production this would call an external
            // notification service.  We mark high/critical as immediately
            // delivered and lower severities as pending (batch delivery).
            let status = if severity >= Severity::High {
                NotificationStatus::Delivered
            } else {
                NotificationStatus::Pending
            };

            let record = NotificationRecord {
                notification_id: notification_id.clone(),
                patch_id: patch_id.to_string(),
                contract_id: contract_id.clone(),
                status,
                created_at: now,
                updated_at: now,
                attempt_count: 1,
                last_error: None,
            };

            self.notifications.push(record);
            notification_ids.push(notification_id);
        }

        Ok(notification_ids)
    }

    /// Acknowledge a notification on behalf of the contract owner.
    pub fn acknowledge(&mut self, notification_id: &str) -> Result<(), SecurityPatchError> {
        let record = self
            .notifications
            .iter_mut()
            .find(|n| n.notification_id == notification_id)
            .ok_or_else(|| {
                SecurityPatchError::DistributionError(format!(
                    "Notification '{notification_id}' not found"
                ))
            })?;

        record.status = NotificationStatus::Acknowledged;
        record.updated_at = Utc::now();
        Ok(())
    }

    /// Retry failed notifications for a given patch.
    pub fn retry_failed(&mut self, patch_id: &str) -> Vec<String> {
        let mut retried = Vec::new();

        for record in self.notifications.iter_mut() {
            if record.patch_id == patch_id && record.status == NotificationStatus::Failed {
                record.status = NotificationStatus::Pending;
                record.attempt_count += 1;
                record.updated_at = Utc::now();
                retried.push(record.notification_id.clone());
            }
        }

        retried
    }

    /// List all notifications for a given patch.
    pub fn list_notifications(&self, patch_id: &str) -> Vec<&NotificationRecord> {
        self.notifications
            .iter()
            .filter(|n| n.patch_id == patch_id)
            .collect()
    }

    /// List notifications filtered by status.
    pub fn list_by_status(&self, status: NotificationStatus) -> Vec<&NotificationRecord> {
        self.notifications
            .iter()
            .filter(|n| n.status == status)
            .collect()
    }

    /// Get a summary of notification stats for a patch.
    pub fn notification_summary(&self, patch_id: &str) -> NotificationSummary {
        let relevant: Vec<_> = self
            .notifications
            .iter()
            .filter(|n| n.patch_id == patch_id)
            .collect();

        NotificationSummary {
            total: relevant.len(),
            pending: relevant
                .iter()
                .filter(|n| n.status == NotificationStatus::Pending)
                .count(),
            delivered: relevant
                .iter()
                .filter(|n| n.status == NotificationStatus::Delivered)
                .count(),
            failed: relevant
                .iter()
                .filter(|n| n.status == NotificationStatus::Failed)
                .count(),
            acknowledged: relevant
                .iter()
                .filter(|n| n.status == NotificationStatus::Acknowledged)
                .count(),
        }
    }

    /// Total number of notification records.
    pub fn count(&self) -> usize {
        self.notifications.len()
    }
}

// ---------------------------------------------------------------------------
// Summary helper
// ---------------------------------------------------------------------------

/// Aggregated notification statistics for a single patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationSummary {
    pub total: usize,
    pub pending: usize,
    pub delivered: usize,
    pub failed: usize,
    pub acknowledged: usize,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_vulnerable_contracts() {
        let mut dm = DistributionManager::new();
        let ids = dm
            .notify_vulnerable_contracts(
                "patch-1",
                &["C1".into(), "C2".into(), "C3".into()],
                Severity::Critical,
            )
            .unwrap();

        assert_eq!(ids.len(), 3);
        assert_eq!(dm.count(), 3);

        // Critical → immediately delivered
        let notifs = dm.list_notifications("patch-1");
        assert!(notifs
            .iter()
            .all(|n| n.status == NotificationStatus::Delivered));
    }

    #[test]
    fn test_notify_low_severity_pending() {
        let mut dm = DistributionManager::new();
        dm.notify_vulnerable_contracts("patch-2", &["C1".into()], Severity::Low)
            .unwrap();

        let notifs = dm.list_notifications("patch-2");
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].status, NotificationStatus::Pending);
    }

    #[test]
    fn test_notify_empty_contracts_fails() {
        let mut dm = DistributionManager::new();
        let err = dm.notify_vulnerable_contracts("patch-3", &[], Severity::High);
        assert!(err.is_err());
    }

    #[test]
    fn test_acknowledge_notification() {
        let mut dm = DistributionManager::new();
        let ids = dm
            .notify_vulnerable_contracts("patch-4", &["C1".into()], Severity::High)
            .unwrap();

        dm.acknowledge(&ids[0]).unwrap();
        let notifs = dm.list_notifications("patch-4");
        assert_eq!(notifs[0].status, NotificationStatus::Acknowledged);
    }

    #[test]
    fn test_notification_summary() {
        let mut dm = DistributionManager::new();
        dm.notify_vulnerable_contracts(
            "patch-5",
            &["C1".into(), "C2".into(), "C3".into()],
            Severity::Critical,
        )
        .unwrap();

        let summary = dm.notification_summary("patch-5");
        assert_eq!(summary.total, 3);
        assert_eq!(summary.delivered, 3);
        assert_eq!(summary.pending, 0);
    }

    #[test]
    fn test_list_by_status() {
        let mut dm = DistributionManager::new();
        dm.notify_vulnerable_contracts("p1", &["C1".into()], Severity::Critical)
            .unwrap();
        dm.notify_vulnerable_contracts("p2", &["C2".into()], Severity::Low)
            .unwrap();

        let delivered = dm.list_by_status(NotificationStatus::Delivered);
        assert_eq!(delivered.len(), 1);

        let pending = dm.list_by_status(NotificationStatus::Pending);
        assert_eq!(pending.len(), 1);
    }
}
