use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A single operation to perform on a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub enum BatchOperation {
    Publish {
        contract_id: String,
        wasm_path: String,
        #[serde(default)]
        network: Option<String>,
    },
    Verify {
        contract_id: String,
        #[serde(default)]
        expected_hash: Option<String>,
    },
    UpdateMetadata {
        contract_id: String,
        metadata: serde_json::Value,
    },
    SetNetwork {
        contract_id: String,
        network: String,
    },
    Retire {
        contract_id: String,
        #[serde(default)]
        reason: Option<String>,
    },
}

impl BatchOperation {
    /// Returns a human-readable label for reporting.
    pub fn label(&self) -> String {
        match self {
            BatchOperation::Publish { contract_id, .. } => {
                format!("publish({})", contract_id)
            }
            BatchOperation::Verify { contract_id, .. } => {
                format!("verify({})", contract_id)
            }
            BatchOperation::UpdateMetadata { contract_id, .. } => {
                format!("update-metadata({})", contract_id)
            }
            BatchOperation::SetNetwork { contract_id, .. } => {
                format!("set-network({})", contract_id)
            }
            BatchOperation::Retire { contract_id, .. } => {
                format!("retire({})", contract_id)
            }
        }
    }

    /// Returns the contract_id associated with the operation.
    pub fn contract_id(&self) -> &str {
        match self {
            BatchOperation::Publish { contract_id, .. }
            | BatchOperation::Verify { contract_id, .. }
            | BatchOperation::UpdateMetadata { contract_id, .. }
            | BatchOperation::SetNetwork { contract_id, .. }
            | BatchOperation::Retire { contract_id, .. } => contract_id,
        }
    }
}

/// The top-level batch manifest file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchManifest {
    /// Optional human-readable name for this batch.
    #[serde(default)]
    pub name: Option<String>,

    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,

    /// The list of operations to execute atomically.
    pub operations: Vec<BatchOperation>,
}

impl BatchManifest {
    /// Load and parse a manifest from a JSON or YAML file.
    /// Detects format by file extension.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest: {}", path.display()))?;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let manifest: BatchManifest = match ext {
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .with_context(|| "Failed to parse YAML manifest")?,
            "json" => serde_json::from_str(&content)
                .with_context(|| "Failed to parse JSON manifest")?,
            _ => {
                // Try YAML first, fall back to JSON
                serde_yaml::from_str(&content)
                    .or_else(|_| serde_json::from_str(&content))
                    .with_context(|| "Failed to parse manifest (tried YAML and JSON)")?
            }
        };

        Ok(manifest)
    }

    /// Validate the manifest before execution.
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings: Vec<String> = Vec::new();

        if self.operations.is_empty() {
            anyhow::bail!("Manifest contains no operations");
        }

        for (i, op) in self.operations.iter().enumerate() {
            let cid = op.contract_id();
            if cid.trim().is_empty() {
                anyhow::bail!("Operation {} has an empty contract_id", i + 1);
            }

            // Operation-specific validation
            match op {
                BatchOperation::Publish { wasm_path, .. } => {
                    if wasm_path.trim().is_empty() {
                        anyhow::bail!(
                            "Operation {} (publish): wasm_path cannot be empty",
                            i + 1
                        );
                    }
                    let wasm = Path::new(wasm_path);
                    if !wasm.exists() {
                        warnings.push(format!(
                            "Operation {} (publish): wasm file '{}' not found (may be created before execution)",
                            i + 1,
                            wasm_path
                        ));
                    }
                }
                BatchOperation::SetNetwork { network, .. } => {
                    if network.trim().is_empty() {
                        anyhow::bail!(
                            "Operation {} (set-network): network cannot be empty",
                            i + 1
                        );
                    }
                }
                _ => {}
            }
        }

        Ok(warnings)
    }
}