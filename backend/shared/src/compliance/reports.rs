use crate::compliance::{ComplianceStatus, ComplianceGap, RemediationAdvice, ComplianceFramework};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Comprehensive compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: String,
    pub contract_id: Uuid,
    pub framework: ComplianceFramework,
    pub report_date: DateTime<Utc>,
    pub summary: ReportSummary,
    pub compliance_status: ComplianceStatus,
    pub identified_gaps: Vec<ComplianceGap>,
    pub remediation_advice: Vec<RemediationAdvice>,
    pub recommendations: Vec<String>,
    pub generation_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_requirements: usize,
    pub satisfied_requirements: usize,
    pub gaps_count: usize,
    pub critical_issues: usize,
    pub high_issues: usize,
    pub medium_issues: usize,
    pub low_issues: usize,
    pub compliance_percentage: f64,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "compliant")]
    Compliant,
}

/// Report generator
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate a comprehensive compliance report
    pub fn generate(
        contract_id: Uuid,
        framework: ComplianceFramework,
        status: ComplianceStatus,
        gaps: Vec<ComplianceGap>,
        remediation: Vec<RemediationAdvice>,
        start_time: std::time::Instant,
    ) -> ComplianceReport {
        let (critical, high, medium, low) = Self::count_severity_issues(&gaps);

        let risk_level = Self::determine_risk_level(
            status.compliance_percentage,
            critical,
            status.gaps_count,
        );

        let recommendations = Self::generate_recommendations(&framework, &gaps, status.overall_compliant);

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        let summary = ReportSummary {
            total_requirements: status.total_requirements,
            satisfied_requirements: status.satisfied_requirements,
            gaps_count: status.gaps_count,
            critical_issues: critical,
            high_issues: high,
            medium_issues: medium,
            low_issues: low,
            compliance_percentage: status.compliance_percentage,
            risk_level,
        };

        ComplianceReport {
            id: uuid::Uuid::new_v4().to_string(),
            contract_id,
            framework,
            report_date: Utc::now(),
            summary,
            compliance_status: status,
            identified_gaps: gaps,
            remediation_advice: remediation,
            recommendations,
            generation_time_ms,
        }
    }

    fn count_severity_issues(gaps: &[ComplianceGap]) -> (usize, usize, usize, usize) {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;

        for gap in gaps {
            match gap.severity {
                crate::compliance::Severity::Critical => critical += 1,
                crate::compliance::Severity::High => high += 1,
                crate::compliance::Severity::Medium => medium += 1,
                crate::compliance::Severity::Low => low += 1,
                crate::compliance::Severity::Info => {},
            }
        }

        (critical, high, medium, low)
    }

    fn determine_risk_level(
        compliance_percentage: f64,
        critical_issues: usize,
        gap_count: usize,
    ) -> RiskLevel {
        if critical_issues > 0 {
            RiskLevel::Critical
        } else if compliance_percentage < 50.0 {
            RiskLevel::High
        } else if compliance_percentage < 75.0 {
            RiskLevel::Medium
        } else if compliance_percentage < 100.0 {
            RiskLevel::Low
        } else {
            RiskLevel::Compliant
        }
    }

    fn generate_recommendations(
        framework: &ComplianceFramework,
        gaps: &[ComplianceGap],
        overall_compliant: bool,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if overall_compliant {
            recommendations.push("✓ Contract is in compliance. Maintain current practices and conduct regular audits.".to_string());
            recommendations.push("Consider pursuing certification to demonstrate compliance commitment.".to_string());
        } else {
            if gaps.iter().any(|g| matches!(g.severity, crate::compliance::Severity::Critical)) {
                recommendations.push("⚠️ URGENT: Address all critical issues immediately to avoid compliance violations.".to_string());
            }

            recommendations.push(format!(
                "Create remediation plan for identified gaps. Estimated remediation time varies by gap complexity."
            ));

            recommendations.push("Schedule regular compliance audits (at least quarterly) to maintain compliance.".to_string());

            match framework {
                crate::compliance::ComplianceFramework::GDPR => {
                    recommendations.push("Engage legal counsel to review Data Processing Agreements.".to_string());
                    recommendations.push("Implement privacy impact assessment procedures.".to_string());
                }
                crate::compliance::ComplianceFramework::SOC2 => {
                    recommendations.push("Schedule SOC2 audit by external auditor.".to_string());
                    recommendations.push("Implement comprehensive logging and monitoring.".to_string());
                }
                crate::compliance::ComplianceFramework::HIPAA => {
                    recommendations.push("Conduct HIPAA risk assessment with security officer.".to_string());
                    recommendations.push("Ensure all Business Associates are under signed BAAs.".to_string());
                }
                crate::compliance::ComplianceFramework::ISO27001 => {
                    recommendations.push("Develop Information Security Management System (ISMS).".to_string());
                    recommendations.push("Consider ISO 27001 certification by accredited body.".to_string());
                }
                crate::compliance::ComplianceFramework::PCIDSS => {
                    recommendations.push("Conduct PCI DSS assessment by Qualified Security Assessor (QSA).".to_string());
                    recommendations.push("Implement network segmentation and vulnerability management program.".to_string());
                }
                crate::compliance::ComplianceFramework::Custom(_) => {
                    recommendations.push("Review custom compliance requirements and develop remediation plan.".to_string());
                }
            }
        }

        recommendations
    }

    /// Generate report in multiple formats
    pub fn export_json(&self, report: &ComplianceReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }

    pub fn export_summary(&self, report: &ComplianceReport) -> String {
        format!(
            r#"
╔══════════════════════════════════════════════════════════════════╗
║         COMPLIANCE REPORT - {}
║
║ Contract ID:              {}
║ Framework:                {}
║ Report Date:              {}
║ Generation Time:          {}ms
║
╠══════════════════════════════════════════════════════════════════╣
║ SUMMARY
║
║ Compliance Status:        {}%
║ Requirements Met:         {}/{}
║ Identified Gaps:          {}
║ Risk Level:               {}
║
║ Severity Breakdown:
║   Critical Issues:        {}
║   High Issues:            {}
║   Medium Issues:          {}
║   Low Issues:             {}
║
╠══════════════════════════════════════════════════════════════════╣
║ IDENTIFIED GAPS: {}
║
"#,
            report.framework,
            report.contract_id,
            report.framework,
            report.report_date.format("%Y-%m-%d %H:%M:%S UTC"),
            report.generation_time_ms,
            report.summary.compliance_percentage as i32,
            report.summary.satisfied_requirements,
            report.summary.total_requirements,
            report.summary.gaps_count,
            match report.summary.risk_level {
                RiskLevel::Critical => "🔴 CRITICAL",
                RiskLevel::High => "🟠 HIGH",
                RiskLevel::Medium => "🟡 MEDIUM",
                RiskLevel::Low => "🟢 LOW",
                RiskLevel::Compliant => "✅ COMPLIANT",
            },
            report.summary.critical_issues,
            report.summary.high_issues,
            report.summary.medium_issues,
            report.summary.low_issues,
            report.identified_gaps.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_determination() {
        assert_eq!(ReportGenerator::determine_risk_level(99.0, 0, 1), RiskLevel::Low);
        assert_eq!(ReportGenerator::determine_risk_level(50.0, 0, 5), RiskLevel::Medium);
        assert_eq!(ReportGenerator::determine_risk_level(40.0, 0, 8), RiskLevel::High);
        assert_eq!(ReportGenerator::determine_risk_level(80.0, 1, 2), RiskLevel::Critical);
    }
}
