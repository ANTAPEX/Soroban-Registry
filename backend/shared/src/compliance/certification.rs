use crate::compliance::ComplianceFramework;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Certification for compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCertificate {
    pub id: String,
    pub contract_id: Uuid,
    pub framework: ComplianceFramework,
    pub issued_date: DateTime<Utc>,
    pub expiration_date: DateTime<Utc>,
    pub issuer: String,
    pub verification_body: Option<String>,
    pub certificate_number: String,
    pub scope: String,
    pub status: CertificateStatus,
}

/// Status of a certificate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CertificateStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "suspended")]
    Suspended,
    #[serde(rename = "revoked")]
    Revoked,
}

/// Certification process state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationProcess {
    pub contract_id: Uuid,
    pub framework: ComplianceFramework,
    pub current_stage: CertificationStage,
    pub started_at: DateTime<Utc>,
    pub target_completion: Option<DateTime<Utc>>,
    pub eligibility_verified: bool,
    pub audit_scheduled: bool,
    pub audit_date: Option<DateTime<Utc>>,
    pub assessor: Option<String>,
    pub progress_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificationStage {
    #[serde(rename = "pre_assessment")]
    PreAssessment,
    #[serde(rename = "audit_preparation")]
    AuditPreparation,
    #[serde(rename = "initial_audit")]
    InitialAudit,
    #[serde(rename = "gap_remediation")]
    GapRemediation,
    #[serde(rename = "final_audit")]
    FinalAudit,
    #[serde(rename = "certificate_issuance")]
    CertificateIssuance,
    #[serde(rename = "maintenance")]
    Maintenance,
}

impl std::fmt::Display for CertificationStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreAssessment => write!(f, "Pre-Assessment"),
            Self::AuditPreparation => write!(f, "Audit Preparation"),
            Self::InitialAudit => write!(f, "Initial Audit"),
            Self::GapRemediation => write!(f, "Gap Remediation"),
            Self::FinalAudit => write!(f, "Final Audit"),
            Self::CertificateIssuance => write!(f, "Certificate Issuance"),
            Self::Maintenance => write!(f, "Maintenance"),
        }
    }
}

/// Certification manager
pub struct CertificationManager;

impl CertificationManager {
    /// Check if a contract is eligible for certification
    pub fn check_eligibility(
        compliance_percentage: f64,
        critical_gaps: usize,
    ) -> (bool, String) {
        if compliance_percentage < 90.0 {
            (
                false,
                format!(
                    "Minimum 90% compliance required. Current: {:.1}%",
                    compliance_percentage
                ),
            )
        } else if critical_gaps > 0 {
            (
                false,
                "All critical gaps must be resolved before certification".to_string(),
            )
        } else {
            (true, "Contract is eligible for certification".to_string())
        }
    }

    /// Initiate certification process
    pub fn initiate_process(
        contract_id: Uuid,
        framework: ComplianceFramework,
    ) -> CertificationProcess {
        CertificationProcess {
            contract_id,
            framework,
            current_stage: CertificationStage::PreAssessment,
            started_at: Utc::now(),
            target_completion: Some(Utc::now() + Duration::days(90)),
            eligibility_verified: false,
            audit_scheduled: false,
            audit_date: None,
            assessor: None,
            progress_percentage: 5.0,
        }
    }

    /// Advance certification to next stage
    pub fn advance_stage(mut process: CertificationProcess) -> CertificationProcess {
        process.current_stage = match process.current_stage {
            CertificationStage::PreAssessment => CertificationStage::AuditPreparation,
            CertificationStage::AuditPreparation => CertificationStage::InitialAudit,
            CertificationStage::InitialAudit => CertificationStage::GapRemediation,
            CertificationStage::GapRemediation => CertificationStage::FinalAudit,
            CertificationStage::FinalAudit => CertificationStage::CertificateIssuance,
            CertificationStage::CertificateIssuance => CertificationStage::Maintenance,
            CertificationStage::Maintenance => CertificationStage::Maintenance,
        };

        process.progress_percentage = Self::calculate_progress(&process.current_stage);
        process
    }

    fn calculate_progress(stage: &CertificationStage) -> f32 {
        match stage {
            CertificationStage::PreAssessment => 10.0,
            CertificationStage::AuditPreparation => 25.0,
            CertificationStage::InitialAudit => 40.0,
            CertificationStage::GapRemediation => 60.0,
            CertificationStage::FinalAudit => 80.0,
            CertificationStage::CertificateIssuance => 95.0,
            CertificationStage::Maintenance => 100.0,
        }
    }

