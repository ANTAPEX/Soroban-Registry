use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GasEstimateApiRequest {
    method: String,
    params: Option<serde_json::Value>,
    network: Option<String>,
    source_account: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GasEstimateApiResponse {
    contract_id: String,
    method: String,
    estimated_gas: u64,
    estimated_fee_stroops: i64,
    estimated_fee_xlm: f64,
    cpu_instructions: u64,
    memory_bytes: u64,
    execution_time_ms: u64,
    cost_breakdown: CostBreakdown,
    optimization_suggestions: Vec<Suggestion>,
    accuracy: Option<Accuracy>,
}

#[derive(Debug, Deserialize)]
struct CostBreakdown {
    compute_fee: i64,
    storage_fee: i64,
    bandwidth_fee: i64,
    refundable_fee: i64,
    total_fee: i64,
}

#[derive(Debug, Deserialize)]
struct Suggestion {
    category: String,
    description: String,
    estimated_savings_percent: f64,
    priority: String,
}

#[derive(Debug, Deserialize)]
struct Accuracy {
    historical_samples: i64,
    mean_deviation_percent: f64,
    p95_deviation_percent: f64,
    within_10_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct MethodAccuracy {
    method_name: String,
    network: String,
    sample_count: i64,
    mean_deviation_percent: f64,
    within_10_percent_ratio: f64,
}

pub async fn estimate(
    api_url: &str,
    contract_id: &str,
    method: &str,
    params: Option<&str>,
    network: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let client = reqwest::Client::new();

    let params_value: Option<serde_json::Value> = params
        .map(|p| serde_json::from_str(p))
        .transpose()
        .context("Invalid JSON params")?;

    let request = GasEstimateApiRequest {
        method: method.to_string(),
        params: params_value,
        network: network.map(String::from),
        source_account: None,
    };

    let response = client
        .post(format!(
            "{}/api/contracts/{}/gas-estimate",
            api_url, contract_id
        ))
        .json(&request)
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API returned {}: {}", status, body);
    }

    let estimate: GasEstimateApiResponse = response
        .json()
        .await
        .context("Failed to parse gas estimate response")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contract_id": estimate.contract_id,
                "method": estimate.method,
                "estimated_gas": estimate.estimated_gas,
                "estimated_fee_stroops": estimate.estimated_fee_stroops,
                "estimated_fee_xlm": estimate.estimated_fee_xlm,
                "cpu_instructions": estimate.cpu_instructions,
                "memory_bytes": estimate.memory_bytes,
                "execution_time_ms": estimate.execution_time_ms,
                "cost_breakdown": {
                    "compute_fee": estimate.cost_breakdown.compute_fee,
                    "storage_fee": estimate.cost_breakdown.storage_fee,
                    "bandwidth_fee": estimate.cost_breakdown.bandwidth_fee,
                    "refundable_fee": estimate.cost_breakdown.refundable_fee,
                    "total_fee": estimate.cost_breakdown.total_fee,
                },
                "optimization_suggestions": estimate.optimization_suggestions.iter().map(|s| {
                    serde_json::json!({
                        "category": s.category,
                        "description": s.description,
                        "savings_percent": s.estimated_savings_percent,
                        "priority": s.priority,
                    })
                }).collect::<Vec<_>>(),
                "accuracy": estimate.accuracy.as_ref().map(|a| serde_json::json!({
                    "samples": a.historical_samples,
                    "mean_deviation": a.mean_deviation_percent,
                    "p95_deviation": a.p95_deviation_percent,
                    "within_10_percent": a.within_10_percent,
                })),
            }))?
        );
        return Ok(());
    }

    // Pretty-print output
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════╗"
            .bright_cyan()
    );
    println!(
        "{}",
        "║              GAS ESTIMATION REPORT                       ║"
            .bright_cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════╝"
            .bright_cyan()
    );
    println!();
    println!(
        "  Contract:  {}",
        estimate.contract_id.bright_white()
    );
    println!(
        "  Method:    {}",
        estimate.method.bright_yellow()
    );
    println!(
        "  Sim Time:  {}",
        format!("{}ms", estimate.execution_time_ms).dimmed()
    );
    println!();

    // Resource usage
    println!("{}", "  Resource Usage".underline().bright_white());
    println!(
        "    CPU Instructions:  {:>15}",
        format_number(estimate.cpu_instructions)
    );
    println!(
        "    Memory:            {:>15}",
        format_bytes(estimate.memory_bytes)
    );
    println!(
        "    Estimated Gas:     {:>15}",
        format_number(estimate.estimated_gas)
    );
    println!();

    // Cost breakdown
    println!("{}", "  Cost Breakdown".underline().bright_white());
    println!(
        "    Compute Fee:       {:>12} stroops",
        format_number(estimate.cost_breakdown.compute_fee as u64)
    );
    println!(
        "    Storage Fee:       {:>12} stroops",
        format_number(estimate.cost_breakdown.storage_fee as u64)
    );
    println!(
        "    Bandwidth Fee:     {:>12} stroops",
        format_number(estimate.cost_breakdown.bandwidth_fee as u64)
    );
    println!(
        "    Refundable Fee:    {:>12} stroops",
        format_number(estimate.cost_breakdown.refundable_fee as u64)
    );
    println!(
        "    {}",
        "─────────────────────────────────────────".dimmed()
    );
    println!(
        "    {}   {:>12} stroops",
        "Total Fee:".bright_green(),
        format_number(estimate.cost_breakdown.total_fee as u64)
    );
    println!(
        "    {}   {:>15.7} XLM",
        "Total Fee:".bright_green(),
        estimate.estimated_fee_xlm
    );
    println!();

    // Accuracy
    if let Some(ref accuracy) = estimate.accuracy {
        println!("{}", "  Estimation Accuracy".underline().bright_white());
        println!(
            "    Historical Samples:  {}",
            accuracy.historical_samples
        );
        println!(
            "    Mean Deviation:      {:.1}%",
            accuracy.mean_deviation_percent
        );
        println!(
            "    P95 Deviation:       {:.1}%",
            accuracy.p95_deviation_percent
        );
        let within_str = format!("{:.1}%", accuracy.within_10_percent);
        let colored_within = if accuracy.within_10_percent >= 90.0 {
            within_str.bright_green()
        } else if accuracy.within_10_percent >= 70.0 {
            within_str.bright_yellow()
        } else {
            within_str.bright_red()
        };
        println!("    Within 10% Target:   {}", colored_within);
        println!();
    }

    // Optimization suggestions
    if !estimate.optimization_suggestions.is_empty() {
        println!(
            "{}",
            "  Optimization Suggestions".underline().bright_white()
        );
        for (i, suggestion) in estimate.optimization_suggestions.iter().enumerate() {
            let priority_colored = match suggestion.priority.as_str() {
                "high" => format!("[{}]", suggestion.priority.to_uppercase()).bright_red(),
                "medium" => format!("[{}]", suggestion.priority.to_uppercase()).bright_yellow(),
                _ => format!("[{}]", suggestion.priority.to_uppercase()).bright_blue(),
            };
            println!(
                "    {}. {} {} (~{:.0}% savings)",
                i + 1,
                priority_colored,
                suggestion.description,
                suggestion.estimated_savings_percent
            );
        }
        println!();
    }

    Ok(())
}

