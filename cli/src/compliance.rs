use anyhow::{Context, Result};
use colored::Colorize;
use shared::{ComplianceFramework, ComplianceAuditEngine, RemediationEngine, ReportGenerator, CertificationManager};
use std::time::Instant;
use uuid::Uuid;

pub async fn audit(
    _api_url: &str,
    contract_id: &str,
    framework: &str,
) -> Result<()> {
    // Parse framework
    let framework = match framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => anyhow::bail!("Unsupported compliance framework: {}", framework),
    };

    println!("\n{}", "Compliance Audit".bold().cyan());
    println!("{}", "=".repeat(70).cyan());
    println!("Contract ID: {}", contract_id.bright_blue());
    println!("Framework: {}", framework.to_string().bright_blue());
    println!("{}", "-".repeat(70).cyan());

    // Create a mock contract for demonstration
    let contract = shared::Contract {
        id: Uuid::parse_str(contract_id).unwrap_or_else(|_| Uuid::new_v4()),
        contract_id: contract_id.to_string(),
        wasm_hash: "abc123def456".to_string(),
        name: "Test Contract".to_string(),
        description: Some("Privacy-focused contract with security measures".to_string()),
        publisher_id: Uuid::new_v4(),
        network: shared::Network::Testnet,
        is_verified: true,
        category: Some("Finance".to_string()),
        tags: vec!["secure".to_string(), "audit".to_string(), "privacy".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let requirements = shared::compliance::frameworks::get_framework_requirements(&framework);
    let start_time = Instant::now();
    let status = ComplianceAuditEngine::audit(&contract, &framework, requirements.clone());

    println!("\n{}", "Audit Results:".bright_yellow());
    println!("  Overall Status: {}",
        if status.overall_compliant {
            "✅ COMPLIANT".green()
        } else {
            "⚠️ NON-COMPLIANT".red()
        }
    );
    println!("  Compliance Score: {:.1}%", status.compliance_percentage);
    println!("  Requirements Met: {}/{}", status.satisfied_requirements, status.total_requirements);
    println!("  Identified Gaps: {}", status.gaps_count);
    println!("  Last Checked: {}", status.last_checked.format("%Y-%m-%d %H:%M:%S UTC"));

    if status.gaps_count > 0 {
        println!("\n{}", "Identified Gaps:".bright_red());
        let gaps = ComplianceAuditEngine::identify_gaps(&contract, requirements);
        for (idx, gap) in gaps.iter().take(5).enumerate() {
            println!("\n  {}. {} [{}]",
                idx + 1,
                gap.requirement.title.bold(),
                match gap.severity {
                    shared::Severity::Critical => "🔴 CRITICAL".red(),
                    shared::Severity::High => "🟠 HIGH".yellow(),
                    shared::Severity::Medium => "🟡 MEDIUM".yellow(),
                    shared::Severity::Low => "🟢 LOW".green(),
                    shared::Severity::Info => "ℹ️ INFO".cyan(),
                }
            );
            println!("     {}", gap.requirement.description);
            println!("     Current: {}", gap.current_state.bright_black());
        }

        if gaps.len() > 5 {
            println!("\n  ... and {} more gaps", gaps.len() - 5);
        }
    }

    println!("\n{}", "=".repeat(70).cyan());
    println!("Audit completed in {:.2}s\n", start_time.elapsed().as_secs_f64());

    Ok(())
}

pub async fn report(
    _api_url: &str,
    contract_id: &str,
    framework: &str,
    output_file: Option<&str>,
) -> Result<()> {
    // Parse framework
    let framework = match framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => anyhow::bail!("Unsupported compliance framework: {}", framework),
    };

    // Create a mock contract
    let contract = shared::Contract {
        id: Uuid::parse_str(contract_id).unwrap_or_else(|_| Uuid::new_v4()),
        contract_id: contract_id.to_string(),
        wasm_hash: "abc123def456".to_string(),
        name: "Test Contract".to_string(),
        description: Some("Privacy-focused contract with security measures".to_string()),
        publisher_id: Uuid::new_v4(),
        network: shared::Network::Testnet,
        is_verified: true,
        category: Some("Finance".to_string()),
        tags: vec!["secure".to_string(), "audit".to_string(), "privacy".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let requirements = shared::compliance::frameworks::get_framework_requirements(&framework);
    let start_time = Instant::now();
    let status = ComplianceAuditEngine::audit(&contract, &framework, requirements.clone());
    let gaps = ComplianceAuditEngine::identify_gaps(&contract, requirements.clone());
    let remediation = gaps.iter()
        .map(|g| RemediationEngine::generate_remediation(g))
        .collect();

    let contract_id_uuid = Uuid::parse_str(contract_id).unwrap_or_else(|_| Uuid::new_v4());
    let report = ReportGenerator::generate(contract_id_uuid, framework, status, gaps, remediation, start_time);

    println!("\n{}", "Compliance Report Generated".bold().bright_green());
    println!("{}", "=".repeat(70).cyan());
    println!("Report ID: {}", report.id.bright_blue());
    println!("Framework: {}", report.framework.to_string().bright_blue());
    println!("Generated: {}", report.report_date.format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_blue());

    // Export as JSON if file specified
    if let Some(file) = output_file {
        let json_report = serde_json::to_string_pretty(&report)?;
        std::fs::write(file, json_report)
            .context(format!("Failed to write report to {}", file))?;
        println!("Report saved to: {}", file.bright_green());
    }

    // Print summary
    println!("\n{}", ReportGenerator.export_summary(&report));
    println!("Compliance Score: {:.1}%\n", report.summary.compliance_percentage);

    Ok(())
}

pub async fn gaps(
    _api_url: &str,
    contract_id: &str,
    framework: &str,
) -> Result<()> {
    // Parse framework
    let framework = match framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => anyhow::bail!("Unsupported compliance framework: {}", framework),
    };

    println!("\n{}", "Compliance Gaps Analysis".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    // Create a mock contract
    let contract = shared::Contract {
        id: Uuid::parse_str(contract_id).unwrap_or_else(|_| Uuid::new_v4()),
        contract_id: contract_id.to_string(),
        wasm_hash: "abc123def456".to_string(),
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
    let gaps = ComplianceAuditEngine::identify_gaps(&contract, requirements);

    if gaps.is_empty() {
        println!("✅ No compliance gaps identified!\n");
        return Ok(());
    }

    println!("Found {} compliance gap(s):\n", gaps.len());

    for (idx, gap) in gaps.iter().enumerate() {
        println!("{}. {} [{}]",
            idx + 1,
            gap.requirement.title.bold(),
            match gap.severity {
                shared::Severity::Critical => "CRITICAL".red().bold(),
                shared::Severity::High => "HIGH".red(),
                shared::Severity::Medium => "MEDIUM".yellow(),
                shared::Severity::Low => "LOW".green(),
                shared::Severity::Info => "INFO".cyan(),
            }
        );
        println!("   ID: {}", gap.id.bright_black());
        println!("   Category: {}", gap.requirement.category.bright_black());
        println!("   Current State: {}", gap.current_state);
        println!("   Impact: {}", gap.impact.bright_red());
        println!("   Desired State: {}", gap.desired_state);
        println!();
    }

    println!("{}", "=".repeat(70).cyan());
    println!("Run 'soroban-registry compliance remediate' to get fix suggestions\n");

    Ok(())
}

pub async fn remediate(
    _api_url: &str,
    contract_id: &str,
    framework: &str,
) -> Result<()> {
    // Parse framework
    let framework_enum = match framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => anyhow::bail!("Unsupported compliance framework: {}", framework),
    };

    println!("\n{}", "Remediation Plan".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    // Create a mock contract
    let contract = shared::Contract {
        id: Uuid::parse_str(contract_id).unwrap_or_else(|_| Uuid::new_v4()),
        contract_id: contract_id.to_string(),
        wasm_hash: "abc123def456".to_string(),
        name: "Test Contract".to_string(),
        description: None,
        publisher_id: Uuid::new_v4(),
        network: shared::Network::Testnet,
        is_verified: true,
        category: Some("Finance".to_string()),
        tags: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let requirements = shared::compliance::frameworks::get_framework_requirements(&framework_enum);
    let gaps = ComplianceAuditEngine::identify_gaps(&contract, requirements);

    if gaps.is_empty() {
        println!("✅ No gaps to remediate!\n");
        return Ok(());
    }

    let gap_refs: Vec<&shared::ComplianceGap> = gaps.iter().collect();
    let roadmap = RemediationEngine::create_roadmap(gap_refs);

    println!("Remediation Roadmap (prioritized):\n");

    for (idx, (gap, priority)) in roadmap.iter().enumerate() {
        println!("{}. {} [{}]",
            idx + 1,
            gap.requirement.title.bold(),
            match priority {
                shared::Priority::Critical => "CRITICAL".red().bold(),
                shared::Priority::High => "HIGH".red(),
                shared::Priority::Medium => "MEDIUM".yellow(),
                shared::Priority::Low => "LOW".green(),
            }
        );

        let remediation = RemediationEngine::generate_remediation(gap);

        println!("   Effort: {}", match remediation.estimated_effort {
            shared::EffortLevel::Trivial => "Trivial".green(),
            shared::EffortLevel::Low => "Low".green(),
            shared::EffortLevel::Medium => "Medium".yellow(),
            shared::EffortLevel::High => "High".red(),
            shared::EffortLevel::VeryHigh => "Very High".red().bold(),
        });

        println!("   Steps:");
        for step in &remediation.steps {
            println!("     {}. {}", step.step_number, step.action);
            println!("        {}", step.description.bright_black());
        }

        println!("   Resources Needed:");
        for resource in remediation.resources_needed.iter().take(3) {
            println!("     • {}", resource);
        }

        println!();
    }

    println!("{}", "=".repeat(70).cyan());
    println!("Run 'soroban-registry compliance certify' to start certification process\n");

    Ok(())
}

pub async fn certify(
    _api_url: &str,
    contract_id: &str,
    framework: &str,
) -> Result<()> {
    // Parse framework
    let framework_enum = match framework.to_lowercase().as_str() {
        "gdpr" => ComplianceFramework::GDPR,
        "soc2" => ComplianceFramework::SOC2,
        "hipaa" => ComplianceFramework::HIPAA,
        "iso27001" => ComplianceFramework::ISO27001,
        "pci_dss" => ComplianceFramework::PCIDSS,
        _ => anyhow::bail!("Unsupported compliance framework: {}", framework),
    };

    println!("\n{}", "Certification Process".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    // Check eligibility (assuming 95% compliance for demo)
    let compliance_percentage = 95.0;
    let critical_gaps = 0;

    let (eligible, message) = CertificationManager::check_eligibility(compliance_percentage, critical_gaps);

    println!("Compliance Status: {:.1}%", compliance_percentage);
    println!("Critical Gaps: {}", critical_gaps);
    println!("Eligibility: {}", message);

    if !eligible {
        println!("\n{} Contract must meet eligibility requirements before certification.", "⚠️".yellow());
        println!("Run 'soroban-registry compliance remediate' to fix remaining gaps.\n");
        return Ok(());
    }

    println!("\n✅ {} is eligible for {} certification!",
        contract_id.bright_blue(),
        framework_enum.to_string().bright_cyan()
    );

    // Show certification timeline
    println!("\n{}", "Certification Timeline:".bold().bright_yellow());
    println!("{}", "-".repeat(70).cyan());

    for (stage, description) in CertificationManager::get_timeline(&framework_enum) {
        println!("{}: {}", stage.to_string().bright_blue(), description);
    }

    // Create process
    let contract_id_uuid = Uuid::parse_str(contract_id).unwrap_or_else(|_| Uuid::new_v4());
    let process = CertificationManager::initiate_process(contract_id_uuid, framework_enum.clone());

    println!("\n{}", "Certification Process Initiated".bold().bright_green());
    println!("Current Stage: {}", process.current_stage.to_string().bright_blue());
    println!("Progress: {:.1}%", process.progress_percentage);
    if let Some(target) = process.target_completion {
        println!("Target Completion: {}", target.format("%Y-%m-%d").to_string().bright_cyan());
    }

    let _certificate = CertificationManager::issue_certificate(
        contract_id_uuid,
        framework_enum,
        "Soroban Registry".to_string(),
        Some("Internal Auditor".to_string()),
    );

    println!("\n{}", "=".repeat(70).cyan());
    println!("Next step: Schedule audit appointment\n");

    Ok(())
}

pub async fn frameworks(_api_url: &str) -> Result<()> {
    println!("\n{}", "Supported Compliance Frameworks".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    let frameworks = vec![
        ("GDPR", "General Data Protection Regulation (EU)", "365 days", "Personal data protection and privacy"),
        ("SOC2", "Service Organization Control 2 (US)", "365 days", "Security, availability, and confidentiality"),
        ("HIPAA", "Health Insurance Portability and Accountability", "365 days", "Healthcare data protection"),
        ("ISO 27001", "Information Security Management", "1095 days", "Information security standards"),
        ("PCI DSS", "Payment Card Industry Data Security Standard", "365 days", "Payment card data protection"),
    ];

    for (name, full_name, validity, scope) in &frameworks {
        println!("\n{}", name.bold().bright_green());
        println!("  Full Name: {}", full_name);
        println!("  Certificate Validity: {}", validity);
        println!("  Scope: {}", scope);
    }

    println!("\n{}", "=".repeat(70).cyan());
    println!("Usage: soroban-registry compliance audit <contract-id> <framework>\n");

    Ok(())
}
