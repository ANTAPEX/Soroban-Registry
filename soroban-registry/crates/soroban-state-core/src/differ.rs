/// State differ for comparing contract state snapshots
use crate::types::*;
use std::collections::HashMap;

/// Diff engine for comparing contract states
pub struct StateDiffer;

impl StateDiffer {
    /// Compute diff between two state snapshots
    pub fn diff(before: &ContractState, after: &ContractState) -> StateDiff {
        let mut before_map: HashMap<String, &StateEntry> = HashMap::new();
        let mut after_map: HashMap<String, &StateEntry> = HashMap::new();

        for entry in &before.entries {
            before_map.insert(format!("{:?}", entry.key), entry);
        }

        for entry in &after.entries {
            after_map.insert(format!("{:?}", entry.key), entry);
        }

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();
        let mut unchanged = 0;

        // Find added and modified
        for (key_str, after_entry) in &after_map {
            if let Some(before_entry) = before_map.get(key_str) {
                if format!("{:?}", before_entry.value) == format!("{:?}", after_entry.value) {
                    unchanged += 1;
                } else {
                    modified.push(ModifiedEntry {
                        key: after_entry.key.clone(),
                        before: before_entry.value.clone(),
                        after: after_entry.value.clone(),
                    });
                }
            } else {
                added.push((*after_entry).clone());
            }
        }

        // Find removed
        for (key_str, before_entry) in &before_map {
            if !after_map.contains_key(key_str) {
                removed.push((*before_entry).clone());
            }
        }

        StateDiff {
            contract_id: before.contract_id.clone(),
            from_ledger: before.ledger,
            to_ledger: after.ledger,
            added,
            removed,
            modified,
            unchanged,
        }
    }

    /// Format diff in human-readable format
    pub fn format_human(diff: &StateDiff) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "Contract State Diff: {}\n",
            &diff.contract_id[..std::cmp::min(12, diff.contract_id.len())]
        ));
        output.push_str(&format!(
            "Ledger {} → {}\n",
            diff.from_ledger, diff.to_ledger
        ));
        output.push_str("────────────────────────────────────────────────────────────\n\n");

        // Added entries
        if !diff.added.is_empty() {
            output.push_str("✓ ADDED\n");
            for entry in &diff.added {
                output.push_str(&format!("  {:?} = {}\n", entry.key, entry.value));
            }
            output.push('\n');
        }

        // Removed entries
        if !diff.removed.is_empty() {
            output.push_str("✗ REMOVED\n");
            for entry in &diff.removed {
                output.push_str(&format!("  {:?} = {}\n", entry.key, entry.value));
            }
            output.push('\n');
        }

        // Modified entries
        if !diff.modified.is_empty() {
            output.push_str("~ CHANGED\n");
            for entry in &diff.modified {
                output.push_str(&format!("  {:?}\n", entry.key));
                output.push_str(&format!("    before: {}\n", entry.before));
                output.push_str(&format!("    after:  {}\n", entry.after));
            }
            output.push('\n');
        }

        let summary = format!(
            "{} changed, {} added, {} removed, {} unchanged",
            diff.modified.len(),
            diff.added.len(),
            diff.removed.len(),
            diff.unchanged
        );
        output.push_str(&summary);

        output
    }

    /// Format diff as JSON
    pub fn format_json(diff: &StateDiff) -> serde_json::Value {
        serde_json::to_value(diff).unwrap_or(serde_json::json!({}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_creation() {
        let state1 = ContractState {
            contract_id: "test".to_string(),
            ledger: 100,
            timestamp: "2024-01-01".to_string(),
            entries: vec![],
        };
        let state2 = ContractState {
            contract_id: "test".to_string(),
            ledger: 200,
            timestamp: "2024-01-02".to_string(),
            entries: vec![],
        };
        let diff = StateDiffer::diff(&state1, &state2);
        assert_eq!(diff.from_ledger, 100);
        assert_eq!(diff.to_ledger, 200);
    }

    #[test]
    fn test_format_human() {
        let diff = StateDiff {
            contract_id: "CABC".to_string(),
            from_ledger: 100,
            to_ledger: 200,
            added: vec![],
            removed: vec![],
            modified: vec![],
            unchanged: 5,
        };
        let formatted = StateDiffer::format_human(&diff);
        assert!(formatted.contains("Ledger 100 → 200"));
    }
}