pub async fn accuracy(
    api_url: &str,
    contract_id: &str,
    json_output: bool,
) -> Result<()> {
    let client = reqwest::Client::new();

    let response = client
        .get(format!(
            "{}/api/contracts/{}/gas-estimate/accuracy",
            api_url, contract_id
        ))
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API returned {}: {}", status, body);
    }

    let methods: Vec<MethodAccuracy> = response
        .json()
        .await
        .context("Failed to parse accuracy response")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&methods)?);
        return Ok(());
    }

    if methods.is_empty() {
        println!(
            "{}",
            "No accuracy data available yet. Run gas estimates and record actual values first."
                .dimmed()
        );
        return Ok(());
    }

    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════╗"
            .bright_cyan()
    );
    println!(
        "{}",
        "║           GAS ESTIMATION ACCURACY                        ║"
            .bright_cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════╝"
            .bright_cyan()
    );
    println!();
    println!(
        "  {:<25} {:<10} {:>8} {:>12} {:>12}",
        "Method".underline(),
        "Network".underline(),
        "Samples".underline(),
        "Mean Dev %".underline(),
        "Within 10%".underline()
    );

    for m in &methods {
        let within_str = format!("{:.1}%", m.within_10_percent_ratio * 100.0);
        let colored_within = if m.within_10_percent_ratio >= 0.9 {
            within_str.bright_green()
        } else if m.within_10_percent_ratio >= 0.7 {
            within_str.bright_yellow()
        } else {
            within_str.bright_red()
        };
        println!(
            "  {:<25} {:<10} {:>8} {:>11.1}% {:>12}",
            m.method_name, m.network, m.sample_count, m.mean_deviation_percent, colored_within
        );
    }
    println!();

    Ok(())
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
