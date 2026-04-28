#![allow(unused_variables)]

mod analyze;
mod analytics;
mod audit_command;
mod backup;
mod batch_register;
mod batch_ops;
mod batch_verify;
mod cicd;
mod codegen;
mod commands;
mod config;
mod contract_verify;
mod contracts;
mod conversions;
mod coverage;
mod dashboard;
mod events;
mod export;
mod formal_verification;
mod fuzz;
mod import;
mod incident;
mod io_utils;
mod manifest;
mod migration;
mod multisig;
mod net;
mod network;
mod package_signing;
mod patch;
mod profiler;
mod release_notes;
mod shell;
mod sla;
mod table_format;
mod test_framework;
mod track_deployment;
mod webhook;
mod wizard;
mod plugins;
mod deploy;
mod upgrade;
mod compare;
mod verification;
mod user_config;
mod version;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use colored::Colorize;
use patch::Severity;

/// Soroban Registry CLI — discover, publish, verify, and deploy Soroban contracts
#[derive(Debug, Parser)]
#[command(name = "soroban-registry", version, about, long_about = None)]
pub struct Cli {
    /// Registry API URL
    #[arg(long, global = true, default_value = "")]
    pub api_url: String,

    /// Stellar network to use (mainnet | testnet | futurenet)
    #[arg(long, global = true)]
    pub network: Option<String>,

    /// Global timeout for network/API operations (seconds)
    #[arg(long, global = true)]
    pub timeout: Option<u64>,

