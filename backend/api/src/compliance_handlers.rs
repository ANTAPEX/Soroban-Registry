use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use shared::{
    ComplianceFramework, ComplianceAuditEngine, RemediationEngine, ReportGenerator,
    CertificationManager, Contract,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

/// Request to audit compliance
#[derive(Debug, Deserialize)]
pub struct ComplianceAuditRequest {
    pub contract_id: String,
    pub framework: String,
}

/// Response from compliance audit
#[derive(Debug, Serialize)]
pub struct ComplianceAuditResponse {
    pub contract_id: String,
    pub framework: String,
    pub overall_compliant: bool,
    pub compliance_percentage: f64,
    pub satisfied_requirements: usize,
    pub total_requirements: usize,
    pub gaps_count: usize,
}

/// Generate compliance report endpoint
#[derive(Debug, Serialize)]
pub struct ComplianceReportResponse {
    pub report_id: String,
    pub contract_id: String,
    pub framework: String,
    pub compliance_percentage: f64,
    pub risk_level: String,
    pub critical_issues: usize,
    pub high_issues: usize,
    pub medium_issues: usize,
    pub low_issues: usize,
    pub total_requirements: usize,
}

/// Certification eligibility response
#[derive(Debug, Serialize)]
pub struct CertificationEligibilityResponse {
    pub eligible: bool,
    pub message: String,
    pub compliance_percentage: f64,
    pub critical_gaps: usize,
}

/// Framework information
#[derive(Debug, Serialize)]
pub struct FrameworkInfo {
    pub name: String,
    pub full_name: String,
    pub certificate_validity_days: usize,
    pub scope: String,
}

/// Get supported frameworks
pub async fn get_frameworks(
    State(_state): State<Arc<AppState>>,
) -> (StatusCode, Json<Vec<FrameworkInfo>>) {
    let frameworks = vec![
        FrameworkInfo {
            name: "GDPR".to_string(),
            full_name: "General Data Protection Regulation (EU)".to_string(),
            certificate_validity_days: 365,
            scope: "Personal data protection and privacy".to_string(),
        },
        FrameworkInfo {
            name: "SOC2".to_string(),
            full_name: "Service Organization Control 2 (US)".to_string(),
            certificate_validity_days: 365,
            scope: "Security, availability, and confidentiality".to_string(),
        },
        FrameworkInfo {
            name: "HIPAA".to_string(),
            full_name: "Health Insurance Portability and Accountability".to_string(),
            certificate_validity_days: 365,
            scope: "Healthcare data protection".to_string(),
        },
        FrameworkInfo {
            name: "ISO27001".to_string(),
            full_name: "Information Security Management".to_string(),
            certificate_validity_days: 1095,
            scope: "Information security standards".to_string(),
        },
        FrameworkInfo {
            name: "PCIDSS".to_string(),
            full_name: "Payment Card Industry Data Security Standard".to_string(),
            certificate_validity_days: 365,
            scope: "Payment card data protection".to_string(),
        },
    ];

    (StatusCode::OK, Json(frameworks))
}

/// Run compliance audit on a contract
pub async fn audit_contract(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ComplianceAuditRequest>,
) -> Result<(StatusCode, Json<ComplianceAuditResponse>), StatusCode> {
    // Parse framework
    let framework = match payload.framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // In production, fetch from database
    // For now, create a mock contract
    let contract = Contract {
        id: Uuid::parse_str(&payload.contract_id).unwrap_or_else(|_| Uuid::new_v4()),
        contract_id: payload.contract_id.clone(),
        wasm_hash: "abc123".to_string(),
        name: "Test Contract".to_string(),
        description: Some("Privacy-focused contract with security measures".to_string()),
        publisher_id: Uuid::new_v4(),
        network: shared::Network::Testnet,
        is_verified: true,
        category: Some("Finance".to_string()),
        tags: vec!["secure".to_string(), "audit".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let requirements = shared::compliance::frameworks::get_framework_requirements(&framework);
    let status = ComplianceAuditEngine::audit(&contract, &framework, requirements);

    let response = ComplianceAuditResponse {
        contract_id: payload.contract_id,
        framework: payload.framework,
        overall_compliant: status.overall_compliant,
        compliance_percentage: status.compliance_percentage,
        satisfied_requirements: status.satisfied_requirements,
        total_requirements: status.total_requirements,
        gaps_count: status.gaps_count,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Generate a compliance report
pub async fn generate_report(
    State(_state): State<Arc<AppState>>,
    Path((contract_id, framework)): Path<(String, String)>,
) -> Result<(StatusCode, Json<ComplianceReportResponse>), StatusCode> {
    // Parse framework
    let framework_enum = match framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Create a mock contract
    let contract = Contract {
        id: Uuid::parse_str(&contract_id).unwrap_or_else(|_| Uuid::new_v4()),
        contract_id: contract_id.clone(),
        wasm_hash: "abc123".to_string(),
        name: "Test Contract".to_string(),
        description: Some("Privacy-focused contract".to_string()),
        publisher_id: Uuid::new_v4(),
        network: shared::Network::Testnet,
        is_verified: true,
        category: Some("Finance".to_string()),
        tags: vec!["secure".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let requirements = shared::compliance::frameworks::get_framework_requirements(&framework_enum);
    let start_time = std::time::Instant::now();
    let status = ComplianceAuditEngine::audit(&contract, &framework_enum, requirements.clone());
    let gaps = ComplianceAuditEngine::identify_gaps(&contract, requirements);
    let remediation = gaps.iter()
        .map(|g| RemediationEngine::generate_remediation(g))
        .collect();

    let contract_id_uuid = Uuid::parse_str(&contract_id).unwrap_or_else(|_| Uuid::new_v4());
    let _report = ReportGenerator::generate(contract_id_uuid, framework_enum, status.clone(), gaps, remediation, start_time);

    let (critical, high, medium, low) = count_severity_issues(&gaps);

    let response = ComplianceReportResponse {
        report_id: format!("REPORT-{}", Uuid::new_v4()),
        contract_id,
        framework,
        compliance_percentage: status.compliance_percentage,
        risk_level: determine_risk_level(status.compliance_percentage, critical),
        critical_issues: critical,
        high_issues: high,
        medium_issues: medium,
        low_issues: low,
        total_requirements: status.total_requirements,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get compliance gaps
pub async fn identify_gaps(
    State(_state): State<Arc<AppState>>,
    Path((contract_id, framework)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    // Parse framework
    let framework_enum = match framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Create a mock contract
    let contract = Contract {
        id: Uuid::parse_str(&contract_id).unwrap_or_else(|_| Uuid::new_v4()),
        contract_id: contract_id.clone(),
        wasm_hash: "abc123".to_string(),
        name: "Test Contract".to_string(),
        description: None,
        publisher_id: Uuid::new_v4(),
        network: shared::Network::Testnet,
        is_verified: false,
        category: None,
        tags: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let requirements = shared::compliance::frameworks::get_framework_requirements(&framework_enum);
    let gaps = ComplianceAuditEngine::identify_gaps(&contract, requirements);

    let gaps_json = serde_json::to_value(&gaps).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, Json(serde_json::json!({
        "contract_id": contract_id,
        "framework": framework,
        "gaps_count": gaps.len(),
        "gaps": gaps_json
    }))))
}

/// Check certification eligibility
pub async fn check_eligibility(
    State(_state): State<Arc<AppState>>,
    Path((contract_id, framework)): Path<(String, String)>,
) -> Result<(StatusCode, Json<CertificationEligibilityResponse>), StatusCode> {
    // For demo: assume 95% compliance and 0 critical gaps
    let (eligible, message) = CertificationManager::check_eligibility(95.0, 0);

    let response = CertificationEligibilityResponse {
        eligible,
        message,
        compliance_percentage: 95.0,
        critical_gaps: 0,
    };

    Ok((StatusCode::OK, Json(response)))
}

fn count_severity_issues(gaps: &[shared::ComplianceGap]) -> (usize, usize, usize, usize) {
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;

    for gap in gaps {
        match gap.severity {
            shared::Severity::Critical => critical += 1,
            shared::Severity::High => high += 1,
            shared::Severity::Medium => medium += 1,
            shared::Severity::Low => low += 1,
            shared::Severity::Info => {},
        }
    }

    (critical, high, medium, low)
}

fn determine_risk_level(compliance_percentage: f64, critical_issues: usize) -> String {
    if critical_issues > 0 {
        "CRITICAL".to_string()
    } else if compliance_percentage < 50.0 {
        "HIGH".to_string()
    } else if compliance_percentage < 75.0 {
        "MEDIUM".to_string()
    } else if compliance_percentage < 100.0 {
        "LOW".to_string()
    } else {
        "COMPLIANT".to_string()
    }
}