    /// Issue certificate upon successful completion
    pub fn issue_certificate(
        contract_id: Uuid,
        framework: ComplianceFramework,
        issuer: String,
        verification_body: Option<String>,
    ) -> ComplianceCertificate {
        let issued_date = Utc::now();
        let expiration_date = match &framework {
            crate::compliance::ComplianceFramework::GDPR => issued_date + Duration::days(365),
            crate::compliance::ComplianceFramework::SOC2 => issued_date + Duration::days(365),
            crate::compliance::ComplianceFramework::HIPAA => issued_date + Duration::days(365),
            crate::compliance::ComplianceFramework::ISO27001 => issued_date + Duration::days(1095), // 3 years
            crate::compliance::ComplianceFramework::PCIDSS => issued_date + Duration::days(365),
            crate::compliance::ComplianceFramework::Custom(_) => issued_date + Duration::days(365),
        };

        ComplianceCertificate {
            id: format!("CERT-{}-{}", uuid::Uuid::new_v4(), Utc::now().timestamp()),
            contract_id,
            framework: framework.clone(),
            issued_date,
            expiration_date,
            issuer,
            verification_body,
            certificate_number: Self::generate_certificate_number(&framework),
            scope: format!("{} Compliance Certification", framework),
            status: CertificateStatus::Active,
        }
    }

    fn generate_certificate_number(framework: &ComplianceFramework) -> String {
        let prefix = match framework {
            crate::compliance::ComplianceFramework::GDPR => "GDPR",
            crate::compliance::ComplianceFramework::SOC2 => "SOC2",
            crate::compliance::ComplianceFramework::HIPAA => "HIPAA",
            crate::compliance::ComplianceFramework::ISO27001 => "ISO27K1",
            crate::compliance::ComplianceFramework::PCIDSS => "PCI",
            crate::compliance::ComplianceFramework::Custom(_) => "CUSTOM",
        };
        format!("{}-{}-{}", prefix, Utc::now().year(), uuid::Uuid::new_v4().to_string()[0..8].to_uppercase())
    }

    /// Schedule audit
    pub fn schedule_audit(
        mut process: CertificationProcess,
        audit_date: DateTime<Utc>,
        assessor: String,
    ) -> CertificationProcess {
        process.audit_scheduled = true;
        process.audit_date = Some(audit_date);
        process.assessor = Some(assessor);
        process
    }

    /// Get certification timeline
    pub fn get_timeline(framework: &ComplianceFramework) -> Vec<(CertificationStage, String)> {
        vec![
            (
                CertificationStage::PreAssessment,
                "1-2 weeks: Initial assessment and gap analysis".to_string(),
            ),
            (
                CertificationStage::AuditPreparation,
                "2-3 weeks: Prepare documentation and systems".to_string(),
            ),
            (
                CertificationStage::InitialAudit,
                "1-2 weeks: Initial audit by external auditor".to_string(),
            ),
            (
                CertificationStage::GapRemediation,
                match framework {
                    crate::compliance::ComplianceFramework::ISO27001 => "4-8 weeks: Remediate findings".to_string(),
                    _ => "2-4 weeks: Remediate findings".to_string(),
                },
            ),
            (
                CertificationStage::FinalAudit,
                "1-2 weeks: Final audit and verification".to_string(),
            ),
            (
                CertificationStage::CertificateIssuance,
                "1 week: Certificate issuance".to_string(),
            ),
            (
                CertificationStage::Maintenance,
                "Ongoing: Annual audits and compliance monitoring".to_string(),
            ),
        ]
    }

    /// Check certificate validity
    pub fn is_valid(cert: &ComplianceCertificate) -> bool {
        match cert.status {
            CertificateStatus::Active => Utc::now() < cert.expiration_date,
            _ => false,
        }
    }

    /// Get days until expiration
    pub fn days_to_expiration(cert: &ComplianceCertificate) -> Option<i64> {
        if Self::is_valid(cert) {
            Some((cert.expiration_date - Utc::now()).num_days())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eligibility_check() {
        let (eligible, _msg) = CertificationManager::check_eligibility(95.0, 0);
        assert!(eligible);

        let (eligible, _msg) = CertificationManager::check_eligibility(85.0, 0);
        assert!(!eligible);

        let (eligible, _msg) = CertificationManager::check_eligibility(95.0, 1);
        assert!(!eligible);
    }

    #[test]
    fn test_stage_progression() {
        let process = CertificationManager::initiate_process(Uuid::new_v4(), ComplianceFramework::GDPR);
        assert_eq!(process.current_stage, CertificationStage::PreAssessment);

        let process = CertificationManager::advance_stage(process);
        assert_eq!(process.current_stage, CertificationStage::AuditPreparation);
    }
}
