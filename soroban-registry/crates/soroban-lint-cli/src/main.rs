use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use serde_json::json;
use soroban_lint_core::{Analyzer, AutoFixer, Diagnostic, LintConfig, Severity};
use soroban_load_balancer::{
    BalancingAlgorithm, LoadBalancer, LoadBalancerConfig, Region,
};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use walkdir::WalkDir;
use soroban_batch::execute_batch;

#[derive(Parser)]
#[command(name = "soroban-registry")]
#[command(about = "Smart contract linting tool for Soroban", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint smart contracts
    Lint {
        /// Path to contract or directory
        #[arg(default_value = ".")]
        path: String,

        /// Minimum severity level to report
        #[arg(long, default_value = "warning")]
        level: String,

        /// Output format
        #[arg(long, default_value = "human")]
        format: String,

        /// Auto-apply safe fixes
        #[arg(long)]
        fix: bool,

        /// Path to config file
        #[arg(long)]
        config: Option<String>,

        /// Comma-separated rules to run
        #[arg(long)]
        rules: Option<String>,

        /// Additional paths to ignore
        #[arg(long)]
        ignore: Option<String>,
    },

    /// List all available rules
    Rules {
        /// Output format
        #[arg(long, default_value = "human")]
        format: String,
    },

    /// Manage contract load balancing
    Balancer {
        #[command(subcommand)]
        action: BalancerCommands,
    },

    /// Execute batch operations on multiple contracts atomically
    Batch {
        #[command(subcommand)]
        action: BatchCommands,
    },
}

