use crate::compliance::{ComplianceFramework, ComplianceRequirement, Severity};

/// GDPR compliance requirements
pub fn gdpr_requirements() -> Vec<ComplianceRequirement> {
    vec![
        ComplianceRequirement {
            id: "gdpr_001".to_string(),
            framework: ComplianceFramework::GDPR,
            title: "Data Processing Agreement".to_string(),
            description: "Ensure a Data Processing Agreement (DPA) is in place with all contract participants".to_string(),
            category: "Legal".to_string(),
            mandatory: true,
            check_fn: "check_dpa".to_string(),
        },
        ComplianceRequirement {
            id: "gdpr_002".to_string(),
            framework: ComplianceFramework::GDPR,
            title: "Data Protection Impact Assessment".to_string(),
            description: "Conduct and document a DPIA for high-risk data processing".to_string(),
            category: "Assessment".to_string(),
            mandatory: true,
            check_fn: "check_dpia".to_string(),
        },
        ComplianceRequirement {
            id: "gdpr_003".to_string(),
            framework: ComplianceFramework::GDPR,
            title: "User Consent Mechanisms".to_string(),
            description: "Implement explicit consent mechanisms for data collection and processing".to_string(),
            category: "User Rights".to_string(),
            mandatory: true,
            check_fn: "check_consent".to_string(),
        },
        ComplianceRequirement {
            id: "gdpr_004".to_string(),
            framework: ComplianceFramework::GDPR,
            title: "Data Subject Rights".to_string(),
            description: "Implement mechanisms for right to access, rectification, erasure, and portability".to_string(),
            category: "User Rights".to_string(),
            mandatory: true,
            check_fn: "check_subject_rights".to_string(),
        },
        ComplianceRequirement {
            id: "gdpr_005".to_string(),
            framework: ComplianceFramework::GDPR,
            title: "Data Breach Notification".to_string(),
            description: "Have a documented process for notifying users within 72 hours of a breach".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_breach_notification".to_string(),
        },
        ComplianceRequirement {
            id: "gdpr_006".to_string(),
            framework: ComplianceFramework::GDPR,
            title: "Privacy by Design".to_string(),
            description: "Implement privacy-by-design principles in contract architecture".to_string(),
            category: "Design".to_string(),
            mandatory: true,
            check_fn: "check_privacy_by_design".to_string(),
        },
    ]
}

/// SOC2 compliance requirements
pub fn soc2_requirements() -> Vec<ComplianceRequirement> {
    vec![
        ComplianceRequirement {
            id: "soc2_001".to_string(),
            framework: ComplianceFramework::SOC2,
            title: "Access Control".to_string(),
            description: "Implement role-based access control (RBAC) with audit logging".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_rbac".to_string(),
        },
        ComplianceRequirement {
            id: "soc2_002".to_string(),
            framework: ComplianceFramework::SOC2,
            title: "Change Management".to_string(),
            description: "Maintain documented change management process for contract updates".to_string(),
            category: "Operations".to_string(),
            mandatory: true,
            check_fn: "check_change_management".to_string(),
        },
        ComplianceRequirement {
            id: "soc2_003".to_string(),
            framework: ComplianceFramework::SOC2,
            title: "Encryption".to_string(),
            description: "Use encryption for data in transit and at rest".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_encryption".to_string(),
        },
        ComplianceRequirement {
            id: "soc2_004".to_string(),
            framework: ComplianceFramework::SOC2,
            title: "Incident Response".to_string(),
            description: "Have an incident response plan with documented procedures".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_incident_response".to_string(),
        },
        ComplianceRequirement {
            id: "soc2_005".to_string(),
            framework: ComplianceFramework::SOC2,
            title: "Audit Logging".to_string(),
            description: "Maintain comprehensive audit logs of all contract interactions".to_string(),
            category: "Operations".to_string(),
            mandatory: true,
            check_fn: "check_audit_logging".to_string(),
        },
        ComplianceRequirement {
            id: "soc2_006".to_string(),
            framework: ComplianceFramework::SOC2,
            title: "Availability Targets".to_string(),
            description: "Define and maintain SLA with uptime targets above 99.5%".to_string(),
            category: "Availability".to_string(),
            mandatory: true,
            check_fn: "check_sla".to_string(),
        },
    ]
}

