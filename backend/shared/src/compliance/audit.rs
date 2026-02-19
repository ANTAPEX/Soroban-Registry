use crate::compliance::{
    ComplianceCheckResult, ComplianceFramework, ComplianceGap, ComplianceRequirement,
    ComplianceStatus, Severity,
};
use crate::Contract;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Audit engine for compliance checks
pub struct ComplianceAuditEngine;

impl ComplianceAuditEngine {
    /// Run a complete compliance audit on a contract for a specific framework
    pub fn audit(
        contract: &Contract,
        framework: &ComplianceFramework,
        requirements: Vec<ComplianceRequirement>,
    ) -> ComplianceStatus {
        let mut check_results = Vec::new();
        let mut gap_count = 0;

        for requirement in requirements.iter() {
            let passed = Self::execute_check(contract, requirement);
            if !passed {
                gap_count += 1;
            }

            check_results.push(ComplianceCheckResult {
                check_id: requirement.id.clone(),
                framework: framework.clone(),
                passed,
                severity: Self::determine_severity(requirement),
                message: if passed {
                    format!("✓ {} passed", requirement.title)
                } else {
                    format!("✗ {} failed", requirement.title)
                },
                details: Some(requirement.description.clone()),
            });
        }

        let total_requirements = requirements.len();
        let satisfied = total_requirements - gap_count;
        let compliance_percentage =
            if total_requirements > 0 {
                (satisfied as f64 / total_requirements as f64) * 100.0
            } else {
                0.0
            };

        ComplianceStatus {
            contract_id: contract.id,
            framework: framework.clone(),
            overall_compliant: gap_count == 0,
            last_checked: Utc::now(),
            gaps_count: gap_count,
            satisfied_requirements: satisfied,
            total_requirements,
            compliance_percentage,
        }
    }

    /// Run checks across multiple frameworks
    pub fn multi_framework_audit(
        contract: &Contract,
        frameworks: Vec<ComplianceFramework>,
    ) -> Vec<ComplianceStatus> {
        frameworks
            .into_iter()
            .map(|framework| {
                let requirements = crate::compliance::frameworks::get_framework_requirements(&framework);
                Self::audit(contract, &framework, requirements)
            })
            .collect()
    }

