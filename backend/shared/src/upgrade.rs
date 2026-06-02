use serde::{Deserialize, Serialize};

/// Minimal representation of a contract state schema for comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Schema {
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub field: Option<String>,
    pub severity: Severity,
    pub message: String,
}

impl Schema {
    /// Parse a simple JSON schema representation produced by tooling or tests.
    /// Expected format: { "fields": [ { "name": "count", "type": "u64" }, ... ] }
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Public entry: compare two schemas and return findings. Conservative: when unsure, emit Warning.
pub fn compare_schemas(old: &Schema, new: &Schema) -> Vec<Finding> {
    run_validation_rules(old, new)
}

// -----------------------------------------------------------------------------
// Declarative validation rule engine
// -----------------------------------------------------------------------------
pub type RuleId = &'static str;

pub trait Rule: Send + Sync {
    fn id(&self) -> RuleId;
    fn description(&self) -> &'static str;
    fn evaluate(&self, old: &Schema, new: &Schema) -> Vec<Finding>;
}

fn run_validation_rules(old: &Schema, new: &Schema) -> Vec<Finding> {
    let mut findings = Vec::new();
    // Built-in rules - engine is extensible: add new boxed rules here.
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(NoFieldRemovalRule {}),
        Box::new(TypeCompatibilityRule {}),
    ];

    for r in rules {
        let mut res = r.evaluate(old, new);
        findings.append(&mut res);
    }

    // Include additions as Info
    let old_names: std::collections::HashSet<_> =
        old.fields.iter().map(|f| f.name.as_str()).collect();
    for f in &new.fields {
        if !old_names.contains(f.name.as_str()) {
            findings.push(Finding {
                field: Some(f.name.clone()),
                severity: Severity::Info,
                message: format!("Field '{}' was added in the new schema.", f.name),
            });
        }
    }

    findings
}

// Rule: disallow field removals (Error)
struct NoFieldRemovalRule {}
impl Rule for NoFieldRemovalRule {
    fn id(&self) -> RuleId {
        "no_field_removal"
    }

    fn description(&self) -> &'static str {
        "Detects removal of fields from the state schema which is always a breaking change."
    }

    fn evaluate(&self, old: &Schema, new: &Schema) -> Vec<Finding> {
        let mut findings = Vec::new();
        let new_names: std::collections::HashSet<_> =
            new.fields.iter().map(|f| f.name.as_str()).collect();
        for f in &old.fields {
            if !new_names.contains(f.name.as_str()) {
                findings.push(Finding {
                    field: Some(f.name.clone()),
                    severity: Severity::Error,
                    message: format!("Field '{}' was removed from new schema", f.name),
                });
            }
        }
        findings
    }
}

// Rule: detect type changes and mark as Warning unless clearly compatible
struct TypeCompatibilityRule {}
impl Rule for TypeCompatibilityRule {
    fn id(&self) -> RuleId {
        "type_compatibility"
    }

    fn description(&self) -> &'static str {
        "Detect incompatible or suspicious type changes between schemas."
    }

    fn evaluate(&self, old: &Schema, new: &Schema) -> Vec<Finding> {
        let mut findings = Vec::new();
        let new_map: std::collections::HashMap<_, _> = new
            .fields
            .iter()
            .map(|f| (f.name.as_str(), &f.type_name))
            .collect();
        for f in &old.fields {
            if let Some(n_ty) = new_map.get(f.name.as_str()) {
                if n_ty.as_str() != f.type_name.as_str() {
                    // Conservative: flag as Warning
                    findings.push(Finding {
                        field: Some(f.name.clone()),
                        severity: Severity::Warning,
                        message: format!(
                            "Field '{}' changed type from '{}' to '{}'",
                            f.name, f.type_name, n_ty
                        ),
                    });
                }
            }
        }
        findings
    }
}
