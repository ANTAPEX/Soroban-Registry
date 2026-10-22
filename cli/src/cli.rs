//! The command-line surface: every `clap` type the binary parses into.
//!
//! Splitting this out of `main.rs` keeps the definition of what the CLI accepts
//! separate from what it then does, which lives in `dispatch`.

use clap::{ArgAction, Parser, Subcommand, ValueEnum};

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

    /// Registry configuration profile to use
    #[arg(long, global = true)]
    pub profile: Option<String>,

    /// Skip local response cache and always fetch fresh data
    #[arg(long, global = true)]
    pub no_cache: bool,

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
        /// Output format: table, json, csv, yaml
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

        /// Filter by network (comma-separated: mainnet,testnet,futurenet). The
        /// field is `networks`, not `network`: clap derives an arg id from the
        /// field name, and the global `--network` (global = true) shares that
        /// id, so a subcommand-local `network` field would collide with it and
        /// panic trying to downcast the matched value.
        #[arg(long)]
        networks: Option<String>,

        /// Filter by category (comma-separated for multiple: DeFi,NFT)
        #[arg(long, short)]
        category: Option<String>,

        /// Output format (table, json, csv, yaml)
        #[arg(long, short, default_value = "table")]
        format: String,
    },

    /// Show detailed info for a specific contract
    Info {
        /// Contract ID or slug
        id: String,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        raw: bool,
    },

    /// Search for contracts in the registry
    Search {
        /// Search query
        query: String,

        /// Only show verified contracts
        #[arg(long)]
        verified_only: bool,

        /// Filter by network (comma-separated: mainnet,testnet,futurenet).
        /// The field is `networks`, not `network`: clap derives an arg id from
        /// the field name, and the global `--network` (global = true) shares
        /// that id, so clap would populate both from either flag.
        #[arg(long)]
        networks: Option<String>,

        /// Filter by category (comma-separated: DeFi,NFT)
        #[arg(long)]
        category: Option<String>,

        /// Sort by (name, created, updated, relevance)
        #[arg(long)]
        sort: Option<String>,

        /// Maximum results to return
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Results offset
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

        /// Exit with code 1 when differences are found (0 = identical, 2 = error)
        #[arg(long)]
        exit_code: bool,

        /// Diff output format: none, unified, side-by-side
        #[arg(long, default_value = "none")]
        diff: String,

        /// Limit compared field groups (metadata,verification,deployment,abi,all)
        #[arg(long, value_delimiter = ',')]
        fields: Option<Vec<String>>,
    },

    /// Generate shell completion scripts (#971)
    Completion {
        /// Target shell
        #[arg(value_enum)]
        shell: crate::commands::completion::CompletionShell,
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
        /// Recipients file/filter for `batch notify`
        #[arg(long)]
        recipients: Option<String>,
        /// Message type for `batch notify`
        #[arg(long, default_value = "info")]
        message_type: String,
        /// Template file or inline template for `batch notify`
        #[arg(long)]
        template: Option<String>,
        /// Preview notification/migration without sending/writing
        #[arg(long)]
        preview: bool,
        /// RFC3339 schedule for `batch notify`
        #[arg(long)]
        schedule: Option<String>,
        /// Channels for `batch notify`: email,in-app,webhook
        #[arg(long, value_delimiter = ',')]
        channels: Vec<String>,
        /// Filter expression for `batch migrate`
        #[arg(long)]
        filter: Option<String>,
        /// Use atomic/fail-safe migration semantics
        #[arg(long)]
        atomic: bool,
        /// Migration report output path
        #[arg(long)]
        report: Option<String>,

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

    /// Perf contract execution performance
    #[command(name = "perf")]
    Perf {
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

    /// Manage your user profile and publishing preferences (#841)
    Profile {
        #[command(subcommand)]
        action: ProfileCommands,
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

    /// Read and edit persisted user configuration values
    Config {
        #[command(subcommand)]
        action: ConfigSubcommands,
    },

    /// Manage authentication sessions and API tokens
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },

    /// Manage contract backups and disaster recovery
    Backup {
        #[command(subcommand)]
        action: BackupCommands,
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

    /// Scan a contract's dependencies for known vulnerabilities
    ScanDeps {
        /// Contract address or registry UUID to scan
        #[arg(long)]
        contract_id: String,
        /// Comma-separated dependency list to scan
        #[arg(long, default_value = ",")]
        dependencies: String,
        /// Exit non-zero when a high-severity finding is reported
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

    /// Manage API keys for programmatic access (#842)
    #[command(name = "api-key")]
    ApiKey {
        #[command(subcommand)]
        action: ApiKeyCommands,
    },

    /// Verify multiple contracts in a bulk batch (#850)
    BatchVerify {
        /// Path to a contract list file (.txt one-ID-per-line, .json, or .yaml)
        #[arg(long)]
        file: Option<String>,

        /// Comma-separated IDs — fallback when --file is absent
        #[arg(long)]
        contracts: Option<String>,

        /// Filter by network when discovering from API (mainnet|testnet|futurenet)
        #[arg(long)]
        network: Option<String>,

        /// Filter by category when discovering from API (e.g. defi, nft)
        #[arg(long)]
        category: Option<String>,

        /// Only include contracts created within this many days
        #[arg(long)]
        age: Option<u32>,

        /// Stellar address or username initiating the batch
        #[arg(long)]
        initiated_by: String,

        /// Verification depth: basic | standard | strict
        #[arg(long, default_value = "standard")]
        level: String,

        /// Export report to file; format inferred from extension (.json or .csv)
        #[arg(long)]
        export: Option<String>,

        /// Save human-readable report to a text file
        #[arg(long)]
        output: Option<String>,

        /// Save cron schedule and print crontab entry
        #[arg(long)]
        schedule: Option<String>,

        /// Output machine-readable JSON
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

    /// Audit multiple contracts in batch for security and best practices
    BatchAudit {
        /// File containing contract paths (one per line) or comma-separated paths
        file: String,
        /// Report format: text, json, markdown
        #[arg(long, default_value = "text")]
        format: String,
        /// Output directory for generated reports
        #[arg(long)]
        output_dir: Option<String>,
        /// Fail on findings at or above this severity
        #[arg(long)]
        fail_on: Option<String>,
        /// Show only high and critical findings
        #[arg(long)]
        high_risk: bool,
        /// Audit profile: basic, standard, comprehensive
        #[arg(long, default_value = "standard")]
        profile: String,
        /// Export audit findings to a file
        #[arg(long)]
        export: Option<String>,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Deploy a contract WASM to multiple networks
    BatchDeploy {
        /// Path to the WASM file
        wasm_file: String,
        /// Comma-separated target networks (mainnet,testnet,futurenet)
        #[arg(long, default_value = "testnet")]
        networks: String,
        /// Signer Stellar address or secret
        #[arg(long)]
        signer: String,
        /// Stop and report failure if any deployment fails (no on-chain rollback)
        #[arg(long)]
        atomic: bool,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Export multiple contracts in bulk
    BatchExport {
        /// Output directory for exported files
        output_dir: String,
        /// Filter query (e.g. network=testnet or category=defi)
        #[arg(long)]
        filter: Option<String>,
        /// Output format: json, csv, archive
        #[arg(long, default_value = "json")]
        format: String,
        /// Organize output by network/category subdirectories
        #[arg(long)]
        organize: bool,
        /// Compress the output directory into a .tar.gz
        #[arg(long)]
        compress: bool,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Import contracts in bulk from a directory
    BatchImport {
        /// Input directory containing contract files to import
        input_dir: String,
        /// Force a specific format (json, csv, archive); auto-detected if omitted
        #[arg(long)]
        format: Option<String>,
        /// How to handle duplicates: skip or fail
        #[arg(long, default_value = "skip")]
        on_duplicate: String,
        /// Preview what would be imported without committing
        #[arg(long)]
        dry_run: bool,
        /// Abort on first error; report atomically
        #[arg(long)]
        atomic: bool,
        /// Output directory for archive imports
        #[arg(long, default_value = "./imported")]
        output_dir: String,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Update metadata for multiple contracts in bulk (#849)
    BatchUpdate {
        /// Path to a YAML or JSON manifest file describing the updates
        #[arg(long)]
        file: Option<String>,

        /// Filter contracts from the API (e.g. "category=defi" or "network=mainnet")
        #[arg(long)]
        filter: Option<String>,

        /// Show what would change without making any writes
        #[arg(long)]
        preview: bool,

        /// Only update contracts where this field=value condition is true
        #[arg(long, value_name = "CONDITION")]
        r#if: Option<String>,

        /// User ID to attribute the update to
        #[arg(long)]
        user_id: Option<String>,

        /// On partial failure, rollback all successfully applied contracts
        #[arg(long)]
        rollback_on_error: bool,

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

    /// Manage local cache of registry API responses (#845)
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
    /// Manage environment variable sets for different deployments (#843)
    Env {
        #[command(subcommand)]
        action: EnvCommands,
    },

    /// Publisher environment diagnostics
    Publisher {
        #[command(subcommand)]
        action: PublisherCommands,
    },

    /// External command (may be provided by an installed plugin)
    #[command(external_subcommand)]
    External(Vec<String>),

    /// Manage signed offline registry snapshots (#1146)
    Snapshot {
        #[command(subcommand)]
        action: SnapshotCommands,
    },
}

/// Sub-commands for the `snapshot` group
#[derive(Debug, Subcommand)]
pub enum SnapshotCommands {
    /// Export a signed offline registry snapshot
    Export {
        /// Output file path
        #[arg(long, short = 'o')]
        output: String,
    },

    /// Sign a registry snapshot
    Sign {
        /// Path to the snapshot JSON file
        snapshot_file: String,

        /// Path to the signing key (Ed25519 PEM or base64)
        #[arg(long)]
        key: String,
    },

    /// Verify a signed registry snapshot locally
    Verify {
        /// Path to the snapshot JSON file
        snapshot_file: String,

        /// Path to the trusted public key
        #[arg(long)]
        trust_key: String,
    },

    /// Inspect a registry snapshot metadata
    Inspect {
        /// Path to the snapshot JSON file
        snapshot_file: String,
    },
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
    UserGet { key: String },
    /// Set a user config value by key
    #[command(name = "set")]
    UserSet { key: String, value: String },
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
pub enum AuthCommands {
    /// Sign in with a GitHub account, Stellar wallet, or API key
    Login {
        /// Authentication method to use
        #[arg(long, value_enum)]
        method: Option<crate::commands::auth::AuthMethod>,

        /// Identity to authenticate with
        #[arg(long)]
        identity: Option<String>,

        /// Secret credential or signing seed
        #[arg(long)]
        secret: Option<String>,

        /// Comma-separated token scopes
        #[arg(long, value_delimiter = ',')]
        scopes: Vec<String>,

        /// Token lifetime, e.g. 1h, 30m, 7d, or seconds
        #[arg(long)]
        expires: Option<String>,
    },

    /// Sign out and remove stored credentials
    Logout {},

    /// Show the current authentication state
    Status {},

    /// Print the current API token, refreshing it when possible
    Token {
        /// Comma-separated token scopes
        #[arg(long, value_delimiter = ',')]
        scopes: Vec<String>,

        /// Token lifetime, e.g. 1h, 30m, 7d, or seconds
        #[arg(long)]
        expires: Option<String>,
    },
}

/// Sub-commands for the `backup` group
#[derive(Debug, Subcommand)]
pub enum BackupCommands {
    /// Create a new contract backup
    Create {
        /// Contract ID to back up
        contract_id: String,
        /// Include full contract state in backup
        #[arg(long)]
        include_state: bool,
    },
    /// List recent backups for a contract
    List {
        /// Contract ID
        contract_id: String,
    },
    /// Restore a contract from a specific backup date
    Restore {
        /// Contract ID to restore
        contract_id: String,
        /// Backup date to restore from (YYYY-MM-DD)
        backup_date: String,
    },
    /// Verify integrity of a specific backup
    Verify {
        /// Contract ID
        contract_id: String,
        /// Backup date to verify (YYYY-MM-DD)
        backup_date: String,
    },
    /// Show backup statistics for a contract
    Stats {
        /// Contract ID
        contract_id: String,
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

/// Sub-commands for `contract category`.
#[derive(Debug, Subcommand)]
pub enum CategoryCommands {
    /// List all categories with descriptions and contract counts
    List {
        /// Scope contract counts to a single network (mainnet | testnet | futurenet)
        #[arg(long)]
        network: Option<String>,

        /// Output format for stdout: table, json, csv, yaml
        #[arg(long, default_value = "table")]
        format: String,

        /// Also write the category list to a file: csv or json
        #[arg(long)]
        export: Option<String>,
    },

    /// Show detailed per-category statistics (counts, recent, trending)
    Stats {
        /// Scope statistics to a single network (mainnet | testnet | futurenet)
        #[arg(long)]
        network: Option<String>,

        /// Output format for stdout: table, json, csv, yaml
        #[arg(long, default_value = "table")]
        format: String,

        /// Also write the statistics to a file: csv or json
        #[arg(long)]
        export: Option<String>,
    },
}

/// Sub-commands for the `contract` group (#522)
#[derive(Debug, Subcommand)]
pub enum ContractCommands {
    /// List registered contracts, a page at a time
    ///
    /// Shows address, name, network, category and last update. JSON and CSV
    /// carry no decoration, so they pipe cleanly into other tools.
    ///
    /// Examples:
    ///   soroban-registry contract list
    ///   soroban-registry contract list --limit 50 --offset 100
    ///   soroban-registry contract list --networks testnet --category DeFi
    ///   soroban-registry contract list --format json | jq '.contracts[].address'
    ///   soroban-registry contract list --format csv > contracts.csv
    #[command(verbatim_doc_comment)]
    List {
        /// Contracts per page (1-100)
        #[arg(long, short, default_value_t = crate::commands::contract::list::DEFAULT_LIMIT)]
        limit: usize,

        /// Contracts to skip; use it with --limit to page through the registry
        #[arg(long, short, default_value_t = 0)]
        offset: usize,

        /// Filter by network (comma-separated: mainnet,testnet,futurenet).
        /// Named `networks` because the global `--network` owns that arg id.
        #[arg(long)]
        networks: Option<String>,

        /// Filter by category (comma-separated: DeFi,NFT)
        #[arg(long, short)]
        category: Option<String>,

        /// Output format: table, json or csv
        #[arg(long, short, default_value = "table")]
        format: String,
    },

    /// Search the registry, one page at a time or across every page
    ///
    /// Pagination is handled for you: `--all` walks pages until the result set
    /// runs out or a safety bound is reached, and never loops on a bad
    /// continuation token. Cursor and offset parameters cannot be combined.
    ///
    /// Usage: soroban-registry contract search <QUERY> [--all] [--max-items <N>]
    ///        [--pagination <cursor|offset>] [--cursor <TOKEN> | --offset <N>]
    Search {
        /// Search query
        query: String,

        /// Filter by network (comma-separated: mainnet,testnet,futurenet)
        #[arg(long)]
        networks: Option<String>,

        /// Filter by category (comma-separated: DeFi,NFT)
        #[arg(long)]
        category: Option<String>,

        /// Filter by tag (comma-separated: defi,amm)
        #[arg(long)]
        tags: Option<String>,

        /// Only show verified contracts
        #[arg(long)]
        verified_only: bool,

        /// Results per page (1-100)
        #[arg(long, default_value = "20")]
        limit: u32,

        /// Start at this row offset. Offset pagination only — cannot be
        /// combined with --cursor.
        #[arg(long, conflicts_with = "cursor")]
        offset: Option<u64>,

        /// Resume from a continuation token returned by a previous run. Cursor
        /// pagination only — the token is opaque and must not be edited.
        #[arg(long)]
        cursor: Option<String>,

        /// Pagination mode: cursor (stable, no skips or duplicates) or offset
        /// (relevance ordered). Defaults to cursor for --all, offset otherwise.
        #[arg(long, value_name = "MODE")]
        pagination: Option<String>,

        /// Fetch every page, up to --max-items / --max-pages
        #[arg(long)]
        all: bool,

        /// Maximum items to fetch with --all (default 1000)
        #[arg(long)]
        max_items: Option<u64>,

        /// Maximum pages to fetch with --all (default 100)
        #[arg(long)]
        max_pages: Option<u64>,

        /// Output as JSON, including pagination metadata
        #[arg(long)]
        json: bool,
    },

    /// Export a signed, offline-verifiable snapshot of a contract (#1116)
    ///
    /// Captures metadata, verification status, dependency scan findings,
    /// deprecation state and successor lineage as of now, signed by the
    /// registry so it can be audited without a live API.
    ///
    /// Usage: soroban-registry contract snapshot <ID> --output <FILE>
    Snapshot {
        /// Contract UUID to snapshot
        id: String,

        /// File to write the signed snapshot to
        #[arg(long, short = 'o')]
        output: String,

        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify a previously exported contract snapshot (#1116)
    ///
    /// Runs entirely offline unless --fetch-key is passed. Exits non-zero when
    /// verification fails, so it can gate a compliance pipeline.
    ///
    /// Usage: soroban-registry contract verify-snapshot <FILE> [--expect-key <FP>]
    VerifySnapshot {
        /// Path to the snapshot file
        file: String,

        /// Registry key fingerprint to pin against. Without it, a valid result
        /// proves only that the bundle is self-consistent.
        #[arg(long)]
        expect_key: Option<String>,

        /// Fail if the snapshot is older than this many days
        #[arg(long)]
        max_age_days: Option<i64>,

        /// Fetch the expected fingerprint from the registry instead of pinning
        /// it locally. Requires network access.
        #[arg(long)]
        fetch_key: bool,

        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Assess security and operational risks for a contract (#837)
    ///
    /// Usage: soroban-registry contract risk <address> [--network <n>] [--threshold <level>] [--json]
    Risk {
        /// On-chain contract address or registry UUID to assess
        address: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Exit with code 1 if overall risk level meets or exceeds this threshold
        /// (low | medium | high | critical)
        #[arg(long)]
        threshold: Option<String>,

        /// Output the risk report as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Deploy and register a new contract in the registry
    ///
    /// Usage: soroban-registry contract deploy <WASM_PATH> --name <NAME> --network <NETWORK>
    ///        [--description <DESC>] [--category <CAT>] [--icon <ICON_PATH>]
    ///        [--interactive] [--publisher <ADDRESS>] [--tags <TAGS>]
    Deploy {
        /// Path to the WASM binary file
        wasm_path: String,

        /// Contract name (human-readable)
        #[arg(long)]
        name: Option<String>,

        /// Contract description
        #[arg(long)]
        description: Option<String>,

        /// Contract category (DeFi, Token, Oracle, NFT, Utility, Other)
        #[arg(long)]
        category: Option<String>,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Path to contract icon file (PNG, JPG, SVG)
        #[arg(long)]
        icon: Option<String>,

        /// Enable interactive mode for guided deployment
        #[arg(long)]
        interactive: bool,

        /// Publisher's Stellar address (if not set, uses default publisher)
        #[arg(long)]
        publisher: Option<String>,

        /// Comma-separated list of tags for the contract
        #[arg(long)]
        tags: Option<String>,

        /// Skip ABI extraction and deployment verification
        #[arg(long)]
        skip_abi: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Register one or more contracts in the registry
    Register {
        /// Path to a YAML or JSON metadata file
        #[arg(long)]
        file: Option<String>,

        /// Enable repeated prompts for multiple contracts
        #[arg(long)]
        batch: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify a contract — a local WASM artifact before publishing, or a
    /// deployed contract's authenticity against the on-chain registry.
    ///
    /// Local:    soroban-registry contract verify --wasm <path> [--verbose] [--json]
    /// On-chain: soroban-registry contract verify <address> --network <network> [--json] [--strict] [--batch] [--no-cache]
    Verify {
        /// On-chain contract address to verify (or comma-separated list for batch
        /// verification). Omit when using --wasm for local verification.
        address: Option<String>,

        /// Path to a local compiled WASM contract to verify before publishing.
        /// Runs the same structural checks the backend uses, offline. In local
        /// mode, pass the global -v/--verbose flag for detailed diagnostics.
        #[arg(long)]
        wasm: Option<String>,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,

        /// Strict mode: fail if any warnings or errors are found
        #[arg(long)]
        strict: bool,

        /// Batch mode: verify multiple contracts (comma-separated addresses)
        #[arg(long)]
        batch: bool,

        /// Skip cache and always fetch fresh data from registry
        #[arg(long)]
        no_cache: bool,
    },

    /// Derive and display a contract's deterministic interface fingerprint
    /// (functions, types, events, errors) from a local compiled WASM
    /// artifact.
    ///
    /// Usage: soroban-registry contract interfaces --wasm <path> [--json]
    Interfaces {
        /// Path to a local compiled WASM contract to inspect.
        #[arg(long)]
        wasm: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Display build-provenance metadata recorded for a contract, read from
    /// a local manifest file.
    ///
    /// Usage: soroban-registry contract provenance --manifest <path> [--json]
    Provenance {
        /// Path to a local provenance manifest (JSON) to display.
        #[arg(long)]
        manifest: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Attempt to independently reproduce a contract's published WASM
    /// artifact from source, and compare its hash against the expected
    /// (registry-recorded) artifact hash.
    ///
    /// Usage: soroban-registry contract verify-build --manifest <path> --source-dir <dir> --expected-hash <hash> [--allow-toolchain-mismatch] [--json]
    VerifyBuild {
        /// Path to a local provenance manifest (JSON) describing the recorded build.
        #[arg(long)]
        manifest: String,

        /// Directory containing the contract's source to rebuild.
        #[arg(long)]
        source_dir: String,

        /// The registry-recorded WASM artifact hash to compare the rebuild against.
        #[arg(long)]
        expected_hash: String,

        /// Proceed with the rebuild even if the locally installed rustc
        /// version doesn't match the version recorded in provenance.
        #[arg(long)]
        allow_toolchain_mismatch: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Structurally compare two local compiled WASM artifacts and classify
    /// ABI changes as compatible, potentially breaking, breaking, or
    /// unknown.
    ///
    /// Usage: soroban-registry contract compatibility --from <wasm> --to <wasm> [--strict] [--json] [--fail-on <level>]
    Compatibility {
        /// Path to the earlier/baseline compiled WASM contract.
        #[arg(long)]
        from: String,

        /// Path to the newer/candidate compiled WASM contract.
        #[arg(long)]
        to: String,

        /// Network passphrase associated with the `--from` artifact.
        #[arg(long)]
        from_network_passphrase: Option<String>,

        /// Network passphrase associated with the `--to` artifact.
        #[arg(long)]
        to_network_passphrase: Option<String>,

        /// Exit non-zero when changes at or above the --fail-on threshold
        /// are found (default threshold: potentially_breaking).
        #[arg(long)]
        strict: bool,

        /// Minimum severity that triggers a non-zero exit under --strict:
        /// breaking | potential | unknown
        #[arg(long, default_value = "potential")]
        fail_on: String,

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

    /// Show contract registry statistics and analytics
    ///
    /// Usage: soroban-registry contract stats [--network testnet] [--category defi]
    Stats {
        /// Filter stats by network
        #[arg(long)]
        network: Option<String>,

        /// Filter stats by category
        #[arg(long)]
        category: Option<String>,

        /// Number of popular contracts to display
        #[arg(long, default_value_t = 10)]
        top_n: usize,

        /// Output format: table, json, csv, yaml
        #[arg(long, default_value = "table")]
        format: String,

        /// Export stats to a file
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Compare against another period, for example 7d or 30d
        #[arg(long)]
        compare: Option<String>,
    },

    /// Export contracts and related registry data for backup or migration
    ///
    /// Usage: soroban-registry contract export [OUTPUT_FILE] --format json
    Export {
        /// Optional output file path
        output_file: Option<String>,

        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Export format: json, csv, jsonl, sqlite, markdown, or archive
        #[arg(long, short = 'f', default_value = "json")]
        format: String,

        /// Filter by network
        #[arg(long)]
        network: Option<String>,

        /// Filter by category
        #[arg(long)]
        category: Option<String>,

        /// Export only contracts updated since this date
        #[arg(long)]
        since: Option<String>,

        /// Write a gzip-compressed export file
        #[arg(long)]
        compress: bool,

        /// Include related data such as versions, dependencies, analytics, and reviews
        #[arg(long, default_value_t = true)]
        include_related: bool,

        /// Number of contracts to fetch per API page
        #[arg(long, default_value_t = 100)]
        page_size: usize,
    },
    /// Manage featured (highlighted) contracts (#832)
    ///
    /// Usage: soroban-registry contract highlight [ADDRESS] --action <add|remove|list|check>
    Highlight {
        /// Contract address (required for add/remove/check)
        address: Option<String>,
        /// Action to perform: add | remove | list | check
        #[arg(long, default_value = "list")]
        action: String,
        /// Curator bearer token for mutating actions (add/remove)
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// View a contract's interactions and call patterns (#835)
    Interaction {
        /// On-chain contract address
        address: String,
        /// Max number of recent interactions to display
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },

    /// Analyze a contract's dependencies and relationships (#836, #1008)
    ///
    /// Retrieves the full dependency graph: contracts this address depends on,
    /// contracts that depend on it, and a recursive dependency tree.
    ///
    /// Use `--summary` for a compact view when dealing with large graphs.
    /// Use `--format json` to get the raw API response for scripting.
    Dependency {
        /// On-chain contract address
        address: String,
        /// Dependency tree depth (0 = direct dependencies only)
        #[arg(long, default_value_t = 1)]
        depth: u32,
        /// Output format: table, json, csv, yaml
        #[arg(long, default_value = "table")]
        format: String,
        /// Compact summary mode: show aggregate counts without the full tree
        #[arg(long)]
        summary: bool,
    },

    /// List what a contract depends on (#1147)
    ///
    /// Usage: soroban-registry contract dependencies <ADDRESS> [--network <NET>]
    ///        [--transitive] [--depth N] [--json]
    ///
    /// A bare contract address registered on more than one network is ambiguous;
    /// pass --network to disambiguate.
    Dependencies {
        /// On-chain contract address or registry UUID
        address: String,
        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long)]
        network: Option<String>,
        /// Walk the whole dependency closure, not just direct edges
        #[arg(long)]
        transitive: bool,
        /// Maximum traversal depth (capped server-side)
        #[arg(long)]
        depth: Option<u32>,
        /// Include on-chain call edges alongside declared ones
        #[arg(long)]
        include_telemetry: bool,
        /// Print the registry response as JSON
        #[arg(long)]
        json: bool,
    },

    /// List what depends on a contract (#1147)
    ///
    /// Usage: soroban-registry contract dependents <ADDRESS> [--network <NET>]
    ///        [--transitive] [--depth N] [--json]
    Dependents {
        /// On-chain contract address or registry UUID
        address: String,
        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long)]
        network: Option<String>,
        /// Walk the whole dependent closure, not just direct edges
        #[arg(long)]
        transitive: bool,
        /// Maximum traversal depth (capped server-side)
        #[arg(long)]
        depth: Option<u32>,
        /// Include on-chain call edges alongside declared ones
        #[arg(long)]
        include_telemetry: bool,
        /// Print the registry response as JSON
        #[arg(long)]
        json: bool,
    },

    /// Report direct and inherited risk across a contract's dependencies (#1147)
    ///
    /// Usage: soroban-registry contract dependency-risk <ADDRESS> [--network <NET>]
    ///        [--depth N] [--fail-on low|medium|high|critical] [--json]
    ///
    /// Each finding carries the shortest dependency path that reaches it.
    /// With --fail-on, exits 1 when the overall risk meets or exceeds the level.
    DependencyRisk {
        /// On-chain contract address or registry UUID
        address: String,
        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long)]
        network: Option<String>,
        /// Maximum traversal depth (capped server-side)
        #[arg(long)]
        depth: Option<u32>,
        /// Exit 1 when overall risk meets or exceeds this level
        #[arg(long)]
        fail_on: Option<String>,
        /// Print the registry response as JSON
        #[arg(long)]
        json: bool,
    },

    /// List and inspect contract categories
    Category {
        #[command(subcommand)]
        action: CategoryCommands,
    },

    /// Update contract metadata after registration (#828)
    ///
    /// Usage: soroban-registry contract update <ADDRESS> [--name ...] [--dry-run]
    Update {
        /// Contract address, slug, or registry UUID
        address: String,

        /// Updated contract name
        #[arg(long)]
        name: Option<String>,

        /// Updated description
        #[arg(long)]
        description: Option<String>,

        /// Updated category
        #[arg(long)]
        category: Option<String>,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Path to a new icon image (PNG, JPG, or SVG)
        #[arg(long)]
        icon: Option<String>,

        /// Contract homepage URL (not yet supported by registry API)
        #[arg(long)]
        homepage: Option<String>,

        /// Preview changes without submitting them
        #[arg(long)]
        dry_run: bool,

        /// Skip interactive confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Import contracts into the registry from an external file (#831)
    ///
    /// Supports JSON, JSONL (newline-delimited JSON), CSV, and archive formats.
    ///
    /// Usage: soroban-registry contract import <INPUT_FILE> [OPTIONS]
    Import {
        /// Path to the input file (JSON, JSONL, CSV, or .tar.gz archive)
        input_file: String,

        /// Input format override (json | jsonl | csv | sqlite | archive).
        /// Inferred from the file extension when omitted.
        #[arg(long, short = 'f')]
        format: Option<String>,

        /// How to handle duplicate contracts: skip | update | fail (default: skip)
        #[arg(long, default_value = "skip")]
        on_duplicate: String,

        /// Network alias mappings, e.g. --network-map futurenet=testnet
        /// May be repeated for multiple aliases.
        #[arg(long = "network-map")]
        network_map: Vec<String>,

        /// Preview what would be imported without writing to the registry
        #[arg(long)]
        dry_run: bool,

        /// Validate all records before importing; abort on any error
        #[arg(long)]
        validate: bool,

        /// Roll back all successful imports if any record fails
        #[arg(long)]
        atomic: bool,

        /// Write the JSON import-summary report to this file path
        /// (prints to stdout when omitted)
        #[arg(long, short = 'o')]
        report_output: Option<String>,

        /// Directory for archive extraction (archive format only)
        #[arg(long, default_value = "./imported")]
        output_dir: String,
    },

    /// Rollback a deprecated contract to active state (#1091)
    ///
    /// Usage: soroban-registry contract rollback <ADDRESS> --reason <REASON> --private-key <KEY>
    Rollback {
        /// Contract address or registry UUID to rollback
        address: String,

        /// Human-readable reason for rollback
        #[arg(long)]
        reason: String,

        /// Publisher's Ed25519 private key (base64-encoded)
        #[arg(long)]
        private_key: String,

        /// Skip interactive confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Detect drift between local lockfile and registry state (#1060)
    ///
    /// Compares a local `soroban-registry.lock.json` against the live registry
    /// and reports added, removed, and changed contract metadata.
    ///
    /// Usage: soroban-registry contract audit [--lockfile PATH] [--fix] [--init --contracts a,b]
    Audit {
        /// Path to lockfile (default: soroban-registry.lock.json)
        #[arg(long, default_value = "soroban-registry.lock.json")]
        lockfile: String,

        /// Auto-sync lockfile to match current registry state
        #[arg(long)]
        fix: bool,

        /// Generate an initial lockfile from the given contract IDs
        #[arg(long)]
        init: bool,

        /// Contract IDs for --init (comma-separated)
        #[arg(long, value_delimiter = ',')]
        contracts: Vec<String>,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Deprecate a contract with publisher-signed authorization (#1091)
    ///
    /// Requires the publisher's Ed25519 private key to sign the deprecation
    /// payload. The backend verifies the signature against the stored publisher
    /// public key before applying the state change.
    ///
    /// Usage: soroban-registry contract deprecate <ADDRESS> --reason <REASON> --private-key <KEY>
    Deprecate {
        /// Contract address or registry UUID to deprecate
        address: String,

        /// Human-readable reason for deprecation
        #[arg(long)]
        reason: String,

        /// Replacement contract ID for downstream migration
        #[arg(long)]
        replacement: Option<String>,

        /// Publisher's Ed25519 private key (base64-encoded)
        #[arg(long)]
        private_key: String,

        /// URL to a migration guide for consumers
        #[arg(long)]
        migration_guide: Option<String>,

        /// Grace period in days before hard removal (default: 90)
        #[arg(long, default_value_t = 90)]
        grace_period_days: i32,

        /// Skip interactive confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage contract event notifications and alerts (#838)
    Notification {
        #[command(subcommand)]
        action: NotificationCommands,
    },

    /// Detect three-way WASM drift (lockfile ↔ registry ↔ chain) (#1191)
    ///
    /// Usage:
    ///   soroban-registry contract drift --id <CONTRACT_ID> [--lockfile <PATH>] [--network <NET>] [--json]
    ///   soroban-registry contract drift --all [--status <STATUS>] [--network <NET>] [--json]
    Drift {
        /// Contract ID or address to check for drift
        #[arg(long)]
        id: Option<String>,

        /// Query drift state registry-wide across all monitored contracts
        #[arg(long)]
        all: bool,

        /// Filter registry-wide drift check by status: match | drift | not_on_chain | unknown
        #[arg(long)]
        status: Option<String>,

        /// Path to local lockfile for three-way comparison (default: soroban-registry.lock.json if present)
        #[arg(long)]
        lockfile: Option<String>,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long)]
        network: Option<String>,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for `contract notification`
#[derive(Debug, Subcommand)]
pub enum NotificationCommands {
    /// Subscribe to alerts for a contract address
    Subscribe {
        /// On-chain contract address
        address: String,

        /// Alert types (comma-separated): updates, audits, security, deployments
        #[arg(long, default_value = "updates,security")]
        alerts: String,

        /// Notification channels (comma-separated): email, webhook, cli
        #[arg(long, default_value = "cli")]
        channels: String,

        /// Notification frequency: instant, daily, weekly
        #[arg(long, default_value = "instant")]
        frequency: String,

        /// Filter by networks (comma-separated, e.g. mainnet,testnet)
        #[arg(long, default_value = "")]
        networks: String,

        /// Filter by categories (comma-separated, e.g. defi,token)
        #[arg(long, default_value = "")]
        categories: String,

        /// Email address or webhook URL for the chosen channel
        #[arg(long)]
        target: Option<String>,
    },

    /// Unsubscribe from alerts for a contract address
    Unsubscribe {
        /// On-chain contract address
        address: String,
    },

    /// List active notification rules
    List {
        /// Filter by contract address (omit to list all)
        address: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Update an existing notification rule
    Configure {
        /// On-chain contract address
        address: String,

        /// New alert types (comma-separated)
        #[arg(long)]
        alerts: Option<String>,

        /// New channels (comma-separated)
        #[arg(long)]
        channels: Option<String>,

        /// New frequency: instant, daily, weekly
        #[arg(long)]
        frequency: Option<String>,

        /// New network filter (comma-separated)
        #[arg(long)]
        networks: Option<String>,

        /// New category filter (comma-separated)
        #[arg(long)]
        categories: Option<String>,

        /// New email address or webhook URL
        #[arg(long)]
        target: Option<String>,
    },

    /// Send a test alert for a subscribed contract
    Test {
        /// On-chain contract address
        address: String,
    },
}

/// Sub-commands for the `api-key` group (#842)
#[derive(Debug, Subcommand)]
pub enum ApiKeyCommands {
    /// Create a new API key
    Create {
        /// Expiry (ISO date or duration, e.g. 2026-12-31 or 30d)
        #[arg(long)]
        expires: Option<String>,
        /// Comma-separated scopes / permissions
        #[arg(long)]
        scopes: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List your API keys
    List {
        #[arg(long)]
        json: bool,
    },
    /// Permanently delete an API key
    Delete {
        /// API key id
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke (disable) an API key without deleting its audit record
    Revoke {
        /// API key id
        id: String,
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for the `env` group (#843)
#[derive(Debug, Subcommand)]
pub enum EnvCommands {
    /// Set a variable in an environment
    ///
    /// Usage: soroban-registry env set <NAME> <VALUE> [--env <environment>]
    Set {
        /// Variable name (shell identifier: letters, digits, underscores)
        name: String,
        /// Value to assign
        value: String,
        /// Target environment (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// Print the full value instead of masking it
        #[arg(long)]
        show_value: bool,
    },

    /// Get a variable's value from an environment
    ///
    /// Usage: soroban-registry env get <NAME> [--env <environment>] [--json]
    Get {
        /// Variable name to look up
        name: String,
        /// Source environment (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// List variables in an environment
    ///
    /// Usage: soroban-registry env list [--env <environment>] [--all] [--merged] [--json]
    List {
        /// Environment to list (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// List variables in every environment
        #[arg(long)]
        all: bool,
        /// Merge global config defaults into the output
        #[arg(long)]
        merged: bool,
        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Copy all variables from one environment to another
    ///
    /// Usage: soroban-registry env copy --from <src> --to <dst>
    Copy {
        /// Source environment name
        #[arg(long)]
        from: String,
        /// Destination environment name
        #[arg(long)]
        to: String,
        /// Overwrite the destination if it already exists
        #[arg(long)]
        overwrite: bool,
    },

    /// Delete a variable from an environment
    ///
    /// Usage: soroban-registry env delete <NAME> [--env <environment>]
    Delete {
        /// Variable name to remove
        name: String,
        /// Source environment (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
    },

    /// Export environment variables as a shell-sourceable file
    ///
    /// Usage: soroban-registry env export [--env <environment>] [--format shell|json|dotenv]
    Export {
        /// Environment to export (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// Output format: shell (default), json, dotenv
        #[arg(long, value_enum, default_value_t = EnvExportFormat::Shell)]
        format: EnvExportFormat,
        /// Merge global config defaults into the export
        #[arg(long)]
        merged: bool,
    },

    /// Switch the active environment
    ///
    /// Usage: soroban-registry env switch <ENVIRONMENT>
    Switch {
        /// Environment name to activate
        environment: String,
    },
}

/// Sub-commands for the `publisher` group
#[derive(Debug, Subcommand)]
pub enum PublisherCommands {
    /// Diagnose the local publishing environment (config, session, signing key, connectivity)
    ///
    /// Usage: soroban-registry publisher doctor [--json]
    Doctor {
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum EnvExportFormat {
    Shell,
    Json,
    Dotenv,
}

impl EnvExportFormat {
    // `pub(crate)` because the dispatcher reads it; it used to sit in the same
    // module as its only caller.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Json => "json",
            Self::Dotenv => "dotenv",
        }
    }
}

/// Sub-commands for the `cache` group (#845)
#[derive(Debug, Subcommand)]
pub enum CacheCommands {
    /// Clear cached entries from disk
    ///
    /// Usage: soroban-registry cache clear [--level disk|memory|all] [--key <key>]
    Clear {
        /// Cache level to clear: disk (default), memory, all
        #[arg(long, default_value = "disk")]
        level: String,
        /// Clear only the entry matching this specific cache key
        #[arg(long)]
        key: Option<String>,
    },

    /// Show cache statistics and configuration
    ///
    /// Usage: soroban-registry cache status [--json]
    Status {
        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Configure cache settings
    ///
    /// Usage: soroban-registry cache configure [--ttl <secs>] [--max-size <bytes>]
    ///                                         [--compression on|off] [--auto-refresh on|off]
    Configure {
        /// Default TTL for cached entries in seconds
        #[arg(long)]
        ttl: Option<u64>,
        /// Maximum disk cache size in bytes (0 = unlimited)
        #[arg(long)]
        max_size: Option<u64>,
        /// Enable or disable compression for disk entries: on | off
        #[arg(long)]
        compression: Option<String>,
        /// Enable or disable automatic refresh of stale entries: on | off
        #[arg(long)]
        auto_refresh: Option<String>,
        /// Output current (or updated) config as JSON
        #[arg(long)]
        json: bool,
    },

    /// Remove stale entries and enforce disk size limit
    ///
    /// Usage: soroban-registry cache optimize [--json]
    Optimize {
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Export cache entries for analysis
    ///
    /// Usage: soroban-registry cache export [--format json|csv] [--include-stale]
    Export {
        /// Output format: json (default) or csv
        #[arg(long, default_value = "json")]
        format: String,
        /// Include stale (expired) entries in the export
        #[arg(long)]
        include_stale: bool,
    },
}

/// Sub-commands for the `profile` group (#841)
#[derive(Debug, Subcommand)]
pub enum ProfileCommands {
    /// Display a publisher profile
    ///
    /// Usage: soroban-registry profile view [--address <stellar-address>] [--json]
    View {
        /// Stellar address or publisher UUID to look up (defaults to the address in local config)
        #[arg(long)]
        address: Option<String>,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Update profile fields
    ///
    /// Usage: soroban-registry profile edit --name <n> --website <url> ...
    Edit {
        /// Display name
        #[arg(long)]
        name: Option<String>,

        /// Short biography or description
        #[arg(long)]
        bio: Option<String>,

        /// Personal or project website URL
        #[arg(long)]
        website: Option<String>,

        /// Contact email address
        #[arg(long)]
        email: Option<String>,

        /// GitHub profile URL
        #[arg(long)]
        github: Option<String>,

        /// Avatar image URL
        #[arg(long)]
        avatar: Option<String>,
    },

    /// Update a single profile field by key
    ///
    /// Usage: soroban-registry profile update --field <key> --value <val>
    Update {
        /// Field to update (name | bio | website | email | github | avatar)
        #[arg(long)]
        field: String,

        /// New value for the field
        #[arg(long)]
        value: String,
    },

    /// List contracts published by a profile
    ///
    /// Usage: soroban-registry profile list-contracts [--address <addr>] [--limit N]
    #[command(name = "list-contracts")]
    ListContracts {
        /// Stellar address or publisher UUID (defaults to local config)
        #[arg(long)]
        address: Option<String>,

        /// Maximum number of contracts to return
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Output format: table | csv
        #[arg(long, default_value = "table")]
        format: String,

        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Export full profile data to JSON or CSV
    ///
    /// Usage: soroban-registry profile export [--address <addr>] [--format json|csv]
    Export {
        /// Stellar address or publisher UUID (defaults to local config)
        #[arg(long)]
        address: Option<String>,

        /// Export format: json | csv
        #[arg(long, default_value = "json")]
        format: String,
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

#[cfg(test)]
mod verbose_flag_tests {
    use super::*;
    use crate::with_large_stack;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        with_large_stack(move || Cli::try_parse_from(args).expect("CLI should parse"))
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

    #[test]
    fn env_export_rejects_invalid_format() {
        let err = with_large_stack(|| {
            Cli::try_parse_from(["soroban-registry", "env", "export", "--format", "invalid"])
                .expect_err("CLI should reject invalid export format")
        });

        assert!(
            err.to_string().contains("possible values"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn env_set_parses_show_value_flag() {
        let cli = parse(&[
            "soroban-registry",
            "env",
            "set",
            "API_KEY",
            "secret",
            "--show-value",
        ]);

        match cli.command {
            Commands::Env {
                action: EnvCommands::Set { show_value, .. },
            } => assert!(show_value),
            _ => panic!("expected env set command"),
        }
    }
}

/// Guards against the regression this module was written for: a subcommand that exists as
/// a module and a dispatch arm but was dropped from the command tree by a bad merge, and
/// so silently disappears from `--help`. These tests introspect the clap tree rather than
/// string-matching help output, so they fail on the definition rather than on its
/// rendering.
#[cfg(test)]
mod command_tree_tests {
    use super::*;
    use crate::with_large_stack;
    use clap::CommandFactory;

    /// `external_subcommand` variants are deliberately hidden and carry no help text, so
    /// they are exempt from the checks below, as is anything explicitly marked hidden.
    fn is_exempt(cmd: &clap::Command) -> bool {
        cmd.is_hide_set() || cmd.get_name().is_empty()
    }

    #[test]
    fn command_tree_is_valid() {
        // Clap's own validator: catches duplicate names, conflicting short flags, and
        // defaults that do not parse, across the whole tree.
        with_large_stack(|| Cli::command().debug_assert());
    }

    #[test]
    fn every_subcommand_has_help_text() {
        fn check(cmd: &clap::Command, path: &str, missing: &mut Vec<String>) {
            for sub in cmd.get_subcommands() {
                if is_exempt(sub) {
                    continue;
                }
                let full = format!("{path} {}", sub.get_name());
                // `help` is generated by clap and carries no `about` of its own.
                if sub.get_name() != "help" && sub.get_about().is_none() {
                    missing.push(full.clone());
                }
                check(sub, &full, missing);
            }
        }

        let missing = with_large_stack(|| {
            let cmd = Cli::command();
            let mut missing = Vec::new();
            check(&cmd, "soroban-registry", &mut missing);
            missing
        });

        assert!(
            missing.is_empty(),
            "these subcommands render with no description in --help: {missing:#?}"
        );
    }

    #[test]
    fn every_top_level_command_appears_in_help() {
        let (names, help) = with_large_stack(|| {
            let mut cmd = Cli::command();
            let names: Vec<String> = cmd
                .get_subcommands()
                .filter(|sub| !is_exempt(sub))
                .map(|sub| sub.get_name().to_string())
                .collect();
            let help = cmd.render_long_help().to_string();
            (names, help)
        });

        assert!(
            !names.is_empty(),
            "the command tree reported no top-level subcommands"
        );

        let missing: Vec<&String> = names
            .iter()
            .filter(|name| !help.contains(name.as_str()))
            .collect();

        assert!(
            missing.is_empty(),
            "these commands are defined but absent from --help: {missing:#?}"
        );
    }

    /// The three variants this branch restored after merges dropped them. Named
    /// explicitly so a future regression points straight at the cause.
    #[test]
    fn restored_commands_are_reachable() {
        for args in [
            ["contract", "category"].as_slice(),
            ["contract", "notification"].as_slice(),
            ["publisher", "doctor"].as_slice(),
        ] {
            let args = args.to_vec();
            let rendered = args.join(" ");
            let found = with_large_stack(move || {
                let mut cmd = Cli::command();
                for part in &args {
                    match cmd.find_subcommand(part) {
                        Some(sub) => cmd = sub.clone(),
                        None => return false,
                    }
                }
                true
            });

            assert!(
                found,
                "`soroban-registry {rendered}` is not in the command tree"
            );
        }
    }
}
