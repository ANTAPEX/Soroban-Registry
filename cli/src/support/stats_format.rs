//! Rendering registry statistics as a table or as CSV.

use anyhow::Result;
use colored::Colorize;

pub(crate) fn format_stats_table(stats: &serde_json::Value) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "\n{}",
        "Soroban Registry Statistics".bold().cyan()
    ));
    out.push_str(&format!("\n{}\n", "=".repeat(60).cyan()));

    // Basic counts
    if let Some(total) = stats["total_contracts"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Total Contracts", &total.to_string())
        ));
    }
    if let Some(publishers) = stats["total_publishers"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Total Publishers", &publishers.to_string())
        ));
    }
    if let Some(verified) = stats["verified_contracts"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Verified Contracts", &verified.to_string())
        ));
    }
    if let Some(pct) = stats["verification_percentage"].as_f64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Verification Rate", &format!("{:.1}%", pct))
        ));
    }
    out.push_str("\n");

    // Growth
    out.push_str(&format!("{}", "Growth".bold()));
    out.push_str(&format!("\n{}\n", "─".repeat(40).bright_black()));
    if let Some(c7) = stats["contracts_last_7d"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("  Last 7 days", &c7.to_string())
        ));
    }
    if let Some(c30) = stats["contracts_last_30d"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("  Last 30 days", &c30.to_string())
        ));
    }
    if let Some(p30) = stats["new_publishers_last_30d"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("  New publishers (30d)", &p30.to_string())
        ));
    }
    out.push_str("\n");

    // Top contracts
    if let Some(top) = stats["top_contracts"].as_array() {
        out.push_str(&format!("{}", "Top 10 Contracts by Interactions".bold()));
        out.push_str(&format!("\n{}\n", "─".repeat(40).bright_black()));
        for (i, contract) in top.iter().enumerate().take(10) {
            let name = contract["name"].as_str().unwrap_or("N/A");
            let count = contract["interaction_count"].as_i64().unwrap_or(0);
            out.push_str(&format!(
                "  {}. {} ({})\n",
                (i + 1).to_string().bright_blue(),
                name.bold(),
                count.to_string().green()
            ));
        }
        out.push_str("\n");
    }

    // Network breakdown
    if let Some(networks) = stats["network_stats"].as_array() {
        out.push_str(&format!("{}", "By Network".bold()));
        out.push_str(&format!("\n{}\n", "─".repeat(40).bright_black()));
        for net in networks {
            let n = match net["network"].as_str() {
                Some("mainnet") => "Mainnet".cyan(),
                Some("testnet") => "Testnet".yellow(),
                Some("futurenet") => "Futurenet".magenta(),
                _ => net["network"].as_str().unwrap_or("").into(),
            };
            let count = net["contract_count"].as_i64().unwrap_or(0);
            out.push_str(&format!("  {}: {} contracts\n", n, count));
        }
        out.push_str("\n");
    }

    // Generated at
    if let Some(gen) = stats["generated_at"].as_str() {
        out.push_str(&format!("Generated at: {}\n", gen.bright_black()));
    }

    out
}

pub(crate) fn format_stats_csv(stats: &serde_json::Value) -> Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(["metric", "scope", "value"])?;

    for key in [
        "total_contracts",
        "total_publishers",
        "verified_contracts",
        "verification_percentage",
        "contracts_last_24h",
        "contracts_last_7d",
        "contracts_last_30d",
        "new_publishers_last_30d",
    ] {
        if let Some(value) = stats.get(key) {
            writer.write_record(["summary", key, &value.to_string()])?;
        }
    }

    if let Some(networks) = stats
        .get("network_stats")
        .and_then(serde_json::Value::as_array)
    {
        for network in networks {
            writer.write_record([
                "network",
                network
                    .get("network")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown"),
                &network
                    .get("contract_count")
                    .map(serde_json::Value::to_string)
                    .unwrap_or_else(|| "0".to_string()),
            ])?;
        }
    }

    if let Some(categories) = stats
        .get("category_stats")
        .and_then(serde_json::Value::as_array)
    {
        for category in categories {
            writer.write_record([
                "category",
                category
                    .get("category")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("uncategorized"),
                &category
                    .get("contract_count")
                    .map(serde_json::Value::to_string)
                    .unwrap_or_else(|| "0".to_string()),
            ])?;
        }
    }

    if let Some(top) = stats
        .get("top_contracts")
        .and_then(serde_json::Value::as_array)
    {
        for contract in top {
            writer.write_record([
                "top_contract",
                contract
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        contract
                            .get("contract_id")
                            .and_then(serde_json::Value::as_str)
                    })
                    .unwrap_or("unknown"),
                &contract
                    .get("interaction_count")
                    .map(serde_json::Value::to_string)
                    .unwrap_or_else(|| "0".to_string()),
            ])?;
        }
    }

    let bytes = writer.into_inner()?;
    Ok(String::from_utf8(bytes)?)
}

fn format_kv(key: &str, value: &str) -> String {
    format!("  {} {}", key.bold().cyan(), value.bright_white())
}