    /// Identify compliance gaps in detail
    pub fn identify_gaps(
        contract: &Contract,
        requirements: Vec<ComplianceRequirement>,
    ) -> Vec<ComplianceGap> {
        requirements
            .iter()
            .filter_map(|req| {
                if !Self::execute_check(contract, req) {
                    Some(ComplianceGap {
                        id: format!("gap_{}", req.id),
                        requirement: req.clone(),
                        severity: Self::determine_severity(req),
                        impact: Self::describe_impact(req),
                        current_state: Self::assess_current_state(contract, req),
                        desired_state: req.description.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn execute_check(contract: &Contract, requirement: &ComplianceRequirement) -> bool {
        // Simulated check logic - in production, this would call actual validation functions
        match requirement.check_fn.as_str() {
            // GDPR checks
            "check_dpa" => contract.description.as_ref()
                .map(|d| d.contains("DPA") || d.contains("Data Processing"))
                .unwrap_or(false),
            "check_dpia" => contract.tags.iter().any(|t| t.contains("DPIA")),
            "check_consent" => contract.tags.iter().any(|t| t.contains("consent"))
                || contract.description.as_ref()
                    .map(|d| d.contains("consent"))
                    .unwrap_or(false),
            "check_subject_rights" => contract.tags.iter().any(|t| t.contains("GDPR"))
                || contract.description.as_ref()
                    .map(|d| d.contains("user rights") || d.contains("data rights"))
                    .unwrap_or(false),
            "check_breach_notification" => contract.tags.iter().any(|t| t.contains("breach"))
                || contract.description.as_ref()
                    .map(|d| d.contains("breach"))
                    .unwrap_or(false),
            "check_privacy_by_design" => contract.description.as_ref()
                .map(|d| d.contains("privacy") || d.contains("secure"))
                .unwrap_or(false),

            // SOC2 checks
            "check_rbac" => contract.tags.iter().any(|t| t.contains("rbac"))
                || contract.description.as_ref()
                    .map(|d| d.contains("access control"))
                    .unwrap_or(false),
            "check_change_management" => contract.tags.iter().any(|t| t.contains("versioning"))
                || contract.description.as_ref()
                    .map(|d| d.contains("version"))
                    .unwrap_or(false),
            "check_encryption" => contract.tags.iter().any(|t| t.contains("encryption"))
                || contract.description.as_ref()
                    .map(|d| d.contains("encrypt") || d.contains("secure"))
                    .unwrap_or(false),
            "check_incident_response" => contract.tags.iter().any(|t| t.contains("incident")),
            "check_audit_logging" => contract.tags.iter().any(|t| t.contains("audit"))
                || contract.description.as_ref()
                    .map(|d| d.contains("log") || d.contains("audit"))
                    .unwrap_or(false),
            "check_sla" => contract.tags.iter().any(|t| t.contains("sla"))
                || contract.description.as_ref()
                    .map(|d| d.contains("availability") || d.contains("uptime"))
                    .unwrap_or(false),

            // HIPAA checks
            "check_phi_encryption" => contract.tags.iter().any(|t| t.contains("encryption"))
                && contract.description.as_ref()
                    .map(|d| d.contains("health") || d.contains("phi"))
                    .unwrap_or(false),
            "check_phi_access_control" => contract.tags.iter().any(|t| t.contains("rbac")),
            "check_baa" => contract.tags.iter().any(|t| t.contains("baa")),
            "check_data_integrity" => contract.tags.iter().any(|t| t.contains("integrity"))
                || contract.description.as_ref()
                    .map(|d| d.contains("integrity"))
                    .unwrap_or(false),

            // ISO 27001 checks
            "check_isms_policy" => contract.description.is_some(),
            "check_risk_assessment" => contract.tags.iter().any(|t| t.contains("risk")),
            "check_least_privilege" => contract.tags.iter().any(|t| t.contains("privilege"))
                || contract.description.as_ref()
                    .map(|d| d.contains("least privilege"))
                    .unwrap_or(false),
            "check_cryptography" => contract.tags.iter().any(|t| t.contains("crypto")),

            // PCI DSS checks
            "check_network_security" => contract.tags.iter().any(|t| t.contains("firewall")),
            "check_default_creds" => contract.tags.iter().any(|t| t.contains("hardened")),
            "check_cardholder_data" => contract.tags.iter().any(|t| t.contains("encryption")),
            "check_vulnerability_management" => contract.tags.iter().any(|t| t.contains("scan")),

            _ => false,
        }
    }

    fn determine_severity(requirement: &ComplianceRequirement) -> Severity {
        if requirement.mandatory {
            Severity::Critical
        } else {
            Severity::Medium
        }
    }

    fn describe_impact(requirement: &ComplianceRequirement) -> String {
        match requirement.framework {
            ComplianceFramework::GDPR => {
                "Failure to meet GDPR requirements can result in fines up to €20M or 4% of global turnover."
                    .to_string()
            }
            ComplianceFramework::SOC2 => {
                "Non-compliance may result in loss of customer trust and audit failures.".to_string()
            }
            ComplianceFramework::HIPAA => {
                "Violations can result in penalties up to $1.5M per year and criminal liability."
                    .to_string()
            }
            ComplianceFramework::ISO27001 => {
                "Non-certification affects business credibility and marketability.".to_string()
            }
            ComplianceFramework::PCIDSS => {
                "Non-compliance can result in payment processing restrictions and fines."
                    .to_string()
            }
            ComplianceFramework::Custom(_) => "Custom compliance requirement not met.".to_string(),
        }
    }

    fn assess_current_state(contract: &Contract, requirement: &ComplianceRequirement) -> String {
        if contract.tags.is_empty() && contract.description.is_none() {
            "No documentation or tags indicating compliance measures".to_string()
        } else {
            let mut indicators = Vec::new();
            if let Some(desc) = &contract.description {
                if desc.len() > 30 {
                    indicators.push("Has description");
                }
            }
            if !contract.tags.is_empty() {
                indicators.push("Has tags");
            }
            if indicators.is_empty() {
                "Minimal compliance indication".to_string()
            } else {
                format!("Partial implementation: {}", indicators.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_audit_engine() {
        let contract = Contract {
            id: Uuid::new_v4(),
            contract_id: "CBDLTOXIKNRGRPTDYXCYDMNFYVSF5YGMSEXQQE5VCN4WXZZH7VFPBRT".to_string(),
            wasm_hash: "abc123".to_string(),
            name: "Test Contract".to_string(),
            description: Some("Privacy-focused contract with DPA".to_string()),
            publisher_id: Uuid::new_v4(),
            network: crate::Network::Testnet,
            is_verified: true,
            category: Some("Security".to_string()),
            tags: vec!["gdpr".to_string(), "privacy".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let requirements = crate::compliance::frameworks::gdpr_requirements();
        let status = ComplianceAuditEngine::audit(&contract, &ComplianceFramework::GDPR, requirements);

        assert!(!status.overall_compliant); // Not all requirements met
        assert!(status.gaps_count > 0);
    }
}
