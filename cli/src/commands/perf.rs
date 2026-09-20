//! `soroban-registry perf` — profile a contract call.

use crate::support::profiler;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn generate_flame_graph_file(profile: &profiler::ProfileData, output_path: &str) -> Result<()> {
    profiler::generate_flame_graph(profile, Path::new(output_path))
}

pub fn profile(
    contract_path: &str,
    method: Option<&str>,
    output: Option<&str>,
    flamegraph: Option<&str>,
    compare: Option<&str>,
    show_recommendations: bool,
) -> Result<()> {
    println!("\n{}", "Profiling contract execution...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let profile_data = profiler::profile_contract(contract_path, method)
        .with_context(|| format!("Failed to profile contract: {}", contract_path))?;

    if let Some(method_name) = method {
        if profile_data.functions.is_empty() {
            anyhow::bail!(
                "Method '{}' was not found in contract: {}",
                method_name,
                contract_path
            );
        }
    }

    println!("{}: {}", "Contract".bold(), contract_path);
    println!(
        "{}: {:.2}ms",
        "Total duration".bold(),
        profile_data.total_duration.as_secs_f64() * 1000.0
    );
    println!(
        "{}: {}",
        "Functions profiled".bold(),
        profile_data.functions.len()
    );

    if let Some(output_path) = output {
        let profile_json = serde_json::to_string_pretty(&profile_data)
            .context("Failed to serialize profile data")?;
        fs::write(output_path, profile_json)
            .with_context(|| format!("Failed to write profile output: {}", output_path))?;
        println!(
            "{} Profile output written to {}",
            "[OK]".green(),
            output_path
        );
    }

    if let Some(flamegraph_path) = flamegraph {
        generate_flame_graph_file(&profile_data, flamegraph_path)
            .with_context(|| format!("Failed to generate flame graph at {}", flamegraph_path))?;
        println!(
            "{} Flame graph written to {}",
            "[OK]".green(),
            flamegraph_path
        );
    }

    if let Some(baseline_path) = compare {
        let baseline = profiler::load_baseline(baseline_path)
            .with_context(|| format!("Failed to load baseline profile from {}", baseline_path))?;
        let comparisons = profiler::compare_profiles(&baseline, &profile_data);

        println!("\n{}", "Profile comparison:".bold().yellow());
        if comparisons.is_empty() {
            println!("No comparable function data found.");
        } else {
            for change in comparisons.iter().take(10) {
                let diff_ms = change.time_diff_ns as f64 / 1_000_000.0;
                println!(
                    "  {} [{}] {:+.2}% ({:+.3}ms)",
                    change.function.bold(),
                    change.status,
                    change.time_diff_percent,
                    diff_ms
                );
            }
            if comparisons.len() > 10 {
                println!("  ...and {} more", comparisons.len() - 10);
            }
        }
    }

    if show_recommendations {
        let recommendations = profiler::generate_recommendations(&profile_data);
        println!("\n{}", "Recommendations:".bold().magenta());
        for recommendation in recommendations {
            println!("  - {}", recommendation);
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::profiler;
    use std::collections::HashMap;
    use std::fs;
    use std::time::Duration;

    fn sample_profile() -> profiler::ProfileData {
        let mut functions = HashMap::new();
        functions.insert(
            "main".to_string(),
            profiler::FunctionProfile {
                name: "main".to_string(),
                total_time: Duration::from_millis(10),
                call_count: 1,
                avg_time: Duration::from_millis(10),
                min_time: Duration::from_millis(10),
                max_time: Duration::from_millis(10),
                children: vec![],
            },
        );

        profiler::ProfileData {
            contract_path: "contract.rs".to_string(),
            method: Some("main".to_string()),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            total_duration: Duration::from_millis(10),
            functions,
            call_stack: vec![],
            overhead_percent: 0.0,
        }
    }

    fn write_sample_contract(temp_dir: &tempfile::TempDir) -> String {
        let contract_path = temp_dir.path().join("sample_contract.rs");
        fs::write(
            &contract_path,
            "pub fn main() {}\nfn helper_one() {}\nfn helper_two() {}\n",
        )
        .expect("failed to write sample contract");
        contract_path.to_string_lossy().into_owned()
    }

    #[test]
    fn generate_flame_graph_file_writes_svg_for_valid_path() {
        let profile = sample_profile();
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let output_path = temp_dir.path().join("flamegraph-output.svg");
        let output_path_str = output_path.to_string_lossy().into_owned();

        generate_flame_graph_file(&profile, &output_path_str)
            .expect("expected flame graph generation to succeed");
        assert!(output_path.exists(), "expected output file to exist");
    }

    #[test]
    fn generate_flame_graph_file_returns_error_for_invalid_path() {
        let profile = sample_profile();
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let invalid_output = temp_dir
            .path()
            .join("missing-dir")
            .join("flamegraph-output.svg");
        let invalid_output_str = invalid_output.to_string_lossy().into_owned();

        let err = generate_flame_graph_file(&profile, &invalid_output_str)
            .expect_err("expected flame graph generation to fail for invalid path");
        assert!(
            err.to_string().contains("Failed to write flame graph"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn profile_writes_json_and_flamegraph_outputs() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);
        let json_output = temp_dir.path().join("profile-output.json");
        let flame_output = temp_dir.path().join("profile-output.svg");
        let json_output_str = json_output.to_string_lossy().into_owned();
        let flame_output_str = flame_output.to_string_lossy().into_owned();

        profile(
            &contract_path,
            None,
            Some(&json_output_str),
            Some(&flame_output_str),
            None,
            true,
        )
        .expect("expected profiling to succeed");

        assert!(
            json_output.exists(),
            "expected JSON profile output to exist"
        );
        assert!(
            flame_output.exists(),
            "expected flame graph output to exist"
        );
    }

    #[test]
    fn profile_supports_baseline_comparison() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);
        let baseline_path = temp_dir.path().join("baseline.json");
        let baseline_path_str = baseline_path.to_string_lossy().into_owned();

        let baseline_json =
            serde_json::to_string_pretty(&sample_profile()).expect("failed to serialize baseline");
        fs::write(&baseline_path, baseline_json).expect("failed to write baseline file");

        profile(
            &contract_path,
            None,
            None,
            None,
            Some(&baseline_path_str),
            false,
        )
        .expect("expected profiling with baseline comparison to succeed");
    }

    #[test]
    fn profile_returns_error_for_missing_baseline() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);
        let missing_baseline = temp_dir.path().join("missing-baseline.json");
        let missing_baseline_str = missing_baseline.to_string_lossy().into_owned();

        let err = profile(
            &contract_path,
            None,
            None,
            None,
            Some(&missing_baseline_str),
            false,
        )
        .expect_err("expected missing baseline to fail");

        assert!(
            err.to_string()
                .contains("Failed to load baseline profile from"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn profile_returns_error_for_unknown_method() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);

        let err = profile(
            &contract_path,
            Some("does_not_exist"),
            None,
            None,
            None,
            false,
        )
        .expect_err("expected unknown method to fail");

        assert!(
            err.to_string().contains("was not found in contract"),
            "unexpected error: {err}"
        );
    }
}