#[derive(Subcommand)]
enum BatchCommands {
    /// Execute a batch operations manifest
    Execute {
        /// Path to the JSON or YAML manifest file
        file: String,

        /// Output format for execution report
        #[arg(long, default_value = "human")]
        format: String,

        /// Dry run - validate manifest without executing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum BalancerCommands {
    /// Start the load balancer with registered instances
    Start {
        /// Algorithm to use: round-robin | least-loaded | geographic
        #[arg(long, default_value = "round-robin")]
        algorithm: String,

        /// Path to instances config JSON file
        #[arg(long)]
        config: String,
    },

    /// Show current load balancer status and metrics
    Status {
        #[arg(long, default_value = "human")]
        format: String,

        /// Path to instances config JSON file (optional)
        #[arg(long)]
        config: Option<String>,
    },

    /// Register a new contract instance
    Register {
        /// Unique instance ID
        #[arg(long)]
        id: String,

        /// Contract ID (Stellar strkey)
        #[arg(long)]
        contract_id: String,

        /// RPC endpoint URL
        #[arg(long)]
        rpc: String,

        /// Geographic region
        #[arg(long, default_value = "us-east")]
        region: String,

        /// Instance weight (higher = more traffic)
        #[arg(long, default_value = "1")]
        weight: u32,
    },

    /// Remove an instance from the pool
    Remove {
        /// Instance ID to remove
        id: String,
    },

    /// Route a test request and show which instance was selected
    Route {
        /// Optional session key for affinity testing
        #[arg(long)]
        session: Option<String>,

        #[arg(long, default_value = "human")]
        format: String,

        /// Path to instances config JSON file (optional)
        #[arg(long)]
        config: Option<String>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Lint {
            path,
            level,
            format,
            fix,
            config,
            rules,
            ignore,
        } => {
            lint_command(path, level, format, fix, config, rules, ignore)?;
        }
        Commands::Rules { format } => {
            rules_command(format)?;
        }
        Commands::Balancer { action } => {
            balancer_command(action)?;
        }
        Commands::Batch { action } => {
            batch_command(action)?;
        }
    }

    Ok(())
}

fn batch_command(action: BatchCommands) -> Result<()> {
    match action {
        BatchCommands::Execute { file, format, dry_run } => {
            let path = PathBuf::from(&file);
            
            // Validate file exists and has correct extension
            if !path.exists() {
                anyhow::bail!("Manifest file '{}' does not exist", file);
            }

            let extension = path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");
            
            if !matches!(extension, "json" | "yaml" | "yml") {
                anyhow::bail!("Manifest file must have .json, .yaml, or .yml extension");
            }

            println!("📋 Loading batch manifest: {}", file.cyan());
            
            if dry_run {
                println!("{}", "🔍 DRY RUN MODE - No operations will be executed".yellow().bold());
            }

            // Use the soroban_batch crate function
            let report = execute_batch(&file, dry_run, &format)?;

            // Exit with error code if any operations failed
            if report.iter().any(|r| r.status == "failed") {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn lint_command(
    path: String,
    level: String,
    format: String,
    fix: bool,
    config_path: Option<String>,
    rules_filter: Option<String>,
    _ignore_filter: Option<String>,
) -> Result<()> {
    let start_time = Instant::now();

    let mut config = LintConfig::load(config_path.as_deref())?;

    if level != "warning" {
        config.lint.level = level.clone();
    }

    let min_severity = config.min_severity();
    let analyzer = Analyzer::new();

    let rule_ids: Vec<&str> = if let Some(rules_str) = &rules_filter {
        rules_str.split(',').collect()
    } else {
        vec![]
    };

    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let path_obj = PathBuf::from(&path);

    if path_obj.is_file() {
        if path_obj.extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(&path)?;
            let file_diags = if rule_ids.is_empty() {
                analyzer.analyze_file(&path, &content)?
            } else {
                analyzer.analyze_file_with_rules(&path, &content, &rule_ids)?
            };
            diagnostics.extend(file_diags);
        }
    } else if path_obj.is_dir() {
        for entry in WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let file_path = entry.path();
            let file_path_str = file_path.to_string_lossy().to_string();

            if config.should_ignore(&file_path_str) {
                continue;
            }

            let content = fs::read_to_string(&file_path)?;
            let file_diags = if rule_ids.is_empty() {
                analyzer.analyze_file(&file_path_str, &content)?
            } else {
                analyzer.analyze_file_with_rules(&file_path_str, &content, &rule_ids)?
            };
            diagnostics.extend(file_diags);
        }
    }

    if fix {
        match AutoFixer::apply_fixes(&diagnostics) {
            Ok(count) => {
                if count > 0 {
                    println!("✅ Applied {} fixes", count);
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to apply fixes: {}", e);
            }
        }
    }

    diagnostics = Analyzer::filter_by_severity(diagnostics, min_severity);
    Analyzer::sort_diagnostics(&mut diagnostics);

    let error_count = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warning_count = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
    let info_count = diagnostics.iter().filter(|d| d.severity == Severity::Info).count();

    let duration = start_time.elapsed();

    if format == "json" {
        output_json(&diagnostics, error_count, warning_count, info_count, duration)?;
    } else {
        output_human(&diagnostics, error_count, warning_count, info_count, duration);
    }

    if error_count > 0 || (warning_count > 0 && min_severity <= Severity::Warning) {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}

fn rules_command(format: String) -> Result<()> {
    let analyzer = Analyzer::new();
    let rules = analyzer.list_rules();

    if format == "json" {
        let rules_json: Vec<_> = rules
            .iter()
            .map(|(id, severity)| {
                json!({
                    "id": id,
                    "severity": format!("{:?}", severity).to_lowercase()
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rules_json)?);
    } else {
        println!("Available Lint Rules:\n");
        for (id, severity) in &rules {
            let severity_str = format!("{:?}", severity).to_lowercase();
            println!("  {} [{}]", id, severity_str);
        }
        println!("\nTotal: {} rules", rules.len());
    }

    Ok(())
}

fn balancer_command(action: BalancerCommands) -> Result<()> {
    match action {
        BalancerCommands::Start { algorithm, config: _config } => {
            let algo = match algorithm.as_str() {
                "least-loaded" => BalancingAlgorithm::LeastLoaded,
                "geographic"   => BalancingAlgorithm::Geographic,
                _              => BalancingAlgorithm::RoundRobin,
            };
            let cfg = LoadBalancerConfig { algorithm: algo, ..Default::default() };
            let lb = LoadBalancer::new(cfg);
            println!("✅ Load balancer started ({} instances registered)", lb.total_count());
        }

        BalancerCommands::Status { format, config: _config } => {
            let lb = LoadBalancer::new(LoadBalancerConfig::default());
            let metrics = lb.metrics();
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&metrics)?);
            } else {
                println!("Healthy instances : {}", lb.healthy_count());
                println!("Total instances   : {}", lb.total_count());
                println!("Active sessions   : (run with a live balancer for live data)");
            }
        }

        BalancerCommands::Register { id, contract_id, rpc, region, weight } => {
            let r = match region.as_str() {
                "us-west"      => Region::UsWest,
                "eu-west"      => Region::EuWest,
                "eu-central"   => Region::EuCentral,
                "ap-southeast" => Region::ApSoutheast,
                "ap-northeast" => Region::ApNortheast,
                _              => Region::UsEast,
            };
            let lb = LoadBalancer::new(LoadBalancerConfig::default());
            lb.register_instance(&id, &contract_id, &rpc, r, weight);
            println!("✅ Registered instance '{}' → {}", id, rpc);
        }

        BalancerCommands::Remove { id } => {
            let lb = LoadBalancer::new(LoadBalancerConfig::default());
            lb.remove_instance(&id);
            println!("✅ Removed instance '{}'", id);
        }

        BalancerCommands::Route { session, format, config: _config } => {
            let lb = LoadBalancer::new(LoadBalancerConfig::default());
            match lb.route(session.as_deref()) {
                Ok(result) => {
                    if format == "json" {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    } else {
                        println!("Routed to instance : {}", result.instance_id);
                        println!("Contract ID        : {}", result.contract_id);
                        println!("RPC endpoint       : {}", result.rpc_endpoint);
                        println!("Algorithm          : {:?}", result.algorithm_used);
                        println!("Session affinity   : {}", result.session_affinity);
                    }
                }
                Err(e) => eprintln!("❌ Routing failed: {}", e),
            }
        }
    }

    Ok(())
}

fn output_human(
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    duration: std::time::Duration,
) {
    for diag in diagnostics {
        let severity_str = match diag.severity {
            Severity::Error => "[ERROR]".red().bold(),
            Severity::Warning => "[WARNING]".yellow().bold(),
            Severity::Info => "[INFO]".cyan(),
        };

        println!("{} {} {}", severity_str, diag.rule_id, diag.span);
        println!("  → {}", diag.message);

        if let Some(suggestion) = &diag.suggestion {
            println!("  Suggestion: {}", suggestion);
        }
        println!();
    }

    let summary = if error_count > 0 {
        format!(
            "Found {} {}, {} {}, {} {}",
            error_count,
            if error_count == 1 { "error" } else { "errors" },
            warning_count,
            if warning_count == 1 { "warning" } else { "warnings" },
            info_count,
            if info_count == 1 { "info" } else { "infos" }
        )
        .red()
        .bold()
    } else if warning_count > 0 {
        format!(
            "Found {} {}, {} {}",
            warning_count,
            if warning_count == 1 { "warning" } else { "warnings" },
            info_count,
            if info_count == 1 { "info" } else { "infos" }
        )
        .yellow()
    } else {
        "No issues found!".green().bold()
    };

    println!(
        "{}. Linting completed in {:.1}s.",
        summary,
        duration.as_secs_f64()
    );
}

fn output_json(
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    duration: std::time::Duration,
) -> Result<()> {
    let output = json!({
        "summary": {
            "errors": error_count,
            "warnings": warning_count,
            "infos": info_count,
            "duration_ms": duration.as_millis()
        },
        "diagnostics": diagnostics
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}