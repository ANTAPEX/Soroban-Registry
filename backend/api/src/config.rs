use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FeatureFlagEntry {
    pub key: String,
    pub is_enabled: bool,
    pub rollout_percentage: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub elasticsearch_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub host: String,
    pub log_level: String,
    #[serde(default = "default_cache_url")]
    pub redis_url: String,
    /// JSON-encoded feature flags configuration.
    /// Format: [{"key":"flag_name","is_enabled":true,"rollout_percentage":100}]
    #[serde(default)]
    pub feature_flags_json: String,
}

fn default_cache_url() -> String {
    "redis://localhost:6379".to_string()
}

pub fn load_config() -> Result<AppConfig> {
    dotenv::dotenv().ok();

    let config = envy::from_env::<AppConfig>()
        .context("Failed to load configuration from environment variables")?;

    validate_config(&config)?;

    Ok(config)
}

pub fn parse_feature_flags(json_str: &str) -> Vec<FeatureFlagEntry> {
    if json_str.is_empty() {
        return Vec::new();
    }
    serde_json::from_str(json_str).unwrap_or_else(|e| {
        tracing::warn!("Failed to parse FEATURE_FLAGS_JSON: {}. Using defaults.", e);
        Vec::new()
    })
}

fn validate_config(config: &AppConfig) -> Result<()> {
    if config.jwt_secret.len() < 32 {
        anyhow::bail!("JWT_SECRET must be at least 32 characters long for security");
    }

    if !config.database_url.starts_with("postgres://")
        && !config.database_url.starts_with("postgresql://")
    {
        anyhow::bail!("DATABASE_URL must be a valid postgres connection string");
    }

    Ok(())
}
