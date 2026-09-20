use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContractSnapshot {
    contract_id: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    schema: BTreeMap<String, String>,
    #[serde(default)]
    state: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemaDiff {
    added_fields: Vec<String>,
    removed_fields: Vec<String>,
    changed_types: Vec<TypeChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypeChange {
    field: String,
    old_type: String,
    new_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationRecord {
    id: String,
    action: String,
    timestamp: String,
    status: String,
    old_id: Option<String>,
    new_id: Option<String>,
    diff: Option<SchemaDiff>,
    warnings: Vec<String>,
    before_state: Option<Value>,
    after_state: Option<Value>,
    backup_old_snapshot: Option<ContractSnapshot>,
    backup_new_snapshot: Option<ContractSnapshot>,
}

pub fn preview(old_id: &str, new_id: &str) -> Result<()> {
    let old_snapshot = load_snapshot(old_id)?;
    let new_snapshot = load_snapshot(new_id)?;

    let diff = analyze_internal(&old_snapshot, &new_snapshot);
    let issues = validate_internal(&old_snapshot, &new_snapshot, &diff);
    let (migrated, dry_run_warnings) = dry_run_internal(&old_snapshot, &new_snapshot, &diff);

    print_diff(old_id, new_id, &diff);
    print_validation(&issues);

    println!("\n{}", "Dry-run Migrated State".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!(
        "{}",
        serde_json::to_string_pretty(&Value::Object(migrated.clone()))?
    );

    if !dry_run_warnings.is_empty() {
        println!("\n{}", "Dry-run Notes".bold().yellow());
        for warning in &dry_run_warnings {
            println!("- {}", warning);
        }
    }

    append_history(MigrationRecord {
        id: Uuid::new_v4().to_string(),
        action: "preview".to_string(),
        timestamp: Utc::now().to_rfc3339(),
        status: "success".to_string(),
        old_id: Some(old_id.to_string()),
        new_id: Some(new_id.to_string()),
        diff: Some(diff),
        warnings: issues.into_iter().chain(dry_run_warnings).collect(),
        before_state: Some(Value::Object(old_snapshot.state)),
        after_state: Some(Value::Object(migrated)),
        backup_old_snapshot: None,
        backup_new_snapshot: None,
    })?;

    Ok(())
}

pub fn analyze(old_id: &str, new_id: &str) -> Result<()> {
    let old_snapshot = load_snapshot(old_id)?;
    let new_snapshot = load_snapshot(new_id)?;
    let diff = analyze_internal(&old_snapshot, &new_snapshot);
    print_diff(old_id, new_id, &diff);
    Ok(())
}

pub fn generate_template(
    old_id: &str,
    new_id: &str,
    language: &str,
    output: Option<&str>,
) -> Result<()> {
    let old_snapshot = load_snapshot(old_id)?;
    let new_snapshot = load_snapshot(new_id)?;
    let diff = analyze_internal(&old_snapshot, &new_snapshot);

    let (extension, template) = match language.to_ascii_lowercase().as_str() {
        "rust" | "rs" => ("rs", rust_template(old_id, new_id, &diff)),
        "js" | "javascript" => ("js", js_template(old_id, new_id, &diff)),
        _ => bail!("Unsupported language '{}'. Use rust or js.", language),
    };

    let default_name = format!(
        "migration_{}_to_{}.{}",
        slug(old_id),
        slug(new_id),
        extension
    );
    let output_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_name));

    fs::write(&output_path, template).with_context(|| {
        format!(
            "Failed to write migration template to {}",
            output_path.display()
        )
    })?;

    println!(
        "{} {}",
        "Generated migration template:".green().bold(),
        output_path.display()
    );
    Ok(())
}

pub fn validate(old_id: &str, new_id: &str) -> Result<()> {
    let old_snapshot = load_snapshot(old_id)?;
    let new_snapshot = load_snapshot(new_id)?;
    let diff = analyze_internal(&old_snapshot, &new_snapshot);
    let issues = validate_internal(&old_snapshot, &new_snapshot, &diff);
    print_validation(&issues);

    if issues.is_empty() {
        Ok(())
    } else {
        bail!("Validation found potential data loss or type incompatibilities")
    }
}

pub fn apply(old_id: &str, new_id: &str) -> Result<()> {
    let old_snapshot = load_snapshot(old_id)?;
    let mut new_snapshot = load_snapshot(new_id)?;
    let diff = analyze_internal(&old_snapshot, &new_snapshot);
    let issues = validate_internal(&old_snapshot, &new_snapshot, &diff);
    if !issues.is_empty() {
        for issue in &issues {
            eprintln!("{} {}", "Validation issue:".red().bold(), issue);
        }
        bail!("Migration aborted due to validation issues")
    }

    let (migrated_state, warnings) = dry_run_internal(&old_snapshot, &new_snapshot, &diff);
    let new_snapshot_path = snapshot_path(new_id);
    let previous_new_snapshot = if new_snapshot_path.exists() {
        Some(load_snapshot(new_id)?)
    } else {
        None
    };

    new_snapshot.state = migrated_state.clone();
    persist_snapshot(&new_snapshot)?;

    let migration_id = Uuid::new_v4().to_string();
    append_history(MigrationRecord {
        id: migration_id.clone(),
        action: "apply".to_string(),
        timestamp: Utc::now().to_rfc3339(),
        status: "success".to_string(),
        old_id: Some(old_id.to_string()),
        new_id: Some(new_id.to_string()),
        diff: Some(diff),
        warnings,
        before_state: Some(Value::Object(old_snapshot.state.clone())),
        after_state: Some(Value::Object(migrated_state)),
        backup_old_snapshot: Some(old_snapshot),
        backup_new_snapshot: previous_new_snapshot,
    })?;

    println!(
        "{} {}",
        "Migration applied successfully. ID:".green().bold(),
        migration_id
    );
    Ok(())
}

pub fn rollback(migration_id: &str) -> Result<()> {
    let records = read_history()?;
    let record = records
        .into_iter()
        .rev()
        .find(|r| r.id == migration_id && r.action == "apply" && r.status == "success")
        .ok_or_else(|| anyhow!("Apply migration record not found for id {}", migration_id))?;

    let old_snapshot = record
        .backup_old_snapshot
        .ok_or_else(|| anyhow!("Rollback metadata missing old snapshot"))?;
    let old_id = old_snapshot.contract_id.clone();
    persist_snapshot(&old_snapshot)?;

    if let Some(new_id) = record.new_id {
        if let Some(previous_new) = record.backup_new_snapshot {
            persist_snapshot(&previous_new)?;
        } else {
            let new_path = snapshot_path(&new_id);
            if new_path.exists() {
                fs::remove_file(&new_path)
                    .with_context(|| format!("Failed to remove {}", new_path.display()))?;
            }
        }
    }

    append_history(MigrationRecord {
        id: Uuid::new_v4().to_string(),
        action: "rollback".to_string(),
        timestamp: Utc::now().to_rfc3339(),
        status: "success".to_string(),
        old_id: Some(old_id),
        new_id: None,
        diff: None,
        warnings: vec![format!("Rolled back migration {}", migration_id)],
        before_state: None,
        after_state: None,
        backup_old_snapshot: None,
        backup_new_snapshot: None,
    })?;

    println!(
        "{} {}",
        "Rollback completed for migration:".green().bold(),
        migration_id
    );
    Ok(())
}

pub fn history(limit: usize) -> Result<()> {
    let records = read_history()?;
    println!("\n{}", "Migration History".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    for record in records.iter().rev().take(limit) {
        println!(
            "{} | {} | {} | {} -> {}",
            record.timestamp,
            record.id,
            record.action,
            record.old_id.as_deref().unwrap_or("-"),
            record.new_id.as_deref().unwrap_or("-")
        );
        if !record.warnings.is_empty() {
            println!("  warnings: {}", record.warnings.join(" | "));
        }
    }

    Ok(())
}

fn analyze_internal(
    old_snapshot: &ContractSnapshot,
    new_snapshot: &ContractSnapshot,
) -> SchemaDiff {
    let mut added_fields = Vec::new();
    let mut removed_fields = Vec::new();
    let mut changed_types = Vec::new();

    for (field, new_ty) in &new_snapshot.schema {
        match old_snapshot.schema.get(field) {
            Some(old_ty) if old_ty != new_ty => changed_types.push(TypeChange {
                field: field.clone(),
                old_type: old_ty.clone(),
                new_type: new_ty.clone(),
            }),
            None => added_fields.push(field.clone()),
            _ => {}
        }
    }

    for field in old_snapshot.schema.keys() {
        if !new_snapshot.schema.contains_key(field) {
            removed_fields.push(field.clone());
        }
    }

    SchemaDiff {
        added_fields,
        removed_fields,
        changed_types,
    }
}

fn validate_internal(
    old_snapshot: &ContractSnapshot,
    new_snapshot: &ContractSnapshot,
    diff: &SchemaDiff,
) -> Vec<String> {
    let mut issues = Vec::new();

    for field in &diff.removed_fields {
        if let Some(value) = old_snapshot.state.get(field) {
            if !value.is_null() {
                issues.push(format!(
                    "Field '{}' is removed but currently contains data; migration would drop value {}",
                    field, value
                ));
            }
        }
    }

    for change in &diff.changed_types {
        if let Some(value) = old_snapshot.state.get(&change.field) {
            if convert_value(value, &change.new_type).is_none() {
                issues.push(format!(
                    "Field '{}' type change {} -> {} is not safely convertible for value {}",
                    change.field, change.old_type, change.new_type, value
                ));
            }
        }
    }

    for (field, new_ty) in &new_snapshot.schema {
        if let Some(value) = old_snapshot.state.get(field) {
            if convert_value(value, new_ty).is_none() {
                issues.push(format!(
                    "Field '{}' cannot be represented as target type '{}'",
                    field, new_ty
                ));
            }
        }
    }

    issues
}

fn dry_run_internal(
    old_snapshot: &ContractSnapshot,
    new_snapshot: &ContractSnapshot,
    diff: &SchemaDiff,
) -> (Map<String, Value>, Vec<String>) {
    let mut migrated = Map::new();
    let mut warnings = Vec::new();

    for (field, new_ty) in &new_snapshot.schema {
        let value = match old_snapshot.state.get(field) {
            Some(existing) => match convert_value(existing, new_ty) {
                Some(converted) => converted,
                None => {
                    warnings.push(format!(
                        "Field '{}' could not convert to '{}'; using default value",
                        field, new_ty
                    ));
                    default_for_type(new_ty)
                }
            },
            None => default_for_type(new_ty),
        };

        migrated.insert(field.clone(), value);
    }

    for field in &diff.removed_fields {
        if old_snapshot.state.contains_key(field) {
            warnings.push(format!(
                "Field '{}' removed in new schema and omitted from migrated state",
                field
            ));
        }
    }

    (migrated, warnings)
}

fn convert_value(value: &Value, target_type: &str) -> Option<Value> {
    match normalize_type(target_type).as_str() {
        "string" => Some(Value::String(match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })),
        "number" | "float" => match value {
            Value::Number(_) => Some(value.clone()),
            Value::String(s) => s
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number),
            Value::Bool(b) => Some(Value::Number(serde_json::Number::from(if *b {
                1
            } else {
                0
            }))),
            _ => None,
        },
        "integer" | "int" => match value {
            Value::Number(n) => n
                .as_i64()
                .map(|i| Value::Number(serde_json::Number::from(i))),
            Value::String(s) => s
                .parse::<i64>()
                .ok()
                .map(|i| Value::Number(serde_json::Number::from(i))),
            Value::Bool(b) => Some(Value::Number(serde_json::Number::from(if *b {
                1
            } else {
                0
            }))),
            _ => None,
        },
        "boolean" | "bool" => match value {
            Value::Bool(_) => Some(value.clone()),
            Value::Number(n) => n.as_i64().map(|i| Value::Bool(i != 0)),
            Value::String(s) => match s.to_ascii_lowercase().as_str() {
                "true" | "1" => Some(Value::Bool(true)),
                "false" | "0" => Some(Value::Bool(false)),
                _ => None,
            },
            _ => None,
        },
        "array" => value.as_array().map(|_| value.clone()),
        "object" | "map" => value.as_object().map(|_| value.clone()),
        _ => Some(value.clone()),
    }
}

fn default_for_type(target_type: &str) -> Value {
    match normalize_type(target_type).as_str() {
        "string" => Value::String(String::new()),
        "number" | "float" => Value::Number(serde_json::Number::from(0)),
        "integer" | "int" => Value::Number(serde_json::Number::from(0)),
        "boolean" | "bool" => Value::Bool(false),
        "array" => Value::Array(Vec::new()),
        "object" | "map" => Value::Object(Map::new()),
        _ => Value::Null,
    }
}

fn normalize_type(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn print_diff(old_id: &str, new_id: &str, diff: &SchemaDiff) {
    println!(
        "\n{} {} -> {}",
        "Schema Diff".bold().cyan(),
        old_id.bright_blue(),
        new_id.bright_blue()
    );
    println!("{}", "=".repeat(80).cyan());

    println!("Added fields: {}", diff.added_fields.len());
    for field in &diff.added_fields {
        println!("  + {}", field.green());
    }

    println!("Removed fields: {}", diff.removed_fields.len());
    for field in &diff.removed_fields {
        println!("  - {}", field.red());
    }

    println!("Type changes: {}", diff.changed_types.len());
    for change in &diff.changed_types {
        println!(
            "  ~ {}: {} -> {}",
            change.field.yellow(),
            change.old_type,
            change.new_type
        );
    }
}

fn print_validation(issues: &[String]) {
    println!("\n{}", "Validation".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if issues.is_empty() {
        println!("{}", "No data loss risks detected.".green().bold());
        return;
    }

    println!("{}", "Potential migration risks:".red().bold());
    for issue in issues {
        println!("- {}", issue);
    }
}

fn rust_template(old_id: &str, new_id: &str, diff: &SchemaDiff) -> String {
    let mut lines = vec![
        format!(
            "// Auto-generated migration template: {} -> {}",
            old_id, new_id
        ),
        "use serde_json::{Map, Value};".to_string(),
        "".to_string(),
        "pub fn migrate_state(old_state: &Map<String, Value>) -> Map<String, Value> {".to_string(),
        "    let mut new_state = Map::new();".to_string(),
        "".to_string(),
    ];

    for field in &diff.added_fields {
        lines.push(format!(
            "    // TODO: initialize added field '{}' with the correct default",
            field
        ));
        lines.push(format!(
            "    new_state.insert(\"{}\".to_string(), Value::Null);",
            field
        ));
        lines.push(String::new());
    }

    for change in &diff.changed_types {
        lines.push(format!(
            "    // TODO: transform '{}' from {} to {}",
            change.field, change.old_type, change.new_type
        ));
        lines.push(format!(
            "    if let Some(value) = old_state.get(\"{}\") {{ new_state.insert(\"{}\".to_string(), value.clone()); }}",
            change.field, change.field
        ));
        lines.push(String::new());
    }

    for field in &diff.removed_fields {
        lines.push(format!(
            "    // Removed field '{}' intentionally omitted from new state",
            field
        ));
    }

    lines.push("    new_state".to_string());
    lines.push("}".to_string());

    lines.join("\n")
}

fn js_template(old_id: &str, new_id: &str, diff: &SchemaDiff) -> String {
    let mut lines = vec![
        format!(
            "// Auto-generated migration template: {} -> {}",
            old_id, new_id
        ),
        "function migrateState(oldState) {".to_string(),
        "  const newState = {};".to_string(),
        "".to_string(),
    ];

    for field in &diff.added_fields {
        lines.push(format!(
            "  // TODO: initialize added field '{}' with the correct default",
            field
        ));
        lines.push(format!("  newState[\"{}\"] = null;", field));
        lines.push(String::new());
    }

    for change in &diff.changed_types {
        lines.push(format!(
            "  // TODO: transform '{}' from {} to {}",
            change.field, change.old_type, change.new_type
        ));
        lines.push(format!(
            "  if (Object.prototype.hasOwnProperty.call(oldState, \"{}\")) newState[\"{}\"] = oldState[\"{}\"];",
            change.field, change.field, change.field
        ));
        lines.push(String::new());
    }

    for field in &diff.removed_fields {
        lines.push(format!(
            "  // Removed field '{}' intentionally omitted from new state",
            field
        ));
    }

    lines.push("  return newState;".to_string());
    lines.push("}".to_string());
    lines.push(String::new());
    lines.push("module.exports = { migrateState };".to_string());

    lines.join("\n")
}

fn base_dir() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;
    let base = cwd.join(".soroban-registry");
    fs::create_dir_all(base.join("contracts"))
        .context("Failed to create migration data directory")?;
    Ok(base)
}

fn snapshot_path(contract_id: &str) -> PathBuf {
    Path::new(".soroban-registry")
        .join("contracts")
        .join(format!("{}.json", contract_id))
}

fn load_snapshot(contract_id: &str) -> Result<ContractSnapshot> {
    let path = snapshot_path(contract_id);
    let data = fs::read_to_string(&path).with_context(|| {
        format!(
            "Contract snapshot not found: {}. Create it at .soroban-registry/contracts/{}.json",
            path.display(),
            contract_id
        )
    })?;

    let mut snapshot: ContractSnapshot = serde_json::from_str(&data)
        .with_context(|| format!("Invalid snapshot JSON: {}", path.display()))?;

    if snapshot.contract_id.trim().is_empty() {
        snapshot.contract_id = contract_id.to_string();
    }

    Ok(snapshot)
}

fn persist_snapshot(snapshot: &ContractSnapshot) -> Result<()> {
    let base = base_dir()?;
    let path = base
        .join("contracts")
        .join(format!("{}.json", snapshot.contract_id));

    fs::write(&path, serde_json::to_string_pretty(snapshot)?)
        .with_context(|| format!("Failed to persist snapshot {}", path.display()))?;

    Ok(())
}

fn history_path() -> Result<PathBuf> {
    Ok(base_dir()?.join("migration_history.jsonl"))
}

fn append_history(record: MigrationRecord) -> Result<()> {
    let path = history_path()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Failed to open history file {}", path.display()))?;

    writeln!(file, "{}", serde_json::to_string(&record)?)
        .with_context(|| format!("Failed to append history record to {}", path.display()))?;

    Ok(())
}

fn read_history() -> Result<Vec<MigrationRecord>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path)
        .with_context(|| format!("Failed to open history file {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut records = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: MigrationRecord = serde_json::from_str(&line)
            .with_context(|| "Failed to parse migration history line")?;
        records.push(record);
    }

    Ok(records)
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_schema_changes() {
        let old = ContractSnapshot {
            contract_id: "old".to_string(),
            version: None,
            schema: BTreeMap::from([
                ("owner".to_string(), "string".to_string()),
                ("balance".to_string(), "number".to_string()),
            ]),
            state: Map::new(),
        };
        let new = ContractSnapshot {
            contract_id: "new".to_string(),
            version: None,
            schema: BTreeMap::from([
                ("owner".to_string(), "string".to_string()),
                ("balance".to_string(), "string".to_string()),
                ("nonce".to_string(), "integer".to_string()),
            ]),
            state: Map::new(),
        };

        let diff = analyze_internal(&old, &new);
        assert_eq!(diff.added_fields, vec!["nonce"]);
        assert!(diff.removed_fields.is_empty());
        assert_eq!(diff.changed_types.len(), 1);
        assert_eq!(diff.changed_types[0].field, "balance");
    }

    #[test]
    fn dry_run_maps_state() {
        let old = ContractSnapshot {
            contract_id: "old".to_string(),
            version: None,
            schema: BTreeMap::from([
                ("owner".to_string(), "string".to_string()),
                ("count".to_string(), "number".to_string()),
            ]),
            state: [
                ("owner".to_string(), Value::String("alice".to_string())),
                (
                    "count".to_string(),
                    Value::Number(serde_json::Number::from(3)),
                ),
            ]
            .into_iter()
            .collect(),
        };
        let new = ContractSnapshot {
            contract_id: "new".to_string(),
            version: None,
            schema: BTreeMap::from([
                ("owner".to_string(), "string".to_string()),
                ("count".to_string(), "string".to_string()),
                ("active".to_string(), "boolean".to_string()),
            ]),
            state: Map::new(),
        };

        let diff = analyze_internal(&old, &new);
        let (migrated, _warnings) = dry_run_internal(&old, &new, &diff);

        assert_eq!(
            migrated.get("owner").unwrap(),
            &Value::String("alice".to_string())
        );
        assert_eq!(
            migrated.get("count").unwrap(),
            &Value::String("3".to_string())
        );
        assert_eq!(migrated.get("active").unwrap(), &Value::Bool(false));
    }
}
