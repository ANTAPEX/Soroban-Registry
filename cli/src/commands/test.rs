//! `soroban-registry test` — run a contract's test suite.

use crate::support::test_framework;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

fn detect_test_command(contract_dir: &Path) -> Option<String> {
    if contract_dir.join("Cargo.toml").exists() {
        return Some("cargo test".to_string());
    }

    if contract_dir.join("package.json").exists() {
        if contract_dir.join("pnpm-lock.yaml").exists() {
            return Some("pnpm test".to_string());
        }
        if contract_dir.join("yarn.lock").exists() {
            return Some("yarn test".to_string());
        }
        return Some("npm test".to_string());
    }

    None
}

fn summarize_failure(stdout: &str, stderr: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    let combined = format!("{}\n{}", stdout, stderr).to_lowercase();

    if combined.contains("failed") || combined.contains("panic") {
        suggestions
            .push("Review failing test output and fix assertions or runtime errors.".to_string());
    }
    if combined.contains("not found") || combined.contains("no such file") {
        suggestions.push("Check file paths and project setup before running tests.".to_string());
    }
    if combined.contains("permission") {
        suggestions
            .push("Verify file permissions and execution rights for test tools.".to_string());
    }

    if suggestions.is_empty() {
        suggestions.push(
            "Inspect test logs above for the first concrete error and address it first."
                .to_string(),
        );
    }

    suggestions
}

fn parse_tarpaulin_percent(report: &serde_json::Value) -> Option<f64> {
    let files = report.get("files")?.as_array()?;

    let mut covered_lines: u64 = 0;
    let mut coverable_lines: u64 = 0;

    for file in files {
        if let Some(traces) = file.get("traces").and_then(|t| t.as_array()) {
            for trace in traces {
                if trace.get("line").and_then(|l| l.as_u64()).is_some() {
                    coverable_lines += 1;
                    if let Some(stats) = trace.get("stats").and_then(|s| s.as_object()) {
                        if stats.values().any(|v| v.as_u64().unwrap_or(0) > 0) {
                            covered_lines += 1;
                        }
                    }
                }
            }
        }
    }

    if coverable_lines == 0 {
        None
    } else {
        Some((covered_lines as f64 / coverable_lines as f64) * 100.0)
    }
}

fn run_rust_coverage(contract_dir: &Path) -> Result<Option<f64>> {
    let output_dir = contract_dir.join(".soroban-registry").join("coverage");
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).with_context(|| {
            format!(
                "Failed to create coverage output dir: {}",
                output_dir.display()
            )
        })?;
    }

    let output_dir_str = output_dir.to_string_lossy().to_string();
    let status = Command::new("cargo")
        .current_dir(contract_dir)
        .args([
            "tarpaulin",
            "--out",
            "Json",
            "--output-dir",
            &output_dir_str,
            "--branch",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            let report_path = output_dir.join("tarpaulin-report.json");
            if !report_path.exists() {
                return Ok(None);
            }

            let content = fs::read_to_string(&report_path).with_context(|| {
                format!("Failed reading coverage report: {}", report_path.display())
            })?;
            let json: serde_json::Value = serde_json::from_str(&content).with_context(|| {
                format!("Failed parsing coverage report: {}", report_path.display())
            })?;
            Ok(parse_tarpaulin_percent(&json))
        }
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TestSuiteOptions<'a> {
    pub test_file: Option<&'a str>,
    pub contract_path: &'a str,
    pub test_command: Option<&'a str>,
    pub junit_output: Option<&'a str>,
    pub show_coverage: bool,
    pub verbose: bool,
    pub require_coverage: bool,
    pub coverage_threshold: f64,
    pub setup_hook: Option<&'a str>,
    pub teardown_hook: Option<&'a str>,
    pub mock_config: Option<&'a str>,
    pub report_output: Option<&'a str>,
    pub profile_output: Option<&'a str>,
    pub load_iterations: u32,
}

fn run_shell_hook(label: &str, command: &str, contract_dir: &Path) -> Result<()> {
    println!("{} {} {}", "→".cyan(), label.bold(), command.bright_blue());
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(contract_dir)
        .status()
        .with_context(|| format!("Failed to execute {} hook: {}", label, command))?;

    if !status.success() {
        anyhow::bail!("{} hook failed: {}", label, command);
    }

    Ok(())
}

fn read_mock_config(path: &str) -> Result<serde_json::Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read mock config: {}", path))?;
    if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml::from_str(&raw)
            .with_context(|| format!("Failed to parse YAML mock config: {}", path))
    } else {
        serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse JSON mock config: {}", path))
    }
}

