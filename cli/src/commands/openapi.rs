//! `soroban-registry openapi` — generate an OpenAPI document from a contract ABI.

use anyhow::Result;
use colored::Colorize;
use std::fs;

/// Load ABI JSON string from WASM (soroban bindings) or from a JSON file
fn load_abi_json(contract_path: &str) -> Result<String> {
    if contract_path.to_lowercase().ends_with(".wasm") {
        let output = std::process::Command::new("soroban")
            .args(["contract", "bindings", "json", "--wasm", contract_path])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run soroban bindings: {}", e))?;
        if !output.status.success() {
            anyhow::bail!(
                "soroban bindings failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Ok(fs::read_to_string(contract_path)?)
    }
}

/// Generate markdown from ContractABI
fn abi_to_markdown(abi: &contract_abi::ContractABI) -> String {
    let mut md = format!("# {}\n\n", abi.name);
    if let Some(v) = &abi.version {
        md.push_str(&format!("Version: {}\n\n", v));
    }
    md.push_str("## Functions\n\n");
    for func in abi.public_functions() {
        md.push_str(&format!("### `{}`\n\n", func.name));
        if let Some(doc) = &func.doc {
            md.push_str(&format!("{}\n\n", doc));
        }
        md.push_str("**Parameters:**\n");
        if func.params.is_empty() {
            md.push_str("- None\n");
        } else {
            for p in &func.params {
                md.push_str(&format!(
                    "- `{}`: `{}`\n",
                    p.name,
                    p.param_type.display_name()
                ));
            }
        }
        md.push_str(&format!(
            "\n**Returns:** `{}`\n\n",
            func.return_type.display_name()
        ));
    }
    if !abi.errors.is_empty() {
        md.push_str("## Errors\n\n");
        for e in &abi.errors {
            md.push_str(&format!(
                "- **{}** (code {}): {}\n",
                e.name,
                e.code,
                e.doc.as_deref().unwrap_or("")
            ));
        }
    }
    md
}

/// Generate self-contained HTML with Swagger UI and inline OpenAPI spec (JSON)
fn openapi_to_html(spec_json: &str, title: &str) -> String {
    let spec_escaped = spec_json.replace("</script>", "<\\/script>");
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>{} - API Docs</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script type="application/json" id="openapi-spec">{}</script>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    (function() {{
      var el = document.getElementById('openapi-spec');
      try {{
        var spec = JSON.parse(el.textContent);
        SwaggerUIBundle({{ spec: spec, dom_id: '#swagger-ui' }});
      }} catch (e) {{
        document.getElementById('swagger-ui').innerHTML = '<p>Failed to load spec: ' + e.message + '</p>';
      }}
    }})();
  </script>
</body>
</html>
"#,
        title, spec_escaped
    )
}

pub fn openapi(contract_path: &str, output: &str, format: &str) -> Result<()> {
    println!("\n{}", "Generating OpenAPI documentation...".bold().cyan());
    let abi_json = load_abi_json(contract_path)?;
    let contract_name = std::path::Path::new(contract_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("contract");
    let abi = contract_abi::parse_json_spec(&abi_json, contract_name)
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI: {}", e))?;
    let content = match format.to_lowercase().as_str() {
        "yaml" | "yml" => {
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            contract_abi::to_yaml(&doc).map_err(|e| anyhow::anyhow!("{}", e))?
        }
        "json" => {
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            contract_abi::to_json(&doc).map_err(|e| anyhow::anyhow!("{}", e))?
        }
        "markdown" | "md" => abi_to_markdown(&abi),
        "html" => {
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            let json = contract_abi::to_json(&doc).map_err(|e| anyhow::anyhow!("{}", e))?;
            openapi_to_html(&json, &abi.name)
        }
        "pdf" => {
            println!("{}", "PDF: Generate YAML first, then run: npx @redocly/cli build-docs openapi.yaml -o doc.pdf".yellow());
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            let yaml = contract_abi::to_yaml(&doc).map_err(|e| anyhow::anyhow!("{}", e))?;
            let yaml_path = output.trim_end_matches(".pdf").to_string() + ".yaml";
            fs::write(&yaml_path, &yaml)?;
            println!("{} Wrote {}", "[OK]".green(), yaml_path);
            return Ok(());
        }
        _ => anyhow::bail!(
            "Unsupported format '{}'. Use: yaml, json, markdown, html, pdf",
            format
        ),
    };
    fs::write(output, content)?;
    println!("{} Documentation saved to: {}", "[OK]".green(), output);
    Ok(())
}