    /// Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv).
    #[arg(
        long,
        short = 'v',
        global = true,
        action = ArgAction::Count,
        long_help = "Enable verbose output. Repeat the flag to raise the log level:\n  \
                     (none)  warn   — errors and warnings only (default)\n  \
                     -v      info   — high-level operations\n  \
                     -vv     debug  — HTTP requests, responses, and timing\n  \
                     -vvv+   trace  — full internal tracing"
    )]
    pub verbose: u8,

    /// Check for CLI updates before running the command.
    #[arg(long, global = true)]
    pub check_updates: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Query contract analytics and statistics
    Analytics {
        /// Query type: top-contracts, trending, by-category, by-network
        query: String,
        /// Time period: 7d, 30d, 90d, or RFC3339 range start..end
        #[arg(long, default_value = "30d")]
        period: String,
        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
        /// Sort mode: value_desc, value_asc, key_asc, key_desc
        #[arg(long)]
        sort: Option<String>,
        /// Export output to a file
        #[arg(long)]
        export: Option<String>,
    },

    /// Get comprehensive registry statistics
    Stats {
        /// Timeframe: 7d, 30d, or all (default: all)
        #[arg(long, default_value = "all")]
        timeframe: String,
        /// Output format: table, json, yaml
        #[arg(long, default_value = "table")]
        format: String,
        /// Export to file
        #[arg(long)]
        output: Option<String>,
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

        /// Path to contract project directory for preflight testing
        #[arg(long, default_value = ".")]
        contract_path: String,

        /// Custom test command to run before submission
        #[arg(long)]
        test_command: Option<String>,

        /// Require coverage data and fail if unavailable
        #[arg(long)]
        require_coverage: bool,

        /// Minimum required coverage percentage (0-100)
        #[arg(long, default_value_t = 0.0)]
        coverage_threshold: f64,

        /// Skip pre-submission contract tests
        #[arg(long)]
        skip_tests: bool,
    },

    /// List contracts in the registry
    List {
        /// Max number of contracts to list
        #[arg(long, short, default_value = "20")]
        limit: usize,

        /// Number of contracts to skip
        #[arg(long, short, default_value = "0")]
        offset: usize,

        /// Filter by network (mainnet, testnet, futurenet)
        #[arg(long, short)]
        network: Option<crate::config::Network>,

        /// Filter by category
        #[arg(long, short)]
        category: Option<String>,

        /// Output format (table, json, csv)
        #[arg(long, short, default_value = "table")]
        format: String,
    },

    /// Show detailed info for a specific contract
    Info {
        /// Contract ID or slug
        id: String,
    },

    /// Compare multiple contracts
    Compare {
        /// Contract IDs to compare (2 to 4 contracts)
        #[arg(required = true, num_args = 2..=4)]
        ids: Vec<String>,
        
        /// Output detailed comparison as JSON
        #[arg(long)]
        json: bool,
        
        /// Export comparison report to a file (csv or json)
        #[arg(long)]
        export: Option<String>,
        
        /// Export format (csv or json). Derived from file extension if not provided.
        #[arg(long)]
        format: Option<String>,
    },

    /// Check CLI version and update availability
    Version {
        /// Check upstream for newer versions
        #[arg(long, default_value_t = true)]
        check_updates: bool,
        /// Print update instructions immediately when newer version exists
        #[arg(long, default_value_t = false)]
        auto_update: bool,
        /// Roll back to a previous version (manual install helper)
        #[arg(long)]
        rollback: Option<String>,
    },

    /// Launch an interactive, real-time terminal dashboard
    Dashboard {
        /// Minimum interval between UI renders (milliseconds)
        #[arg(long, default_value = "100")]
        refresh_rate: u64,
        /// Filter by contract category
        #[arg(long)]
        category: Option<String>,
        /// WebSocket URL (or set SOROBAN_REGISTRY_WS_URL)
        #[arg(long, env = "SOROBAN_REGISTRY_WS_URL")]
        ws_url: Option<String>,
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
    /// Analyze upgrades between two contract versions or schema files
    UpgradeAnalyze {
        /// Old contract version ID or local schema JSON file
        old: String,

        /// New contract version ID or local schema JSON file
        new: String,

        /// Output JSON
        #[arg(long)]
        json: bool,
    },

    /// Export contract registry data or a contract archive
    Export {
        /// Contract registry ID (UUID or on-chain address). Omit to export a filtered contract list.
        #[arg(long)]
        id: Option<String>,

        /// Output file path. Defaults to contracts-export.<format> or contract-export.tar.gz for archive.
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Path to contract source directory
        #[arg(long, default_value = ".")]
        contract_dir: String,

        /// Export format: json, csv, markdown, or archive
        #[arg(long, short = 'f')]
        format: Option<String>,

        /// Filter to apply to registry exports, e.g. --filter network=mainnet --filter verified_only=true
        #[arg(long = "filter")]
        filters: Vec<String>,

        /// Number of contracts to fetch per API page for list exports
        #[arg(long, default_value_t = 100)]
        page_size: usize,
    },

    /// Import contract data from a file (JSON, CSV, or Archive)
    Import {
        /// Path to the import file
        file: String,

        /// Format of the file (json | csv | archive). If omitted, inferred from extension.
        #[arg(long)]
        format: Option<String>,

        /// Directory to extract into (only for archive format)
        #[arg(long, default_value = "./imported")]
        output_dir: String,

        /// Validate the data before importing
        #[arg(long)]
        validate: bool,

        /// Perform a dry run without actually importing
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate documentation from a contract WASM
    Doc {
        /// Path to contract WASM file
        contract_path: String,

        /// Output directory
        #[arg(long, default_value = "docs")]
        output: String,
    },

    /// Generate OpenAPI 3.0 spec from contract ABI
    Openapi {
        /// Path to contract WASM file or ABI JSON file
        contract_path: String,

        /// Output file path
        #[arg(long, short = 'o', default_value = "openapi.yaml")]
        output: String,

        /// Output format: yaml, json, markdown, html
        #[arg(long, short = 'f', default_value = "yaml")]
        format: String,
    },

    /// Start an interactive contract deployment workflow
    Deploy {},

    /// Manage contract semantic versions
    #[command(name = "versions")]
    VersionSemver {
        #[command(subcommand)]
        action: VersionCommands,
    },

    /// Perform batch operations on multiple contracts
    Batch {
        /// Operation: tag, categorize, verify, deprecate
        operation: String,
        /// Contract IDs
        contracts: Vec<String>,
        /// Optional file containing contract IDs (one per line)
        #[arg(long)]
        file: Option<String>,
        /// Optional operation value (required for tag/categorize)
        #[arg(long)]
        value: Option<String>,
        /// Roll back already-applied operations when any item fails
        #[arg(long)]
        rollback_on_error: bool,
        /// Output JSON summary
        #[arg(long)]
        json: bool,
    },

    /// Manage contract upgrades and rollbacks
    Upgrade {
        #[command(subcommand)]
        action: UpgradeSubcommands,
    },

    /// Launch the interactive setup wizard
    Wizard {},

    /// Enter interactive REPL mode
    #[command(alias = "shell")]
    Repl {
        /// Initial network
        #[arg(long)]
        network: Option<String>,
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
        /// Optional path to scenario test file (YAML or JSON)
        ///
        /// If omitted, auto-detects and runs contract project tests.
        test_file: Option<String>,

        /// Path to contract directory or file
        #[arg(long)]
        contract_path: Option<String>,

        /// Custom test command (for auto-detected project tests mode)
        #[arg(long)]
        test_command: Option<String>,

        /// Output JUnit XML report
        #[arg(long)]
        junit: Option<String>,

        /// Show coverage report
        #[arg(long, default_value = "true")]
        coverage: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,

        /// Require coverage data and fail if unavailable
        #[arg(long)]
        require_coverage: bool,

        /// Minimum required coverage percentage (0-100)
        #[arg(long, default_value_t = 0.0)]
        coverage_threshold: f64,

        /// Optional shell command to run before executing tests
        #[arg(long)]
        setup_hook: Option<String>,

        /// Optional shell command to run after executing tests
        #[arg(long)]
        teardown_hook: Option<String>,

        /// Optional JSON or YAML file describing mock services used in the run
        #[arg(long)]
        mock_config: Option<String>,

        /// Optional JSON report output for the full test session
        #[arg(long)]
        report: Option<String>,

        /// Optional JSON profile output for load-test metadata
        #[arg(long)]
        profile_output: Option<String>,

        /// Number of iterations to simulate for load testing
        #[arg(long, default_value_t = 1)]
        load_iterations: u32,
    },

    /// Run a local contract security audit
    Audit {
        /// Path to contract file or project directory
        contract_path: String,

        /// Output format: text, json, markdown
        #[arg(long, default_value = "text")]
        format: String,

        /// Optional report output file
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Fail the command when findings at or above this severity are present
        #[arg(long)]
        fail_on: Option<String>,
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

    /// Inspect and modify contract state (dev/test mutation only)
    State {
        #[command(subcommand)]
        action: StateSubcommands,
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
    VerifyPackage {
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

    /// Verify a contract in the registry (check status, submit for audit, or show history)
    Verify {
        /// Contract UUID or on-chain address
        #[arg(required_unless_present_any = ["history", "check"])]
        id: Option<String>,

        /// Submit for verification (requires id or local project)
        #[arg(long, short = 's')]
        submit: bool,

        /// Check current verification status
        #[arg(long, short = 'c')]
        check: bool,

        /// Show verification history
        #[arg(long)]
        history: bool,

        /// Verification level: basic, intermediate, advanced
        #[arg(long, default_value = "basic")]
        level: String,

        /// Output results as JSON
        #[arg(long, short = 'j')]
        json: bool,

        /// Path to contract project directory (defaults to current dir)
        #[arg(long, default_value = ".")]
        path: String,

        /// Optional notes for submission
        #[arg(long)]
        notes: Option<String>,
    },

    /// Verify a contract binary against an Ed25519 signature locally
    VerifyContract {
        /// Path to the contract WASM/binary file
        wasm_path: String,

        /// Contract ID used when signing
        #[arg(long)]
        contract_id: String,

        /// Contract version used when signing
        #[arg(long)]
        version: String,

        /// Ed25519 signature (base64)
        #[arg(long)]
        signature: String,

        /// Ed25519 public key (base64)
        #[arg(long)]
        public_key: String,
    },

    /// Manage signing keys and signatures
    Keys {
        #[command(subcommand)]
        action: KeysCommands,
    },

    /// Contract deployment verification and security scan (#522)
    Contract {
        #[command(subcommand)]
        action: ContractCommands,
    },

    /// Verify multiple contracts in a single atomic batch (all succeed or all rollback)
    BatchVerify {
        /// Comma-separated list of contract IDs to verify.
        /// Optionally suffix with @version (e.g. abc123@1.0.0,def456)
        #[arg(long)]
        contracts: String,

        /// Stellar address or username initiating the batch (recorded in audit log)
        #[arg(long)]
        initiated_by: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage webhooks for contract lifecycle events
    Webhook {
        #[command(subcommand)]
        action: WebhookCommands,
    },

    /// Auto-generate and manage release notes for contract versions
    ReleaseNotes {
        #[command(subcommand)]
        action: ReleaseNotesCommands,
    },

    /// CI/CD pipeline integration and automation
    Cicd {
        #[command(subcommand)]
        action: CicdCommands,
    },

    /// Check the status of supported Stellar networks
    Network {
        #[command(subcommand)]
        action: NetworkCommands,
    },

    /// Register multiple contracts from a YAML or JSON manifest file
    BatchRegister {
        /// Path to the manifest file (.yaml, .yml, or .json)
        #[arg(long)]
        manifest: String,

        /// Publisher Stellar address (overrides `publisher` field in the manifest)
        #[arg(long)]
        publisher: Option<String>,

        /// Validate all entries and show what would be registered without submitting
        #[arg(long)]
        dry_run: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Run advanced analysis on a deployed contract (#530)
    Analyze {
        /// On-chain contract ID to analyse
        contract_id: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Report format: text (default), json, yaml
        #[arg(long, default_value = "text")]
        report_format: String,

        /// Write the report to a file instead of stdout
        #[arg(long, short = 'o')]
        output: Option<String>,
    },

    /// Track contract deployment status until confirmed or timeout (#524)
    TrackDeployment {
        /// On-chain contract ID
        #[arg(long)]
        contract_id: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Optional transaction hash to track (polls transaction endpoints first)
        #[arg(long)]
        tx_hash: Option<String>,

        /// Maximum wait time in seconds before exiting with code 2
        #[arg(long, default_value_t = 60)]
        wait_timeout: u64,

        /// Output machine-readable JSON status
        #[arg(long)]
        json: bool,
    },

    /// Plugin management (install, configure, run)
    Plugins {
        #[command(subcommand)]
        action: PluginCommands,
    },

    /// External command (may be provided by an installed plugin)
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// Sub-commands for the `network` group
#[derive(Debug, Subcommand)]
pub enum NetworkCommands {
    /// Show status of all supported Stellar networks
    Status {
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for the `release-notes` group
#[derive(Debug, Subcommand)]
pub enum ReleaseNotesCommands {
    /// Auto-generate release notes from code diff and changelog
    Generate {
        /// Contract registry ID (UUID or on-chain ID)
        #[arg(long)]
        contract_id: String,

        /// Version to generate notes for (semver, e.g. 1.2.0)
        #[arg(long)]
        version: String,

        /// Previous version to diff against (auto-detected if omitted)
        #[arg(long)]
        previous_version: Option<String>,

        /// Path to CHANGELOG.md file (auto-detected if present in cwd)
        #[arg(long)]
        changelog: Option<String>,

        /// On-chain contract address to include in notes
        #[arg(long)]
        contract_address: Option<String>,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// View generated release notes for a version
    View {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Version to view
        #[arg(long)]
        version: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Edit draft release notes before publishing
    Edit {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Version to edit
        #[arg(long)]
        version: String,

        /// Path to a file containing the new release notes text
        #[arg(long)]
        file: Option<String>,

        /// Inline text for the release notes
        #[arg(long)]
        text: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Publish (finalize) release notes
    Publish {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Version to publish
        #[arg(long)]
        version: String,

        /// Skip updating the contract_versions.release_notes column
        #[arg(long)]
        skip_version_update: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all release notes for a contract
    List {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for the `cicd` group
#[derive(Debug, Subcommand)]
pub enum CicdCommands {
    /// Run a full CI/CD pipeline (validate, scan, build, publish, verify)
    Run {
        /// Path to contract directory
        #[arg(long, default_value = ".")]
        contract_path: String,

        /// Network to target (testnet|mainnet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Skip security scans
        #[arg(long)]
        skip_scan: bool,

        /// Auto-register contract if not found in registry
        #[arg(long, default_value_t = true)]
        auto_register: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate the current environment for CI/CD integration
    Validate {
        /// Path to contract directory
        #[arg(long, default_value = ".")]
        contract_path: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommands {
    /// Get a user config value by key
    #[command(name = "get")]
    UserGet {
        key: String,
    },
    /// Set a user config value by key
    #[command(name = "set")]
    UserSet {
        key: String,
        value: String,
    },
    /// List all persisted user config values
    #[command(name = "list")]
    UserList {},
    /// Reset user config to defaults
    #[command(name = "reset")]
    UserReset {},

    /// Get contract environment configuration
    #[command(name = "contract-get")]
    ContractGet {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    /// Set contract environment configuration
    #[command(name = "contract-set")]
    ContractSet {
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
    /// Show contract config history
    #[command(name = "contract-history")]
    ContractHistory {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    /// Roll back contract config to a previous version
    #[command(name = "contract-rollback")]
    ContractRollback {
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

#[derive(Debug, Subcommand)]
pub enum StateSubcommands {
    /// Get a single state value by key
    Get {
        /// Contract identifier
        contract_id: String,
        /// State key
        key: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set a state key/value (testnet and futurenet only)
    Set {
        /// Contract identifier
        contract_id: String,
        /// State key
        key: String,
        /// New value (JSON is parsed, otherwise stored as string)
        value: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Dump full contract state
    Dump {
        /// Contract identifier
        contract_id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a state snapshot
    Snapshot {
        /// Contract identifier
        contract_id: String,
        /// Optional label for the snapshot
        #[arg(long)]
        label: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List saved state snapshots
    Snapshots {
        /// Contract identifier
        contract_id: String,
        /// Maximum number of snapshots to return
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Browse state change history
    History {
        /// Contract identifier
        contract_id: String,
        /// Filter by key
        #[arg(long)]
        key: Option<String>,
        /// Maximum number of entries to return
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for the `plugins` group
#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// List installed plugins and their commands
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Browse the registry marketplace
    Marketplace {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Install a plugin from the registry
    Install {
        /// Plugin name
        name: String,
        /// Optional version (defaults to marketplace version)
        #[arg(long)]
        version: Option<String>,
    },

    /// Uninstall an installed plugin
    Uninstall {
        /// Plugin name
        name: String,
        /// Optional version (defaults to removing all versions)
        #[arg(long)]
        version: Option<String>,
    },

    /// Run a plugin-provided command explicitly
    Run {
        /// The plugin command name
        command: String,
        /// Arguments passed to the plugin command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Enable/disable plugins and set per-plugin configuration
    Config {
        #[command(subcommand)]
        action: PluginConfigCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum PluginConfigCommands {
    /// Get the current JSON config for a plugin
    Get {
        /// Plugin name
        name: String,
    },

    /// Replace the plugin JSON config (must be a JSON object)
    Set {
        /// Plugin name
        name: String,
        /// JSON object
        #[arg(long)]
        json: String,
    },

    /// Disable a plugin (commands won't be discovered)
    Disable {
        /// Plugin name
        name: String,
    },

    /// Enable a plugin (default)
    Enable {
        /// Plugin name
        name: String,
    },
}

/// Sub-commands for the `contracts` group
#[derive(Debug, Subcommand)]
pub enum ContractsCommands {
    /// List contracts with filtering and pagination
    List {
        /// Filter by network (mainnet, testnet, futurenet)
        #[arg(long)]
        network: Option<String>,

        /// Filter by category (e.g., DEX, token, lending, oracle)
        #[arg(long)]
        category: Option<String>,

        /// Maximum number of contracts to return
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Number of contracts to skip (for pagination)
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Sort by field: name, created_at, health_score, network
        #[arg(long, default_value = "created_at")]
        sort_by: String,

        /// Sort order: asc or desc
        #[arg(long, default_value = "desc")]
        sort_order: String,

        /// Output format: table, json, or csv
        #[arg(long, default_value = "table")]
        format: String,

        /// Output results as JSON (shorthand for --format json)
        #[arg(long)]
        json: bool,

        /// Output results as CSV (shorthand for --format csv)
        #[arg(long)]
        csv: bool,
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

/// Sub-commands for the `contract` group (#522)
#[derive(Debug, Subcommand)]
pub enum ContractCommands {
    /// Verify a deployed contract's authenticity against the on-chain registry
    ///
    /// Usage: soroban-registry contract verify <address> --network <network> [--json]
    Verify {
        /// On-chain contract address to verify
        address: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    
    /// Display detailed information about a contract
    ///
    /// Usage: soroban-registry contract details <address> --network <network> [--json]
    Details {
        /// On-chain contract address to inspect
        address: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for the `webhook` group
#[derive(Debug, Subcommand)]
pub enum WebhookCommands {
    /// Register a new webhook subscription
    Create {
        /// Endpoint URL to receive events (must be HTTPS in production)
        #[arg(long)]
        url: String,

        /// Comma-separated list of events to subscribe to.
        /// Valid: contract.published, contract.verified,
        ///        contract.failed_verification, version.created
        #[arg(long)]
        events: String,

        /// Optional HMAC-SHA256 secret key (auto-generated if omitted)
        #[arg(long)]
        secret: Option<String>,
    },

    /// List all registered webhooks
    List {},

    /// Delete a webhook by ID
    Delete {
        /// Webhook ID to delete
        webhook_id: String,
    },

    /// Send a test event to a webhook
    Test {
        /// Webhook ID to test
        webhook_id: String,
    },

    /// View delivery logs for a webhook
    Logs {
        /// Webhook ID
        webhook_id: String,

        /// Maximum number of log entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Manually retry a dead-letter delivery
    Retry {
        /// Delivery ID to retry
        delivery_id: String,
    },

    /// Verify a webhook payload signature locally
    VerifySig {
        /// HMAC secret key used for signing
        #[arg(long)]
        secret: String,

        /// Raw JSON payload body
        #[arg(long)]
        payload: String,

        /// Signature header value (e.g. sha256=abc123...)
        #[arg(long)]
        signature: String,
    },
}

/// Sub-commands for the `migrate` group
#[derive(Debug, Subcommand)]
pub enum MigrateCommands {
    /// Preview migration outcome (dry-run)
    Preview { old_id: String, new_id: String },
    /// Analyze schema differences between versions
    Analyze { old_id: String, new_id: String },
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
    Validate { old_id: String, new_id: String },
    /// Apply migration and record history
    Apply { old_id: String, new_id: String },
    /// Rollback a migration by migration ID
    Rollback { migration_id: String },
    /// Show migration history
    History {
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum VersionCommands {
    /// List versions for a contract
    List {
        /// Contract identifier
        contract_id: String,
    },
    /// Bump the semantic version
    Bump {
        /// Current version
        current: String,
        /// Bump level: major, minor, or patch
        #[arg(long, default_value = "patch")]
        level: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum UpgradeSubcommands {
    /// Analyze compatibility between two contract versions
    Analyze {
        /// Path to old WASM
        old_wasm: String,
        /// Path to new WASM
        new_wasm: String,
    },
    /// Apply an upgrade to a deployed contract
    Apply {
        /// Contract identifier
        contract_id: String,
        /// Path to new WASM
        new_wasm: String,
    },
    /// Rollback a contract to a previous version
    Rollback {
        /// Contract identifier
        contract_id: String,
        /// Version to rollback to
        version: String,
    },
    /// Generate a migration script template between versions
    Generate {
        /// Old contract identifier
        old_id: String,
        /// New contract identifier
        new_id: String,
        /// Language (rust or js)
        #[arg(long, default_value = "rust")]
        language: String,
        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    if cli.check_updates {
        let update_checks_enabled = user_config::load()
            .map(|cfg| cfg.update_checks_enabled)
            .unwrap_or(true);
        if update_checks_enabled {
            let _ = version::check_version(true, false, None).await;
        }
    }

    let cli_api_base = if cli.api_url.trim().is_empty() {
        None
    } else {
        Some(cli.api_url.clone())
    };
    let runtime = config::resolve_runtime_config(cli.network.clone(), cli_api_base, cli.timeout)?;
    cli.api_url = runtime.api_base;
    cli.network = Some(runtime.network.to_string());
    cli.timeout = Some(runtime.timeout);

    // ── Initialise logger ─────────────────────────────────────────────────────
    // -v counts; each level raises verbosity by one step.
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::Builder::new()
        .parse_filters(log_level)
        .format_timestamp(None) // no timestamps in CLI output
        .format_module_path(cli.verbose > 0) // show module path only when verbose
        .init();

    log::debug!("Verbose mode enabled");
    log::debug!("API URL: {}", cli.api_url);

    handle_command(cli).await
}

pub async fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Repl { network: shell_network } => shell::run(&cli.api_url, shell_network).await,
        _ => {
            // ── Resolve network ───────────────────────────────────────────────────────
            let cfg_network = config::resolve_network(cli.network.clone())?;
            let mut net_str = cfg_network.to_string();
            if net_str == "auto" {
                net_str = "mainnet".to_string();
            }
            let network: commands::Network = net_str.parse().unwrap();

            dispatch_command(cli, network, cfg_network).await
        }
    }
}

pub async fn dispatch_command(
    cli: Cli,
    network: commands::Network,
    cfg_network: crate::config::Network,
) -> Result<()> {
    log::debug!("Network: {:?}", network);

    match cli.command {
        Commands::Repl { .. } => {
            // Already handled at top level, but for completeness or nested calls:
            // We could call shell::run here again but to break recursion we don't.
            println!("{}", "Warning: REPL already running".yellow());
            return Ok(());
        }
        Commands::TrackDeployment {
            contract_id,
            network,
            tx_hash,
            wait_timeout,
            json,
        } => {
            log::debug!(
                "Command: track-deployment | contract_id={} network={} tx_hash={:?} wait_timeout={} json={}",
                contract_id, network, tx_hash, wait_timeout, json
            );
            track_deployment::run(
                &cli.api_url,
                &contract_id,
                &network,
                tx_hash.as_deref(),
                wait_timeout,
                json,
            ).await?;
        }
        Commands::Plugins { action } => match action {
            PluginCommands::List { json } => {
                let installed = plugins::discover_installed()?;
                if json {
                    let out: Vec<serde_json::Value> = installed
                        .into_iter()
                        .map(|p| {
                            serde_json::json!({
                                "manifest": p.manifest,
                                "path": p.manifest_path.to_string_lossy().to_string()
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "plugins": out }))?);
                } else {
                    if installed.is_empty() {
                        println!("{}", "No plugins installed.".yellow());
                    } else {
                        println!("\n{}", "Installed Plugins:".bold().cyan());
                        println!("{}", "=".repeat(80).cyan());
                        for p in installed {
                            let desc = p.manifest.description.clone().unwrap_or_default();
                            println!(
                                "  {}@{}  {}",
                                p.manifest.name.bold(),
                                p.manifest.version.bright_blue(),
                                desc.bright_black()
                            );
                            for cmd in &p.manifest.commands {
                                println!(
                                    "    - {}  {}",
                                    cmd.name.bright_green(),
                                    cmd.description.clone().unwrap_or_default().bright_black()
                                );
                            }
                        }
                    }
                }
            }
            PluginCommands::Marketplace { json } => {
                let marketplace = plugins::fetch_marketplace(&cli.api_url).await?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&marketplace)?);
                } else {
                    if marketplace.plugins.is_empty() {
                        println!("{}", "Marketplace returned no plugins.".yellow());
                    } else {
                        println!("\n{}", "Plugin Marketplace:".bold().cyan());
                        println!("{}", "=".repeat(80).cyan());
                        for p in marketplace.plugins {
                            println!(
                                "  {}@{}  {}",
                                p.name.bold(),
                                p.version.bright_blue(),
                                p.description.unwrap_or_default().bright_black()
                            );
                            for cmd in p.commands {
                                println!(
                                    "    - {}  {}",
                                    cmd.name.bright_green(),
                                    cmd.description.unwrap_or_default().bright_black()
                                );
                            }
                        }
                    }
                }
            }
            PluginCommands::Install { name, version } => {
                plugins::install_from_registry(&cli.api_url, &name, version.as_deref()).await?;
            }
            PluginCommands::Uninstall { name, version } => {
                plugins::uninstall(&name, version.as_deref())?;
            }
            PluginCommands::Run { command, args } => {
                let result =
                    plugins::run_installed_command(&cli.api_url, &network.to_string(), &command, args)
                        .await?;
                print!("{}", result.stdout);
            }
            PluginCommands::Config { action } => match action {
                PluginConfigCommands::Get { name } => {
                    let cfg = plugins::get_plugin_config(&name)?;
                    println!("{}", serde_json::to_string_pretty(&cfg)?);
                }
                PluginConfigCommands::Set { name, json } => {
                    plugins::set_plugin_config_json(&name, &json)?;
                    println!("{} Updated config for {}", "✓".green(), name.bold());
                }
                PluginConfigCommands::Disable { name } => {
                    plugins::set_plugin_enabled(&name, false)?;
                    println!("{} Disabled {}", "✓".green(), name.bold());
                }
                PluginConfigCommands::Enable { name } => {
                    plugins::set_plugin_enabled(&name, true)?;
                    println!("{} Enabled {}", "✓".green(), name.bold());
                }
            },
        },
        Commands::External(args) => {
            if args.is_empty() {
                anyhow::bail!("No external command provided");
            }
            let cmd = args[0].clone();
            let rest = args.into_iter().skip(1).collect::<Vec<_>>();
            let result = plugins::run_installed_command(&cli.api_url, &network.to_string(), &cmd, rest).await?;
            print!("{}", result.stdout);
        }
        Commands::Search {
            query,
            verified_only,
            network: filter_networks,
            category,
            sort,
            limit,
            offset,
            json,
        } => {
            let networks_vec: Vec<String> = filter_networks
                .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: search | query={:?} verified_only={} networks={:?} category={:?} sort={:?}",
                query,
                verified_only,
                networks_vec,
                category,
                sort
            );
            commands::search(
                &cli.api_url,
                &query,
                network,
                verified_only,
                networks_vec,
                category.as_deref(),
                sort.as_deref(),
                limit,
                offset,
                json,
            )
            .await?;
        }
        Commands::Info { id } => {
            commands::contract_info(&cli.api_url, &id).await?;
        }
        Commands::Compare { ids, json, export, format } => {
            compare::run(&cli.api_url, ids, json, export.as_deref(), format.as_deref()).await?;
        }
        Commands::Analytics {
            query,
            period,
            format,
            sort,
            export,
        } => {
            let parsed_query = analytics::AnalyticsQuery::parse(&query)?;
            analytics::run(
                &cli.api_url,
                parsed_query,
                &period,
                &format,
                sort.as_deref(),
                export.as_deref(),
            )
            .await?;
        }
        Commands::Version {
            check_updates,
            auto_update,
            rollback,
        } => {
            version::check_version(check_updates, auto_update, rollback).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            network: _publish_network,
            category,
            tags,
            publisher,
            contract_path,
            test_command,
            require_coverage,
            coverage_threshold,
            skip_tests,
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
                false,
                &contract_path,
                test_command.as_deref(),
                require_coverage,
                coverage_threshold,
                skip_tests,
            )
            .await?;
        }
        Commands::List {
            limit,
            offset,
            network,
            category,
            format,
        } => {
            commands::contract_list(
                &cli.api_url,
                limit,
                offset,
                network.or(Some(cfg_network)),
                category,
                &format,
            )
            .await?;
        }
        Commands::Dashboard {
            refresh_rate,
            category,
            ws_url,
        } => {
            log::debug!(
                "Command: dashboard | refresh_rate={} network={:?} category={:?}",
                refresh_rate,
                cli.network,
                category
            );
            dashboard::run_dashboard(dashboard::DashboardParams {
                refresh_rate_ms: refresh_rate,
                network: cli.network.clone(),
                category,
                ws_url,
            })
            .await?;
        }
        Commands::BreakingChanges {
            old_id,
            new_id,
            json,
        } => {
            log::debug!("Command: breaking-changes | old={} new={}", old_id, new_id);
            commands::breaking_changes(&cli.api_url, &old_id, &new_id, json).await?;
        }
        Commands::UpgradeAnalyze { old, new, json } => {
            log::debug!("Command: upgrade analyze | old={} new={}", old, new);
            commands::upgrade_analyze(&cli.api_url, &old, &new, json).await?;
        }
        Commands::Migrate { action } => match action {
            MigrateCommands::Preview { old_id, new_id } => {
                log::debug!(
                    "Command: migrate preview | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                migration::preview(&old_id, &new_id)?;
            }
            MigrateCommands::Analyze { old_id, new_id } => {
                log::debug!(
                    "Command: migrate analyze | old_id={} new_id={}",
                    old_id,
                    new_id
                );
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
                log::debug!(
                    "Command: migrate validate | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                migration::validate(&old_id, &new_id)?;
            }
            MigrateCommands::Apply { old_id, new_id } => {
                log::debug!(
                    "Command: migrate apply | old_id={} new_id={}",
                    old_id,
                    new_id
                );
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
            format,
            filters,
            page_size,
        } => {
            log::debug!(
                "Command: export | id={:?} output={:?} format={:?}",
                id,
                output,
                format
            );
            commands::export(
                &cli.api_url,
                id.as_deref(),
                output.as_deref(),
                &contract_dir,
                format.as_deref(),
                filters,
                page_size,
            )
            .await?;
        }
        Commands::Import {
            file,
            format,
            output_dir,
            validate,
            dry_run,
        } => {
            let network = cli.network.as_deref();
            log::debug!(
                "Command: import | file={} format={:?} output_dir={} validate={} dry_run={}",
                file, format, output_dir, validate, dry_run
            );
            crate::import::run(&cli.api_url, &file, format.as_deref(), network, &output_dir, validate, dry_run).await?;
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
        Commands::Openapi {
            contract_path,
            output,
            format,
        } => {
            log::debug!(
                "Command: openapi | contract_path={} output={} format={}",
                contract_path,
                output,
                format
            );
            commands::openapi(&contract_path, &output, &format)?;
        }
        Commands::Deploy {} => {
            log::debug!("Command: deploy");
            deploy::run_interactive().await?;
        }
        Commands::VersionSemver { action } => match action {
            VersionCommands::List { contract_id } => {
                log::debug!("Command: version list | contract_id={}", contract_id);
                upgrade::version::list(&contract_id)?;
            }
            VersionCommands::Bump { current, level } => {
                log::debug!("Command: version bump | current={} level={}", current, level);
                let next = upgrade::version::bump(&current, &level)?;
                println!("Next version: {}", next.green().bold());
            }
        },
        Commands::Upgrade { action } => match action {
            UpgradeSubcommands::Analyze { old_wasm, new_wasm } => {
                log::debug!("Command: upgrade analyze | old={} new={}", old_wasm, new_wasm);
                upgrade::manager::analyze(&old_wasm, &new_wasm).await?;
            }
            UpgradeSubcommands::Apply { contract_id, new_wasm } => {
                log::debug!("Command: upgrade apply | contract_id={} new={}", contract_id, new_wasm);
                upgrade::manager::apply(&contract_id, &new_wasm).await?;
            }
            UpgradeSubcommands::Rollback { contract_id, version } => {
                log::debug!("Command: upgrade rollback | contract_id={} version={}", contract_id, version);
                upgrade::manager::rollback(&contract_id, &version).await?;
            }
            UpgradeSubcommands::Generate { old_id, new_id, language, output } => {
                log::debug!("Command: upgrade generate | old={} new={} lang={}", old_id, new_id, language);
                crate::migration::generate_template(&old_id, &new_id, &language, output.as_deref())?;
            }
        },
        Commands::Wizard {} => {
            log::debug!("Command: wizard");
            wizard::run(&cli.api_url).await?;
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
            log::debug!(
                "Command: profile | contract_path={} method={:?} output={:?} flamegraph={:?} compare={:?} recommendations={}",
                contract_path,
                method,
                output,
                flamegraph,
                compare,
                recommendations
            );
            commands::profile(
                &contract_path,
                method.as_deref(),
                output.as_deref(),
                flamegraph.as_deref(),
                compare.as_deref(),
                recommendations,
            )?;
        }
        Commands::Test {
            test_file,
            contract_path,
            test_command,
            junit,
            coverage,
            verbose,
            require_coverage,
            coverage_threshold,
            setup_hook,
            teardown_hook,
            mock_config,
            report,
            profile_output,
            load_iterations,
        } => {
            commands::run_test_suite(commands::TestSuiteOptions {
                test_file: test_file.as_deref(),
                contract_path: contract_path.as_deref().unwrap_or("."),
                test_command: test_command.as_deref(),
                junit_output: junit.as_deref(),
                show_coverage: coverage,
                verbose,
                require_coverage,
                coverage_threshold,
                setup_hook: setup_hook.as_deref(),
                teardown_hook: teardown_hook.as_deref(),
                mock_config: mock_config.as_deref(),
                report_output: report.as_deref(),
                profile_output: profile_output.as_deref(),
                load_iterations,
            })
            .await?;
        }
        Commands::Audit {
            contract_path,
            format,
            output,
            fail_on,
        } => {
            log::debug!(
                "Command: audit | contract_path={} format={} output={:?} fail_on={:?}",
                contract_path,
                format,
                output,
                fail_on
            );
            audit_command::run(
                &contract_path,
                &format,
                output.as_deref(),
                fail_on.as_deref(),
            )?;
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
            ConfigSubcommands::UserGet { key } => {
                user_config::validate_key(&key)?;
                let value = user_config::get_key(&key)?;
                match value {
                    Some(v) => println!("{}", v),
                    None => anyhow::bail!("Key '{}' was not found in user config.", key),
                }
            }
            ConfigSubcommands::UserSet { key, value } => {
                user_config::set_key(&key, &value)?;
                println!("Updated '{}' in user config.", key);
            }
            ConfigSubcommands::UserList {} => {
                let cfg = user_config::list()?;
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            }
            ConfigSubcommands::UserReset {} => {
                let cfg = user_config::reset_to_defaults()?;
                println!("User config reset to defaults:");
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            }
            ConfigSubcommands::ContractGet {
                contract_id,
                environment,
            } => {
                commands::config_get(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::ContractSet {
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
            ConfigSubcommands::ContractHistory {
                contract_id,
                environment,
            } => {
                commands::config_history(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::ContractRollback {
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
        Commands::State { action } => match action {
            StateSubcommands::Get {
                contract_id,
                key,
                json,
            } => {
                commands::state_get(&cli.api_url, &contract_id, &key, network, json).await?;
            }
            StateSubcommands::Set {
                contract_id,
                key,
                value,
                json,
            } => {
                commands::state_set(&cli.api_url, &contract_id, &key, &value, network, json)
                    .await?;
            }
            StateSubcommands::Dump { contract_id, json } => {
                commands::state_dump(&contract_id, network, json)?;
            }
            StateSubcommands::Snapshot {
                contract_id,
                label,
                json,
            } => {
                commands::state_snapshot_create(&contract_id, network, label.as_deref(), json)?;
            }
            StateSubcommands::Snapshots {
                contract_id,
                limit,
                json,
            } => {
                commands::state_snapshot_list(&contract_id, network, limit, json)?;
            }
            StateSubcommands::History {
                contract_id,
                key,
                limit,
                json,
            } => {
                commands::state_history(&contract_id, network, key.as_deref(), limit, json)?;
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
        Commands::VerifyPackage {
            package,
            contract_id,
            version,
            signature,
        } => {
            log::debug!(
                "Command: verify-package | package={} contract_id={}",
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
        Commands::Verify {
            id,
            submit,
            check,
            history,
            level,
            json,
            path,
            notes,
        } => {
            log::debug!("Command: verify | id={:?} submit={} check={}", id, submit, check);
            verification::run(&cli.api_url, id, submit, check, history, level, json, &path, notes).await?;
        }
        Commands::VerifyContract {
            wasm_path,
            contract_id,
            version,
            signature,
            public_key,
        } => {
            log::debug!(
                "Command: verify-contract | wasm_path={} contract_id={} version={}",
                wasm_path,
                contract_id,
                version
            );
            package_signing::verify_contract_local(
                &wasm_path,
                &contract_id,
                &version,
                &signature,
                &public_key,
            )?;
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
        Commands::BatchVerify {
            contracts,
            initiated_by,
            json,
        } => {
            log::debug!(
                "Command: batch-verify | contracts={} initiated_by={}",
                contracts,
                initiated_by
            );
            batch_verify::run_batch_verify(&cli.api_url, &contracts, &initiated_by, json).await?;
        }
        Commands::Webhook { action } => match action {
            WebhookCommands::Create {
                url,
                events,
                secret,
            } => {
                let event_list: Vec<String> =
                    events.split(',').map(|s| s.trim().to_string()).collect();
                log::debug!(
                    "Command: webhook create | url={} events={:?}",
                    url,
                    event_list
                );
                webhook::create_webhook(&cli.api_url, &url, event_list, secret.as_deref()).await?;
            }
            WebhookCommands::List {} => {
                log::debug!("Command: webhook list");
                webhook::list_webhooks(&cli.api_url).await?;
            }
            WebhookCommands::Delete { webhook_id } => {
                log::debug!("Command: webhook delete | id={}", webhook_id);
                webhook::delete_webhook(&cli.api_url, &webhook_id).await?;
            }
            WebhookCommands::Test { webhook_id } => {
                log::debug!("Command: webhook test | id={}", webhook_id);
                webhook::test_webhook(&cli.api_url, &webhook_id).await?;
            }
            WebhookCommands::Logs { webhook_id, limit } => {
                log::debug!("Command: webhook logs | id={} limit={}", webhook_id, limit);
                webhook::webhook_logs(&cli.api_url, &webhook_id, limit).await?;
            }
            WebhookCommands::Retry { delivery_id } => {
                log::debug!("Command: webhook retry | delivery_id={}", delivery_id);
                webhook::retry_delivery(&cli.api_url, &delivery_id).await?;
            }
            WebhookCommands::VerifySig {
                secret,
                payload,
                signature,
            } => {
                log::debug!("Command: webhook verify-sig");
                webhook::verify_signature_cmd(&secret, &payload, &signature)?;
            }
        },
        // ── Contract verify command (#522) ───────────────────────────────────
        Commands::Contract { action } => match action {
            ContractCommands::Verify {
                address,
                network,
                json,
            } => {
                log::debug!(
                    "Command: contract verify | address={} network={} json={}",
                    address,
                    network,
                    json
                );
                contract_verify::run(&cli.api_url, &address, &network, json).await?;
            }
            ContractCommands::Details {
                address,
                network,
                json,
            } => {
                log::debug!(
                    "Command: contract details | address={} network={} json={}",
                    address,
                    network,
                    json
                );
                contracts::run_details(&cli.api_url, &address, &network, json).await?;
            }
        },
        // ── Release Notes commands ───────────────────────────────────────────
        Commands::ReleaseNotes { action } => match action {
            ReleaseNotesCommands::Generate {
                contract_id,
                version,
                previous_version,
                changelog,
                contract_address,
                json,
            } => {
                log::debug!(
                    "Command: release-notes generate | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::generate(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    previous_version.as_deref(),
                    changelog.as_deref(),
                    contract_address.as_deref(),
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::View {
                contract_id,
                version,
                json,
            } => {
                log::debug!(
                    "Command: release-notes view | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::view(&cli.api_url, &contract_id, &version, json).await?;
            }
            ReleaseNotesCommands::Edit {
                contract_id,
                version,
                file,
                text,
                json,
            } => {
                log::debug!(
                    "Command: release-notes edit | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::edit(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    file.as_deref(),
                    text.as_deref(),
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::Publish {
                contract_id,
                version,
                skip_version_update,
                json,
            } => {
                log::debug!(
                    "Command: release-notes publish | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::publish(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    skip_version_update,
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::List { contract_id, json } => {
                log::debug!("Command: release-notes list | contract_id={}", contract_id);
                release_notes::list(&cli.api_url, &contract_id, json).await?;
            }
        },

        Commands::Cicd { action } => match action {
            CicdCommands::Run {
                contract_path,
                network,
                skip_scan,
                auto_register,
                json,
            } => {
                log::debug!(
                    "Command: cicd run | path={} network={}",
                    contract_path,
                    network
                );
                cicd::run_pipeline(
                    &cli.api_url,
                    &contract_path,
                    &network,
                    skip_scan,
                    auto_register,
                    json,
                )
                .await?;
            }
            CicdCommands::Validate { contract_path } => {
                log::debug!("Command: cicd validate | path={}", contract_path);
                cicd::validate_env(&contract_path).await?;
            }
        },

        // ── Network commands (issue #523) ────────────────────────────────────
        Commands::Network { action } => match action {
            NetworkCommands::Status { json } => {
                log::debug!("Command: network status");
                network::status(json).await?;
            }
        },

        // ── Advanced contract analysis (issue #530) ─────────────────────────
        Commands::Analyze {
            contract_id,
            network: net_str,
            report_format,
            output,
        } => {
            log::debug!(
                "Command: analyze | contract_id={} network={} format={}",
                contract_id,
                net_str,
                report_format
            );
            analyze::run(
                &cli.api_url,
                &contract_id,
                &net_str,
                &report_format,
                output.as_deref(),
            )
            .await?;
        }

        // ── Bulk contract registration (issue #525) ──────────────────────────
        Commands::BatchRegister {
            manifest,
            publisher,
            dry_run,
            json,
        } => {
            log::debug!(
                "Command: batch-register | manifest={} dry_run={} publisher={:?}",
                manifest,
                dry_run,
                publisher
            );
            batch_register::run_batch_register(
                &cli.api_url,
                &manifest,
                publisher.as_deref(),
                dry_run,
                json,
            )
            .await?;
        }
        Commands::Batch {
            operation,
            contracts,
            file,
            value,
            rollback_on_error,
            json,
        } => {
            let op = batch_ops::BatchOperation::parse(&operation)?;
            batch_ops::run(
                &cli.api_url,
                op,
                contracts,
                file.as_deref(),
                value.as_deref(),
                rollback_on_error,
                json,
            )
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod verbose_flag_tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("CLI should parse")
    }

    #[test]
    fn no_flag_yields_zero() {
        let cli = parse(&["soroban-registry", "version"]);
        assert_eq!(cli.verbose, 0);
    }

    #[test]
    fn single_short_flag_yields_one() {
        let cli = parse(&["soroban-registry", "-v", "version"]);
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn repeated_short_flags_count() {
        let cli = parse(&["soroban-registry", "-v", "-v", "-v", "version"]);
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn stacked_short_flag_counts() {
        let cli = parse(&["soroban-registry", "-vvv", "version"]);
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn long_flag_counts_too() {
        let cli = parse(&["soroban-registry", "--verbose", "--verbose", "version"]);
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn verbose_works_after_subcommand_when_global() {
        let cli = parse(&["soroban-registry", "version", "-vv"]);
        assert_eq!(cli.verbose, 2);
    }
}