pub async fn run_contract_tests(
    contract_path: &str,
    test_command: Option<&str>,
    require_coverage: bool,
    coverage_threshold: f64,
    show_coverage: bool,
) -> Result<()> {
    let contract_dir = Path::new(contract_path);
    if !contract_dir.exists() {
        anyhow::bail!("Contract path not found: {}", contract_path);
    }

    let selected_command = if let Some(cmd) = test_command {
        cmd.to_string()
    } else if let Some(cmd) = detect_test_command(contract_dir) {
        cmd
    } else {
        anyhow::bail!(
            "No tests detected. Provide a custom command with --test-command, e.g. --test-command 'cargo test'"
        );
    };

    println!("\n{}", "Running Contract Tests...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{} {}", "Command:".bold(), selected_command.bright_blue());

    let start = std::time::Instant::now();
    let output = if cfg!(windows) {
        let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd".to_string());
        Command::new(comspec)
            .arg("/C")
            .arg(&selected_command)
            .current_dir(contract_dir)
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(&selected_command)
            .current_dir(contract_dir)
            .output()
    }
    .with_context(|| format!("Failed to execute test command: {}", selected_command))?;

    let duration = start.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        println!("{} Tests passed in {:.2}s", "[OK]".green(), duration);
    } else {
        println!("{} Tests failed in {:.2}s", "[ERR]".red(), duration);

        if !stdout.trim().is_empty() {
            println!("\n{}\n{}", "Test output:".bold(), stdout);
        }
        if !stderr.trim().is_empty() {
            println!("\n{}\n{}", "Test errors:".bold().red(), stderr);
        }

        println!("\n{}", "Suggested actions:".bold().yellow());
        for suggestion in summarize_failure(&stdout, &stderr) {
            println!("  - {}", suggestion);
        }

        anyhow::bail!("Contract tests failed. Submission blocked.");
    }

    let is_rust_project = contract_dir.join("Cargo.toml").exists();
    let should_collect_coverage = show_coverage || require_coverage || coverage_threshold > 0.0;

    if should_collect_coverage {
        println!("\n{}", "Coverage:".bold().magenta());
        let coverage = if is_rust_project {
            run_rust_coverage(contract_dir)?
        } else {
            None
        };

        if let Some(percent) = coverage {
            println!("  Total Coverage: {:.2}%", percent);

            if coverage_threshold > 0.0 {
                if percent < coverage_threshold {
                    anyhow::bail!(
                        "Coverage {:.2}% is below required threshold {:.2}%",
                        percent,
                        coverage_threshold
                    );
                } else {
                    println!(
                        "  {} Threshold met ({:.2}% >= {:.2}%)",
                        "[OK]".green(),
                        percent,
                        coverage_threshold
                    );
                }
            }
        } else {
            println!("  {} Coverage metrics unavailable.", "[WARN]".yellow());
            if require_coverage {
                anyhow::bail!(
                    "Coverage is required but could not be collected. Install cargo-tarpaulin or provide coverage-enabled test tooling."
                );
            }
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

pub async fn run_tests(
    test_file: &str,
    contract_path: Option<&str>,
    junit_output: Option<&str>,
    show_coverage: bool,
    verbose: bool,
) -> Result<()> {
    let test_path = Path::new(test_file);
    if !test_path.exists() {
        anyhow::bail!("Test file not found: {}", test_file);
    }

    let contract_dir = contract_path.unwrap_or(".");
    let mut runner = test_framework::TestRunner::new(contract_dir)?;

    println!("\n{}", "Running Integration Tests...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let scenario = test_framework::load_test_scenario(test_path)?;

    if verbose {
        println!("\n{}: {}", "Scenario".bold(), scenario.name);
        if let Some(desc) = &scenario.description {
            println!("{}: {}", "Description".bold(), desc);
        }
        println!("{}: {}", "Steps".bold(), scenario.steps.len());
    }

    let start_time = std::time::Instant::now();
    let result = runner.run_scenario(scenario).await?;
    let total_time = start_time.elapsed();

    println!("\n{}", "Test Results:".bold().green());
    println!("{}", "=".repeat(80).cyan());

    let status_icon = if result.passed { "[OK]" } else { "[ERR]" };

    println!(
        "\n{} {} {} ({:.2}ms)",
        status_icon,
        "Scenario:".bold(),
        result.scenario.bold(),
        result.duration.as_secs_f64() * 1000.0
    );

    if !result.passed {
        if let Some(ref err) = result.error {
            println!("{} {}", "Error:".bold().red(), err);
        }
    }

    println!("\n{}", "Step Results:".bold());
    for (i, step) in result.steps.iter().enumerate() {
        let step_icon = if step.passed { "[OK]" } else { "[ERR]" };

        println!(
            "  {}. {} {} ({:.2}ms)",
            i + 1,
            step_icon,
            step.step_name.bold(),
            step.duration.as_secs_f64() * 1000.0
        );

        if verbose {
            println!(
                "     Assertions: {}/{} passed",
                step.assertions_passed,
                step.assertions_passed + step.assertions_failed
            );
        }

        if let Some(ref err) = step.error {
            println!("     {}", err.red());
        }
    }

    if show_coverage {
        println!("\n{}", "Coverage Report:".bold().magenta());
        println!("  Contracts Tested: {}", result.coverage.contracts_tested);
        println!(
            "  Methods Tested: {}/{}",
            result.coverage.methods_tested, result.coverage.total_methods
        );
        println!("  Coverage: {:.2}%", result.coverage.coverage_percent);

        if result.coverage.coverage_percent < 80.0 {
            println!("  {} Low coverage detected!", "[WARN]".yellow());
        }
    }

    let passed = result.passed;
    if let Some(junit_path) = junit_output {
        test_framework::generate_junit_xml(&[result.clone()], Path::new(junit_path))?;
        println!(
            "\n{} JUnit XML report exported to: {}",
            "[OK]".green(),
            junit_path
        );
    }

    if total_time.as_secs() > 5 {
        println!(
            "\n{} Test execution took {:.2}s (target: <5s)",
            "[WARN]".yellow(),
            total_time.as_secs_f64()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    if !passed {
        anyhow::bail!("Tests failed");
    }

    Ok(())
}

pub async fn run_test_suite(options: TestSuiteOptions<'_>) -> Result<()> {
    let contract_dir = Path::new(options.contract_path);
    let started_at = chrono::Utc::now();
    let wall_clock = std::time::Instant::now();

    if let Some(setup_hook) = options.setup_hook {
        run_shell_hook("Setup hook", setup_hook, contract_dir)?;
    }

    let mock_summary = if let Some(mock_config) = options.mock_config {
        let parsed = read_mock_config(mock_config)?;
        let service_count = parsed
            .get("services")
            .and_then(|services| services.as_array())
            .map(|services| services.len())
            .unwrap_or(0);
        println!(
            "{} Loaded mock config {} ({} service definitions)",
            "[OK]".green(),
            mock_config,
            service_count
        );
        Some(serde_json::json!({
            "path": mock_config,
            "service_count": service_count,
        }))
    } else {
        None
    };

    if options.load_iterations > 1 {
        println!(
            "{} Load profile enabled with {} iterations",
            "→".cyan(),
            options.load_iterations
        );
    }

    let result = if let Some(test_file) = options.test_file {
        run_tests(
            test_file,
            Some(options.contract_path),
            options.junit_output,
            options.show_coverage,
            options.verbose,
        )
        .await
    } else {
        run_contract_tests(
            options.contract_path,
            options.test_command,
            options.require_coverage,
            options.coverage_threshold,
            options.show_coverage,
        )
        .await
    };

    let duration_ms = wall_clock.elapsed().as_millis();
    let error_message = result.as_ref().err().map(|err| err.to_string());

    if let Some(report_output) = options.report_output {
        let report = serde_json::json!({
            "started_at": started_at,
            "contract_path": options.contract_path,
            "test_file": options.test_file,
            "load_iterations": options.load_iterations,
            "passed": result.is_ok(),
            "duration_ms": duration_ms,
            "mocking": mock_summary,
            "error": error_message,
        });
        fs::write(report_output, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("Failed to write test report: {}", report_output))?;
        println!(
            "{} Test report written to {}",
            "[OK]".green(),
            report_output
        );
    }

    if let Some(profile_output) = options.profile_output {
        let profile = serde_json::json!({
            "contract_path": options.contract_path,
            "load_iterations": options.load_iterations,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now(),
        });
        fs::write(profile_output, serde_json::to_string_pretty(&profile)?)
            .with_context(|| format!("Failed to write test profile: {}", profile_output))?;
        println!(
            "{} Test profile written to {}",
            "[OK]".green(),
            profile_output
        );
    }

    let teardown_result = if let Some(teardown_hook) = options.teardown_hook {
        run_shell_hook("Teardown hook", teardown_hook, contract_dir)
    } else {
        Ok(())
    };

    result?;
    teardown_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{detect_test_command, parse_tarpaulin_percent};
    use serde_json::json;

    #[test]
    fn detect_test_command_prefers_cargo_when_cargo_toml_exists() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='x'\nversion='0.1.0'",
        )
        .expect("Cargo.toml should be created");

        let detected = detect_test_command(dir.path());
        assert_eq!(detected.as_deref(), Some("cargo test"));
    }

    #[test]
    fn detect_test_command_uses_pnpm_for_node_projects() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        std::fs::write(dir.path().join("package.json"), "{}")
            .expect("package.json should be created");
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'")
            .expect("pnpm-lock.yaml should be created");

        let detected = detect_test_command(dir.path());
        assert_eq!(detected.as_deref(), Some("pnpm test"));
    }

    #[test]
    fn parse_tarpaulin_percent_calculates_expected_ratio() {
        let report = json!({
            "files": [
                {
                    "traces": [
                        {"line": 1, "stats": {"line": 1}},
                        {"line": 2, "stats": {"line": 0}},
                        {"line": 3, "stats": {"line": 2}}
                    ]
                }
            ]
        });

        let percent = parse_tarpaulin_percent(&report).expect("coverage should parse");
        assert!((percent - 66.666).abs() < 0.5);
    }
}
