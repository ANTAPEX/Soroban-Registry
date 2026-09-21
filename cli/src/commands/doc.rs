//! `soroban-registry doc` — generate contract documentation.

use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn doc(contract_path: &str, output: &str) -> Result<()> {
    println!("\n{}", "Generating contract documentation...".bold().cyan());

    let content = format!(
        r#"# Contract Documentation

## Contract Path
{}

## Generated
{}

*This is a placeholder. Full documentation generation coming soon.*
"#,
        contract_path,
        chrono::Utc::now().to_rfc3339()
    );

    fs::write(output, content)?;
    println!("{} Documentation saved to: {}", "[OK]".green(), output);

    Ok(())
}
