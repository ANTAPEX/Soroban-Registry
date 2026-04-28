use serde::Deserialize;
use std::env;
use anyhow::{Context, Result};

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

fn validate_config(config: &AppConfig) -> Result<()> {
    if config.jwt_secret.len() < 32 {
        anyhow::bail!("JWT_SECRET must be at least 32 characters long for security");
    }

    if !config.database_url.starts_with("postgres://") && !config.database_url.starts_with("postgresql://") {
        anyhow::bail!("DATABASE_URL must be a valid postgres connection string");
    }

    Ok(())
}