/// HIPAA compliance requirements
pub fn hipaa_requirements() -> Vec<ComplianceRequirement> {
    vec![
        ComplianceRequirement {
            id: "hipaa_001".to_string(),
            framework: ComplianceFramework::HIPAA,
            title: "PHI Encryption".to_string(),
            description: "Encrypt all Protected Health Information (PHI) using AES-256 or equivalent".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_phi_encryption".to_string(),
        },
        ComplianceRequirement {
            id: "hipaa_002".to_string(),
            framework: ComplianceFramework::HIPAA,
            title: "Access Control".to_string(),
            description: "Implement role-based access control with audit trails for PHI".to_string(),
            category: "Access Control".to_string(),
            mandatory: true,
            check_fn: "check_phi_access_control".to_string(),
        },
        ComplianceRequirement {
            id: "hipaa_003".to_string(),
            framework: ComplianceFramework::HIPAA,
            title: "Business Associate Agreement".to_string(),
            description: "Maintain signed BAA with all contract participants handling PHI".to_string(),
            category: "Legal".to_string(),
            mandatory: true,
            check_fn: "check_baa".to_string(),
        },
        ComplianceRequirement {
            id: "hipaa_004".to_string(),
            framework: ComplianceFramework::HIPAA,
            title: "Data Integrity".to_string(),
            description: "Implement mechanisms to ensure accuracy and completeness of PHI".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_data_integrity".to_string(),
        },
    ]
}

/// ISO 27001 compliance requirements
pub fn iso27001_requirements() -> Vec<ComplianceRequirement> {
    vec![
        ComplianceRequirement {
            id: "iso27001_001".to_string(),
            framework: ComplianceFramework::ISO27001,
            title: "Information Security Policy".to_string(),
            description: "Maintain documented information security policy".to_string(),
            category: "Governance".to_string(),
            mandatory: true,
            check_fn: "check_isms_policy".to_string(),
        },
        ComplianceRequirement {
            id: "iso27001_002".to_string(),
            framework: ComplianceFramework::ISO27001,
            title: "Risk Assessment".to_string(),
            description: "Conduct regular information security risk assessments".to_string(),
            category: "Assessment".to_string(),
            mandatory: true,
            check_fn: "check_risk_assessment".to_string(),
        },
        ComplianceRequirement {
            id: "iso27001_003".to_string(),
            framework: ComplianceFramework::ISO27001,
            title: "Access Control".to_string(),
            description: "Implement least privilege access control".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_least_privilege".to_string(),
        },
        ComplianceRequirement {
            id: "iso27001_004".to_string(),
            framework: ComplianceFramework::ISO27001,
            title: "Cryptography".to_string(),
            description: "Use approved cryptographic algorithms and key management".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_cryptography".to_string(),
        },
    ]
}

/// PCI DSS compliance requirements
pub fn pci_dss_requirements() -> Vec<ComplianceRequirement> {
    vec![
        ComplianceRequirement {
            id: "pci_dss_001".to_string(),
            framework: ComplianceFramework::PCIDSS,
            title: "Secure Network Architecture".to_string(),
            description: "Maintain firewall and secure network configuration".to_string(),
            category: "Network".to_string(),
            mandatory: true,
            check_fn: "check_network_security".to_string(),
        },
        ComplianceRequirement {
            id: "pci_dss_002".to_string(),
            framework: ComplianceFramework::PCIDSS,
            title: "Default Credentials".to_string(),
            description: "Change all default passwords and security parameters".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_default_creds".to_string(),
        },
        ComplianceRequirement {
            id: "pci_dss_003".to_string(),
            framework: ComplianceFramework::PCIDSS,
            title: "Cardholder Data Protection".to_string(),
            description: "Encrypt cardholder data and restrict access".to_string(),
            category: "Data Protection".to_string(),
            mandatory: true,
            check_fn: "check_cardholder_data".to_string(),
        },
        ComplianceRequirement {
            id: "pci_dss_004".to_string(),
            framework: ComplianceFramework::PCIDSS,
            title: "Vulnerability Management".to_string(),
            description: "Maintain vulnerability scanning and patching program".to_string(),
            category: "Security".to_string(),
            mandatory: true,
            check_fn: "check_vulnerability_management".to_string(),
        },
    ]
}

pub fn all_frameworks() -> Vec<ComplianceFramework> {
    vec![
        ComplianceFramework::GDPR,
        ComplianceFramework::SOC2,
        ComplianceFramework::HIPAA,
        ComplianceFramework::ISO27001,
        ComplianceFramework::PCIDSS,
    ]
}

pub fn get_framework_requirements(framework: &ComplianceFramework) -> Vec<ComplianceRequirement> {
    match framework {
        ComplianceFramework::GDPR => gdpr_requirements(),
        ComplianceFramework::SOC2 => soc2_requirements(),
        ComplianceFramework::HIPAA => hipaa_requirements(),
        ComplianceFramework::ISO27001 => iso27001_requirements(),
        ComplianceFramework::PCIDSS => pci_dss_requirements(),
        ComplianceFramework::Custom(_) => vec![],
    }
}
