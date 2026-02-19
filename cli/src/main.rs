mod commands;
mod compliance;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "soroban-registry")]
#[command(about = "CLI tool for the Soroban Contract Registry", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API URL (defaults to http://localhost:3001)
    #[arg(long, env = "SOROBAN_REGISTRY_API_URL", default_value = "http://localhost:3001")]
    api_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for contracts
    Search {
        /// Search query
        query: String,

        /// Filter by network
        #[arg(long)]
        network: Option<String>,

        /// Show only verified contracts
        #[arg(long)]
        verified_only: bool,
    },

    /// Get contract information
    Info {
        /// Contract ID
        contract_id: String,
    },

    /// Publish a contract to the registry
    Publish {
        /// Contract ID (Stellar address)
        #[arg(long)]
        contract_id: String,

        /// Contract name
        #[arg(long)]
        name: String,

        /// Contract description
        #[arg(long)]
        description: Option<String>,

        /// Network (mainnet, testnet, futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Category
        #[arg(long)]
        category: Option<String>,

        /// Tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,

        /// Publisher Stellar address
        #[arg(long)]
        publisher: String,
    },

    /// List recent contracts
    List {
        /// Number of contracts to show
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Filter by network
        #[arg(long)]
        network: Option<String>,
    },

    /// Compliance toolkit for contract validation
    #[command(subcommand)]
    Compliance(ComplianceCommands),
}

#[derive(Subcommand)]
enum ComplianceCommands {
    /// Run compliance audit on a contract
    Audit {
        /// Contract ID
        contract_id: String,

        /// Compliance framework (gdpr, soc2, hipaa, iso27001, pci_dss)
        #[arg(long, short)]
        framework: String,
    },

    /// Generate compliance report
    Report {
        /// Contract ID
        contract_id: String,

        /// Compliance framework
        #[arg(long, short)]
        framework: String,

        /// Output file path for JSON report
        #[arg(long, short)]
        output: Option<String>,
    },

    /// Identify compliance gaps
    Gaps {
        /// Contract ID
        contract_id: String,

        /// Compliance framework
        #[arg(long, short)]
        framework: String,
    },

    /// Get remediation suggestions
    Remediate {
        /// Contract ID
        contract_id: String,

        /// Compliance framework
        #[arg(long, short)]
        framework: String,
    },

    /// Start certification process
    Certify {
        /// Contract ID
        contract_id: String,

        /// Compliance framework
        #[arg(long, short)]
        framework: String,
    },

    /// List supported compliance frameworks
    Frameworks,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search { query, network, verified_only } => {
            commands::search(&cli.api_url, &query, network.as_deref(), verified_only).await?;
        }
        Commands::Info { contract_id } => {
            commands::info(&cli.api_url, &contract_id).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            network,
            category,
            tags,
            publisher,
        } => {
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            commands::publish(
                &cli.api_url,
                &contract_id,
                &name,
                description.as_deref(),
                &network,
                category.as_deref(),
                tags_vec,
                &publisher,
            )
            .await?;
        }
        Commands::List { limit, network } => {
            commands::list(&cli.api_url, limit, network.as_deref()).await?;
        }
        Commands::Compliance(comp_cmd) => {
            match comp_cmd {
                ComplianceCommands::Audit { contract_id, framework } => {
                    compliance::audit(&cli.api_url, &contract_id, &framework).await?;
                }
                ComplianceCommands::Report { contract_id, framework, output } => {
                    compliance::report(&cli.api_url, &contract_id, &framework, output.as_deref()).await?;
                }
                ComplianceCommands::Gaps { contract_id, framework } => {
                    compliance::gaps(&cli.api_url, &contract_id, &framework).await?;
                }
                ComplianceCommands::Remediate { contract_id, framework } => {
                    compliance::remediate(&cli.api_url, &contract_id, &framework).await?;
                }
                ComplianceCommands::Certify { contract_id, framework } => {
                    compliance::certify(&cli.api_url, &contract_id, &framework).await?;
                }
                ComplianceCommands::Frameworks => {
                    compliance::frameworks(&cli.api_url).await?;
                }
            }
        }
    }

    Ok(())
}
