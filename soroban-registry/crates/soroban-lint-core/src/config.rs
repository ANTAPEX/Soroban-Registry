use crate::diagnostic::Severity;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuration for soroban-lint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    pub lint: LintOptions,
    pub rules: Option<HashMap<String, String>>,
    pub ignore: Option<IgnoreOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintOptions {
    pub level: String, // "info", "warning", "error"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreOptions {
    pub paths: Option<Vec<String>>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            lint: LintOptions {
                level: "warning".to_string(),
            },
            rules: None,
            ignore: None,
        }
    }
}

impl LintConfig {
    /// Load config from file or use defaults
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        if let Some(path) = config_path {
            let content = fs::read_to_string(path)
                .context(format!("Failed to read config file: {}", path))?;
            toml::from_str(&content).context("Failed to parse config file")
        } else {
            // Try to find config in current directory
            if Path::new(".soroban-lint.toml").exists() {
                let content = fs::read_to_string(".soroban-lint.toml")?;
                toml::from_str(&content).context("Failed to parse .soroban-lint.toml")
            } else {
                Ok(Self::default())
            }
        }
    }

    /// Get the minimum severity level from config
    pub fn min_severity(&self) -> Severity {
        Severity::parse(&self.lint.level).unwrap_or(Severity::Warning)
    }

    /// Get rule-specific severity override
    pub fn rule_severity(&self, rule_id: &str) -> Option<Severity> {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get(rule_id).and_then(|s| Severity::parse(s)))
    }

    /// Check if a path should be ignored
    pub fn should_ignore(&self, path: &str) -> bool {
        if let Some(ignore) = &self.ignore {
            if let Some(paths) = &ignore.paths {
                return paths.iter().any(|p| {
                    path.contains(p.replace("\\", "/").as_str()) || path.contains(p.as_str())
                });
            }
        }
        false
    }

    /// Save config to file
    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Get or create default config in current directory
pub fn get_or_create_default_config() -> Result<LintConfig> {
    let config_path = ".soroban-lint.toml";
    if !Path::new(config_path).exists() {
        let config = LintConfig::default();
        config.save(config_path)?;
    }
    LintConfig::load(Some(config_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LintConfig::default();
        assert_eq!(config.lint.level, "warning");
        assert_eq!(config.min_severity(), Severity::Warning);
    }

    #[test]
    fn test_severity_from_str() {
        assert_eq!(Severity::parse("error"), Some(Severity::Error));
        assert_eq!(Severity::parse("warning"), Some(Severity::Warning));
        assert_eq!(Severity::parse("info"), Some(Severity::Info));
        assert_eq!(Severity::parse("invalid"), None);
    }

    #[test]
    fn test_ignore_paths() {
        let config = LintConfig {
            ignore: Some(IgnoreOptions {
                paths: Some(vec!["tests/".to_string(), "examples/".to_string()]),
            }),
            ..LintConfig::default()
        };
        assert!(config.should_ignore("tests/file.rs"));
        assert!(config.should_ignore("examples/demo.rs"));
        assert!(!config.should_ignore("src/main.rs"));
    }
}
