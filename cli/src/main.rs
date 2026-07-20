mod backup;
mod commands;
mod config;
mod coverage;
mod events;
mod export;
mod formal_verification;
mod fuzz;
mod import;
mod incident;
mod logs;
mod manifest;
mod migration;
mod multisig;
mod package_signing;
mod patch;
mod profiler;
mod sla;
mod test_framework;
mod wizard;

use anyhow::Result;
use clap::{Parser, Subcommand};
use patch::Severity;
use std::io::Write;

/// Soroban Registry CLI — discover, publish, verify, and deploy Soroban contracts
#[derive(Debug, Parser)]
#[command(name = "soroban-registry", version, about, long_about = None)]
pub struct Cli {
    /// Registry API URL
    #[arg(
        long,
        env = "SOROBAN_REGISTRY_API_URL",
        default_value = "http://localhost:3001"
    )]
    pub api_url: String,

    /// Stellar network to use (mainnet | testnet | futurenet)
    #[arg(long, global = true)]
    pub network: Option<String>,

    /// Enable verbose output (shows HTTP requests, responses, and debug info)
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Search for contracts in the registry
    Search {
        /// Search query
        query: String,
        /// Only show verified contracts
        #[arg(long)]
        verified_only: bool,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Get detailed information about a contract
    Info {
        /// Contract ID to look up
        contract_id: String,
    },

    /// Publish a new contract to the registry
    Publish {
        /// On-chain contract ID
        #[arg(long)]
        contract_id: String,

        /// Human-readable contract name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Network (mainnet, testnet, futurenet)
        #[arg(long, default_value = "Testnet")]
        network: String,

        /// Category
        #[arg(long)]
        category: Option<String>,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Publisher Stellar address
        #[arg(long)]
        publisher: String,
    },

    /// List recent contracts
    List {
        /// Maximum number of contracts to show
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Detect breaking changes between contract versions
    BreakingChanges {
        /// Old contract identifier (UUID or contract_id@version)
        old_id: String,
        /// New contract identifier (UUID or contract_id@version)
        new_id: String,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Contract state migration assistant
    Migrate {
        #[command(subcommand)]
        action: MigrateCommands,
    },

    /// Export a contract archive (.tar.gz)
    Export {
        /// Contract registry ID (UUID)
        #[arg(long)]
        id: String,

        /// Output archive path
        #[arg(long, default_value = "contract-export.tar.gz")]
        output: String,

        /// Path to contract source directory
        #[arg(long, default_value = ".")]
        contract_dir: String,
    },

    /// Import a contract from an archive
    Import {
        /// Path to the archive file
        archive: String,

        /// Directory to extract into
        #[arg(long, default_value = "./imported")]
        output_dir: String,
    },

    /// Generate documentation from a contract WASM
    Doc {
        /// Path to contract WASM file
        contract_path: String,

        /// Output directory
        #[arg(long, default_value = "docs")]
        output: String,
    },

    /// Launch the interactive setup wizard
    Wizard {},

    /// View and manage CLI logs
    Log {
        /// Show the last N matching log entries
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Follow new log entries (tail -f)
        #[arg(long)]
        tail: bool,

        /// Filter by level (info|warn|error|debug)
        #[arg(long, value_enum)]
        level: Option<logs::LogLevel>,

        /// Filter by service/component name
        #[arg(long)]
        service: Option<String>,

        /// Search substring across message + context
        #[arg(long)]
        search: Option<String>,

        /// Export output to a file
        #[arg(long)]
        export: Option<String>,
    },

    /// Show command history
    History {
        /// Filter by search term
        #[arg(long)]
        search: Option<String>,

        /// Maximum number of entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Security patch management
    Patch {
        #[command(subcommand)]
        action: PatchCommands,
    },

    /// Incident response management
    Incident {
        #[command(subcommand)]
        action: IncidentCommands,
    },

    /// Multi-signature contract deployment workflow
    Multisig {
        #[command(subcommand)]
        action: MultisigCommands,
    },

    /// Fuzz testing for contracts
    Fuzz {
        #[arg(long)]
        contract_path: String,
        #[arg(long)]
        duration: u64,
        #[arg(long)]
        timeout: u64,
        #[arg(long)]
        threads: u32,
        #[arg(long)]
        max_cases: u32,
        #[arg(long)]
        output: String,
        #[arg(long)]
        minimize: bool,
    },

    /// Profile contract execution performance
    Profile {
        /// Path to contract file
        contract_path: String,

        /// Method to profile
        #[arg(long)]
        method: Option<String>,

        /// Output JSON file
        #[arg(long)]
        output: Option<String>,

        /// Generate flame graph
        #[arg(long)]
        flamegraph: Option<String>,

        /// Compare with baseline profile
        #[arg(long)]
        compare: Option<String>,

        /// Show recommendations
        #[arg(long, default_value = "true")]
        recommendations: bool,
    },

    /// Run integration tests
    Test {
        /// Path to test file (YAML or JSON)
        test_file: String,

        /// Path to contract directory or file
        #[arg(long)]
        contract_path: Option<String>,

        /// Output JUnit XML report
        #[arg(long)]
        junit: Option<String>,

        /// Show coverage report
        #[arg(long, default_value = "true")]
        coverage: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// SLA compliance monitoring
    Sla {
        #[command(subcommand)]
        action: SlaCommands,
    },

    Config {
        #[command(subcommand)]
        action: ConfigSubcommands,
    },

    /// Run formal verification analysis against a deployed or local contract
    VerifyFormal {
        /// Path to contract file
        contract_path: String,

        /// Path to properties DSL file
        #[arg(long)]
        properties: String,

        /// Output format (json or text)
        #[arg(long, default_value = "text")]
        output: String,

        /// Post results back to registry
        #[arg(long)]
        post: bool,
    },

    ScanDeps {
        #[arg(long)]
        contract_id: String,
        #[arg(long, default_value = ",")]
        dependencies: String,
        #[arg(long, default_value_t = false)]
        fail_on_high: bool,
    },

    /// Measure and report code coverage for contract tests
    Coverage {
        /// Path to contract directory
        contract_path: String,

        /// Path to test directory or file
        #[arg(long)]
        tests: String,

        /// Fail if coverage is below this threshold (0-100)
        #[arg(long, default_value_t = 0.0)]
        threshold: f64,

        /// Output directory for HTML reports
        #[arg(long, default_value = "coverage_report")]
        output: String,
    },

    /// Sign a contract package with your private key
    Sign {
        /// Path to the package file to sign
        package: String,

        /// Private key (base64-encoded Ed25519)
        #[arg(long)]
        private_key: String,

        /// Contract ID
        #[arg(long)]
        contract_id: String,

        /// Package version
        #[arg(long)]
        version: String,

        /// Signature expiration (RFC3339 format)
        #[arg(long)]
        expires_at: Option<String>,
    },

    /// Verify a signed contract package
    Verify {
        /// Path to the package file to verify
        package: String,

        /// Contract ID
        #[arg(long)]
        contract_id: String,

        /// Package version (optional)
        #[arg(long)]
        version: Option<String>,

        /// Signature (base64, optional - will lookup from registry if not provided)
        #[arg(long)]
        signature: Option<String>,
    },

    /// Manage signing keys and signatures
    Keys {
        #[command(subcommand)]
        action: KeysCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommands {
    Get {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    Set {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
        #[arg(long)]
        config_data: String,
        #[arg(long)]
        secrets_data: Option<String>,
        #[arg(long)]
        created_by: String,
    },
    History {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    Rollback {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
        #[arg(long)]
        version: i32,
        #[arg(long)]
        created_by: String,
    },
}

/// Sub-commands for the `sla` group
#[derive(Debug, Subcommand)]
pub enum SlaCommands {
    /// Record hourly SLA metrics for a contract
    Record {
        /// Contract identifier
        id: String,
        /// Uptime percentage (0-100)
        uptime: f64,
        /// Average latency in milliseconds
        latency: f64,
        /// Error rate percentage (0-100)
        error_rate: f64,
    },
    /// Show real-time SLA compliance dashboard
    Status {
        /// Contract identifier
        id: String,
    },
}

/// Sub-commands for the `multisig` group
#[derive(Debug, Subcommand)]
pub enum MultisigCommands {
    /// Create a new multi-sig policy (defines signers and required threshold)
    CreatePolicy {
        #[arg(long)]
        name: String,
        #[arg(long)]
        threshold: u32,
        #[arg(long)]
        signers: String,
        #[arg(long)]
        expiry_secs: Option<u32>,
        #[arg(long)]
        created_by: String,
    },

    /// Create an unsigned deployment proposal
    CreateProposal {
        #[arg(long)]
        contract_name: String,
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        wasm_hash: String,
        #[arg(long, default_value = "testnet")]
        network: String,
        #[arg(long)]
        policy_id: String,
        #[arg(long)]
        proposer: String,
        #[arg(long)]
        description: Option<String>,
    },

    /// Sign a deployment proposal (add your approval)
    Sign {
        proposal_id: String,
        #[arg(long)]
        signer: String,
        #[arg(long)]
        signature_data: Option<String>,
    },

    /// Execute an approved deployment proposal
    Execute { proposal_id: String },

    /// Show full info for a proposal (signatures, policy, status)
    Info { proposal_id: String },

    /// List deployment proposals
    ListProposals {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

/// Sub-commands for the `incident` group
#[derive(Debug, Subcommand)]
pub enum IncidentCommands {
    /// Trigger a new incident for a contract
    Trigger {
        /// On-chain contract ID
        contract_id: String,
        /// Incident severity (critical|high|medium|low)
        #[arg(long)]
        severity: String,
    },
    /// Update the state of an existing incident
    Update {
        /// Incident UUID returned by trigger
        incident_id: String,
        /// New state (detected|responding|contained|recovered|post_review)
        #[arg(long)]
        state: String,
    },
}

/// Sub-commands for the `patch` group
#[derive(Debug, Subcommand)]
pub enum PatchCommands {
    /// Create a new security patch
    Create {
        #[arg(long)]
        version: String,
        #[arg(long)]
        hash: String,
        #[arg(long)]
        severity: String,
        #[arg(long, default_value = "100")]
        rollout: u8,
    },
    /// Notify subscribers about a patch
    Notify {
        #[arg(long)]
        patch_id: String,
    },
    /// Apply a patch to a specific contract
    Apply {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        patch_id: String,
    },
    /// Manage contract dependencies
    Deps {
        #[command(subcommand)]
        command: DepsCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum DepsCommands {
    /// List dependencies for a contract
    List {
        /// Contract ID
        contract_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeysCommands {
    /// Generate a new Ed25519 keypair for signing
    Generate {},

    /// Revoke a signature
    Revoke {
        /// Signature ID to revoke
        signature_id: String,
        /// Address of the revoker
        #[arg(long)]
        revoked_by: String,
        /// Reason for revocation
        #[arg(long)]
        reason: String,
    },

    /// Show chain of custody for a contract
    Custody {
        /// Contract ID
        contract_id: String,
    },

    /// View transparency log
    Log {
        /// Filter by contract ID
        #[arg(long)]
        contract_id: Option<String>,
        /// Filter by entry type
        #[arg(long)]
        entry_type: Option<String>,
        /// Maximum entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

/// Sub-commands for contract migration workflow
#[derive(Debug, Subcommand)]
pub enum MigrateCommands {
    /// Preview migration outcome (dry-run)
    Preview {
        old_id: String,
        new_id: String,
    },
    /// Analyze schema differences between versions
    Analyze {
        old_id: String,
        new_id: String,
    },
    /// Generate migration script template (rust|js)
    Generate {
        old_id: String,
        new_id: String,
        #[arg(long, default_value = "rust")]
        language: String,
        #[arg(long)]
        output: Option<String>,
    },
    /// Validate migration for data loss risks
    Validate {
        old_id: String,
        new_id: String,
    },
    /// Apply migration and record history
    Apply {
        old_id: String,
        new_id: String,
    },
    /// Rollback a migration by migration ID
    Rollback { migration_id: String },
    /// Show migration history
    History {
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // ── Initialise logger ─────────────────────────────────────────────────────
    // --verbose / -v  →  DEBUG level (shows HTTP calls, payloads, timing)
    // default         →  WARN level  (only errors and warnings)
    let log_level = if cli.verbose { "debug" } else { "warn" };
    env_logger::Builder::new()
        .parse_filters(log_level)
        .format_timestamp(None) // no timestamps in CLI output
        .format_module_path(cli.verbose) // show module path only in verbose
        .init();

    log::debug!("Verbose mode enabled");
    log::debug!("API URL: {}", cli.api_url);

    // ── Resolve network ───────────────────────────────────────────────────────
    let network = config::resolve_network(cli.network)?;
    log::debug!("Network: {:?}", network);

    let _ = logs::append_log(&logs::LogEntry {
        timestamp: chrono::Utc::now(),
        level: logs::LogLevel::Info,
        service: "cli".to_string(),
        message: "command_invoked".to_string(),
        context: Some(serde_json::json!({
            "command": format!("{:?}", &cli.command),
            "api_url": &cli.api_url,
            "network": network.to_string(),
        })),
    });

    match cli.command {
        Commands::Search {
            query,
            verified_only,
            json,
        } => {
            log::debug!(
                "Command: search | query={:?} verified_only={}",
                query,
                verified_only
            );
            commands::search(&cli.api_url, &query, network, verified_only, json).await?;
        }
        Commands::Info { contract_id } => {
            log::debug!("Command: info | contract_id={}", contract_id);
            commands::info(&cli.api_url, &contract_id, network).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            network: _, // we use the global network
            category,
            tags,
            publisher,
        } => {
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: publish | contract_id={} name={} tags={:?}",
                contract_id,
                name,
                tags_vec
            );
            commands::publish(
                &cli.api_url,
                &contract_id,
                &name,
                description.as_deref(),
                network,
                category.as_deref(),
                tags_vec,
                &publisher,
            )
            .await?;
        }
        Commands::List { limit, json } => {
            log::debug!("Command: list | limit={}", limit);
            commands::list(&cli.api_url, limit, network, json).await?;
        }
        Commands::BreakingChanges { old_id, new_id, json } => {
            log::debug!("Command: breaking-changes | old={} new={}", old_id, new_id);
            commands::breaking_changes(&cli.api_url, &old_id, &new_id, json).await?;
        }
        Commands::Migrate { action } => match action {
            MigrateCommands::Preview { old_id, new_id } => {
                log::debug!("Command: migrate preview | old_id={} new_id={}", old_id, new_id);
                migration::preview(&old_id, &new_id)?;
            }
            MigrateCommands::Analyze { old_id, new_id } => {
                log::debug!("Command: migrate analyze | old_id={} new_id={}", old_id, new_id);
                migration::analyze(&old_id, &new_id)?;
            }
            MigrateCommands::Generate {
                old_id,
                new_id,
                language,
                output,
            } => {
                log::debug!(
                    "Command: migrate generate | old_id={} new_id={} language={}",
                    old_id,
                    new_id,
                    language
                );
                migration::generate_template(&old_id, &new_id, &language, output.as_deref())?;
            }
            MigrateCommands::Validate { old_id, new_id } => {
                log::debug!("Command: migrate validate | old_id={} new_id={}", old_id, new_id);
                migration::validate(&old_id, &new_id)?;
            }
            MigrateCommands::Apply { old_id, new_id } => {
                log::debug!("Command: migrate apply | old_id={} new_id={}", old_id, new_id);
                migration::apply(&old_id, &new_id)?;
            }
            MigrateCommands::Rollback { migration_id } => {
                log::debug!("Command: migrate rollback | migration_id={}", migration_id);
                migration::rollback(&migration_id)?;
            }
            MigrateCommands::History { limit } => {
                log::debug!("Command: migrate history | limit={}", limit);
                migration::history(limit)?;
            }
        },
        Commands::Export {
            id,
            output,
            contract_dir,
        } => {
            log::debug!("Command: export | id={} output={}", id, output);
            commands::export(&cli.api_url, &id, &output, &contract_dir).await?;
        }
        Commands::Import {
            archive,
            output_dir,
        } => {
            log::debug!(
                "Command: import | archive={} output_dir={}",
                archive,
                output_dir
            );
            commands::import(&cli.api_url, &archive, network, &output_dir).await?;
        }
        Commands::Doc {
            contract_path,
            output,
        } => {
            log::debug!(
                "Command: doc | contract_path={} output={}",
                contract_path,
                output
            );
            commands::doc(&contract_path, &output)?;
        }
        Commands::Wizard {} => {
            log::debug!("Command: wizard");
            wizard::run(&cli.api_url).await?;
        }
        Commands::Log {
            limit,
            tail,
            level,
            service,
            search,
            export,
        } => {
            let path = logs::default_log_path()?;
            let filter = logs::LogFilter { level, service, search };

            if tail {
                let initial = logs::read_logs(&path, &filter, limit)?;
                if initial.is_empty() {
                    println!("No logs found.");
                } else {
                    for entry in &initial {
                        println!("{}", logs::format_entry(entry, true));
                    }
                }

                if let Some(out_path) = export {
                    let mut file = std::fs::File::create(&out_path)?;
                    for entry in initial {
                        writeln!(file, "{}", logs::format_entry(&entry, false))?;
                    }
                }

                logs::follow_logs(&path, filter, move |entry| {
                    println!("{}", logs::format_entry(&entry, true));
                })
                .await?;
            } else {
                let entries = logs::read_logs(&path, &filter, limit)?;
                if entries.is_empty() {
                    println!("No logs found.");
                } else if let Some(out_path) = export {
                    let mut file = std::fs::File::create(&out_path)?;
                    for entry in entries {
                        writeln!(file, "{}", logs::format_entry(&entry, false))?;
                    }
                    println!("Exported logs to {}", out_path);
                } else {
                    for entry in entries {
                        println!("{}", logs::format_entry(&entry, true));
                    }
                }
            }
        }
        Commands::History { search, limit } => {
            log::debug!("Command: history | search={:?} limit={}", search, limit);
            wizard::show_history(search.as_deref(), limit)?;
        }
        Commands::Incident { action } => match action {
            IncidentCommands::Trigger {
                contract_id,
                severity,
            } => {
                log::debug!(
                    "Command: incident trigger | contract_id={} severity={}",
                    contract_id,
                    severity
                );
                commands::incident_trigger(&contract_id, &severity)?;
            }
            IncidentCommands::Update { incident_id, state } => {
                log::debug!(
                    "Command: incident update | incident_id={} state={}",
                    incident_id,
                    state
                );
                commands::incident_update(&incident_id, &state)?;
            }
        },
        Commands::Patch { action } => match action {
            PatchCommands::Create {
                version,
                hash,
                severity,
                rollout,
            } => {
                let sev = severity.parse::<Severity>()?;
                log::debug!(
                    "Command: patch create | version={} rollout={}",
                    version,
                    rollout
                );
                commands::patch_create(&cli.api_url, &version, &hash, sev, rollout).await?;
            }
            PatchCommands::Notify { patch_id } => {
                log::debug!("Command: patch notify | patch_id={}", patch_id);
                commands::patch_notify(&cli.api_url, &patch_id).await?;
            }
            PatchCommands::Apply {
                contract_id,
                patch_id,
            } => {
                log::debug!(
                    "Command: patch apply | contract_id={} patch_id={}",
                    contract_id,
                    patch_id
                );
                commands::patch_apply(&cli.api_url, &contract_id, &patch_id).await?;
            }
            PatchCommands::Deps { command } => match command {
                DepsCommands::List { contract_id } => {
                    commands::deps_list(&cli.api_url, &contract_id).await?;
                }
            },
        },
        // ── Multi-sig commands (issue #47) ───────────────────────────────────
        Commands::Multisig { action } => match action {
            MultisigCommands::CreatePolicy {
                name,
                threshold,
                signers,
                expiry_secs,
                created_by,
            } => {
                let signer_vec: Vec<String> =
                    signers.split(',').map(|s| s.trim().to_string()).collect();
                log::debug!(
                    "Command: multisig create-policy | name={} threshold={} signers={:?}",
                    name,
                    threshold,
                    signer_vec
                );
                multisig::create_policy(
                    &cli.api_url,
                    &name,
                    threshold,
                    signer_vec,
                    expiry_secs,
                    &created_by,
                )
                .await?;
            }
            MultisigCommands::CreateProposal {
                contract_name,
                contract_id,
                wasm_hash,
                network: net_str,
                policy_id,
                proposer,
                description,
            } => {
                log::debug!(
                    "Command: multisig create-proposal | contract_id={} policy_id={}",
                    contract_id,
                    policy_id
                );
                multisig::create_proposal(
                    &cli.api_url,
                    &contract_name,
                    &contract_id,
                    &wasm_hash,
                    &net_str,
                    &policy_id,
                    &proposer,
                    description.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Sign {
                proposal_id,
                signer,
                signature_data,
            } => {
                log::debug!("Command: multisig sign | proposal_id={}", proposal_id);
                multisig::sign_proposal(
                    &cli.api_url,
                    &proposal_id,
                    &signer,
                    signature_data.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Execute { proposal_id } => {
                log::debug!("Command: multisig execute | proposal_id={}", proposal_id);
                multisig::execute_proposal(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::Info { proposal_id } => {
                log::debug!("Command: multisig info | proposal_id={}", proposal_id);
                multisig::proposal_info(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::ListProposals { status, limit } => {
                log::debug!(
                    "Command: multisig list-proposals | status={:?} limit={}",
                    status,
                    limit
                );
                multisig::list_proposals(&cli.api_url, status.as_deref(), limit).await?;
            }
        },
        Commands::Fuzz {
            contract_path,
            duration,
            timeout,
            threads,
            max_cases,
            output,
            minimize,
        } => {
            fuzz::run_fuzzer(
                &contract_path,
                &duration.to_string(),
                &timeout.to_string(),
                threads as usize,
                max_cases as u64,
                &output,
                minimize,
            )
            .await?;
        }
        Commands::Profile {
            contract_path,
            method,
            output,
            flamegraph,
            compare,
            recommendations,
        } => {
            commands::profile(
                &contract_path,
                method.as_deref(),
                output.as_deref(),
                flamegraph.as_deref(),
                compare.as_deref(),
                recommendations,
            )
            .await?;
        }
        Commands::Test {
            test_file,
            contract_path,
            junit,
            coverage,
            verbose,
        } => {
            commands::run_tests(
                &test_file,
                contract_path.as_deref(),
                junit.as_deref(),
                coverage,
                verbose,
            )
            .await?;
        }
        Commands::Sla { action } => match action {
            SlaCommands::Record {
                id,
                uptime,
                latency,
                error_rate,
            } => {
                log::debug!(
                    "Command: sla record | id={} uptime={} latency={} error_rate={}",
                    id,
                    uptime,
                    latency,
                    error_rate
                );
                commands::sla_record(&id, uptime, latency, error_rate)?;
            }
            SlaCommands::Status { id } => {
                log::debug!("Command: sla status | id={}", id);
                commands::sla_status(&id)?;
            }
        },
        Commands::Config { action } => match action {
            ConfigSubcommands::Get {
                contract_id,
                environment,
            } => {
                commands::config_get(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::Set {
                contract_id,
                environment,
                config_data,
                secrets_data,
                created_by,
            } => {
                commands::config_set(
                    &cli.api_url,
                    &contract_id,
                    &environment,
                    &config_data,
                    secrets_data.as_deref(),
                    &created_by,
                )
                .await?;
            }
            ConfigSubcommands::History {
                contract_id,
                environment,
            } => {
                commands::config_history(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::Rollback {
                contract_id,
                environment,
                version,
                created_by,
            } => {
                commands::config_rollback(
                    &cli.api_url,
                    &contract_id,
                    &environment,
                    version,
                    &created_by,
                )
                .await?;
            }
        },
        Commands::VerifyFormal {
            contract_path,
            properties,
            output,
            post,
        } => {
            formal_verification::run(&cli.api_url, &contract_path, &properties, &output, post)
                .await?;
        }
        Commands::ScanDeps {
            contract_id,
            dependencies,
            fail_on_high,
        } => {
            commands::scan_deps(&cli.api_url, &contract_id, &dependencies, fail_on_high).await?;
        }
        Commands::Coverage {
            contract_path,
            tests,
            threshold,
            output,
        } => {
            coverage::run(&contract_path, &tests, threshold, &output).await?;
        }
        Commands::Sign {
            package,
            private_key,
            contract_id,
            version,
            expires_at,
        } => {
            log::debug!(
                "Command: sign | package={} contract_id={} version={}",
                package,
                contract_id,
                version
            );
            package_signing::sign_package(
                &cli.api_url,
                &package,
                &private_key,
                &contract_id,
                &version,
                expires_at.as_deref(),
            )
            .await?;
        }
        Commands::Verify {
            package,
            contract_id,
            version,
            signature,
        } => {
            log::debug!(
                "Command: verify | package={} contract_id={}",
                package,
                contract_id
            );
            package_signing::verify_package(
                &cli.api_url,
                &package,
                &contract_id,
                version.as_deref(),
                signature.as_deref(),
            )
            .await?;
        }
        Commands::Keys { action } => match action {
            KeysCommands::Generate {} => {
                log::debug!("Command: keys generate");
                package_signing::generate_keypair()?;
            }
            KeysCommands::Revoke {
                signature_id,
                revoked_by,
                reason,
            } => {
                log::debug!("Command: keys revoke | signature_id={}", signature_id);
                package_signing::revoke_signature(
                    &cli.api_url,
                    &signature_id,
                    &revoked_by,
                    &reason,
                )
                .await?;
            }
            KeysCommands::Custody { contract_id } => {
                log::debug!("Command: keys custody | contract_id={}", contract_id);
                package_signing::get_chain_of_custody(&cli.api_url, &contract_id).await?;
            }
            KeysCommands::Log {
                contract_id,
                entry_type,
                limit,
            } => {
                log::debug!("Command: keys log");
                package_signing::get_transparency_log(
                    &cli.api_url,
                    contract_id.as_deref(),
                    entry_type.as_deref(),
                    limit,
                )
                .await?;
            }
        },
    }

    Ok(())
}
