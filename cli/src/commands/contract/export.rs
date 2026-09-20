//! `soroban-registry contract export` — export one contract.

use anyhow::Result;
use colored::Colorize;

#[allow(clippy::too_many_arguments)]
pub async fn contract_export(
    api_url: &str,
    output: Option<&str>,
    format: &str,
    network: Option<&str>,
    category: Option<&str>,
    since: Option<&str>,
    compress: bool,
    include_related: bool,
    page_size: usize,
) -> Result<()> {
    let mut filters = Vec::new();
    if let Some(network) = network {
        filters.push(format!("network={}", network));
    }
    if let Some(category) = category {
        filters.push(format!("category={}", category));
    }
    if let Some(since) = since {
        filters.push(format!("updated_from={}", since));
    }

    let resolved_format =
        crate::commands::export::RegistryExportFormat::resolve(Some(format), None, output)?;
    let summary = crate::commands::export::export_registry_data(
        crate::commands::export::RegistryExportOptions {
            api_url,
            id: None,
            output,
            contract_dir: ".",
            format: resolved_format,
            filters,
            page_size,
            include_related,
            compress,
        },
    )
    .await?;

    println!("{}", "Export complete!".green().bold());
    println!(
        "  {}: {}",
        "Format".bold(),
        format!("{:?}", summary.format).to_lowercase()
    );
    println!("  {}: {}", "Items".bold(), summary.items_exported);
    println!("  {}: {}", "Output".bold(), summary.output_path);
    println!("  {}: {}", "SHA-256".bold(), summary.sha256.bright_black());
    println!("  {}: {}\n", "Checksum".bold(), summary.checksum_path);
    Ok(())
}
