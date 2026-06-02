//! URL and domain validation with whitelist support
//!
//! This module provides secure URL validation with:
//! - HTTPS-only enforcement
//! - Domain whitelist support
//! - URL parsing and validation
//! - Safe URL normalization

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

lazy_static! {
    /// HTTPS URL pattern validation
    static ref HTTPS_URL_REGEX: Regex = Regex::new(
        r"^https://[a-zA-Z0-9][a-zA-Z0-9\-\.]*[a-zA-Z0-9](?:\:[0-9]{1,5})?(?:/[^\s]*)?$"
    ).unwrap();

    /// Domain extraction from URL
    static ref DOMAIN_REGEX: Regex = Regex::new(
        r"^https?://(?:www\.)?([a-zA-Z0-9][a-zA-Z0-9\-\.]*[a-zA-Z0-9])"
    ).unwrap();

    /// Default whitelist of allowed domains for source URLs
    /// Users can override via environment variable ALLOWED_DOMAINS
    static ref DEFAULT_DOMAIN_WHITELIST: HashSet<&'static str> = {
        vec![
            "github.com",
            "gitlab.com",
            "gitea.com",
            "git.stellar.org",
            "github.stellar.org",
            "soroban.stellar.org",
            "docs.rs",
            "crates.io",
            "raw.githubusercontent.com",
            "raw.gitlab.com",
            "soroban-registry.com",
            "soroban-registry.stellar.org",
        ].into_iter().collect()
    };
}

/// Get the configured domain whitelist from environment or use default
pub fn get_domain_whitelist() -> HashSet<String> {
    match std::env::var("ALLOWED_DOMAINS") {
        Ok(domains_str) => domains_str
            .split(',')
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty())
            .collect(),
        Err(_) => DEFAULT_DOMAIN_WHITELIST
            .iter()
            .map(|d| d.to_lowercase())
            .collect(),
    }
}

/// Validate URL is HTTPS and domain is whitelisted
pub fn validate_url_https_only_with_whitelist(url: &str) -> Result<(), String> {
    let trimmed = url.trim();

    if trimmed.is_empty() {
        return Ok(()); // Empty URLs are optional fields
    }

    // Must be HTTPS
    if !trimmed.starts_with("https://") {
        return Err("URLs must use HTTPS protocol, HTTP is not allowed".to_string());
    }

    // Must match HTTPS URL pattern
    if !HTTPS_URL_REGEX.is_match(trimmed) {
        return Err("Invalid HTTPS URL format".to_string());
    }

    // Extract and validate domain is whitelisted
    if let Some(caps) = DOMAIN_REGEX.captures(trimmed) {
        if let Some(domain_match) = caps.get(1) {
            let domain = domain_match.as_str().to_lowercase();
            let whitelist = get_domain_whitelist();

            // Check exact match
            if whitelist.contains(&domain) {
                return Ok(());
            }

            // Check wildcard subdomain match (e.g., *.github.com matches api.github.com)
            for allowed in whitelist.iter() {
                if let Some(wildcard_domain) = allowed.strip_prefix("*.") {
                    if domain.ends_with(&format!(".{}", wildcard_domain))
                        || domain == wildcard_domain
                    {
                        return Ok(());
                    }
                }
            }

            return Err(format!(
                "Domain '{}' is not whitelisted. Allowed domains: {:?}",
                domain, whitelist
            ));
        }
    }

    Err("Could not extract domain from URL".to_string())
}

/// Validate URL format without domain restrictions (for testing)
pub fn validate_https_url_only(url: &str) -> Result<(), String> {
    let trimmed = url.trim();

    if trimmed.is_empty() {
        return Ok(());
    }

    if !trimmed.starts_with("https://") {
        return Err("URLs must use HTTPS protocol, HTTP is not allowed".to_string());
    }

    if !HTTPS_URL_REGEX.is_match(trimmed) {
        return Err("Invalid HTTPS URL format".to_string());
    }

    Ok(())
}

/// Parse a URL and extract its components safely
pub fn parse_url_components(url: &str) -> Result<UrlComponents, String> {
    let trimmed = url.trim();

    if trimmed.is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    // Use the url crate or simple regex for parsing
    if let Some(caps) = DOMAIN_REGEX.captures(trimmed) {
        if let Some(domain_match) = caps.get(1) {
            let domain = domain_match.as_str().to_lowercase();

            // Extract path if present
            let path = if let Some(slash_pos) = trimmed[8..].find('/') {
                // Skip protocol (8 chars for "https://")
                trimmed[8 + slash_pos..].to_string()
            } else {
                "/".to_string()
            };

            return Ok(UrlComponents { domain, path });
        }
    }

    Err("Could not parse URL".to_string())
}

#[derive(Debug, Clone)]
pub struct UrlComponents {
    pub domain: String,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_https_url_only() {
        assert!(validate_https_url_only("https://github.com/stellar/rs-soroban-sdk").is_ok());
        assert!(validate_https_url_only("  https://example.com  ").is_ok());

        // HTTP should fail
        assert!(validate_https_url_only("http://example.com").is_err());
        assert!(validate_https_url_only("http://github.com/stellar/rs-soroban-sdk").is_err());

        // FTP should fail
        assert!(validate_https_url_only("ftp://example.com").is_err());

        // Empty is OK (optional field)
        assert!(validate_https_url_only("").is_ok());
        assert!(validate_https_url_only("   ").is_ok());
    }

    #[test]
    fn test_validate_url_with_whitelist() {
        // Should pass with default whitelist
        assert!(validate_url_https_only_with_whitelist(
            "https://github.com/stellar/rs-soroban-sdk"
        )
        .is_ok());
        assert!(validate_url_https_only_with_whitelist("https://gitlab.com/stellar/sdk").is_ok());

        // Should fail for non-whitelisted domain
        assert!(validate_url_https_only_with_whitelist("https://example.com").is_err());
        assert!(validate_url_https_only_with_whitelist("https://attacker.com").is_err());

        // Should fail for HTTP
        assert!(validate_url_https_only_with_whitelist("http://github.com").is_err());
    }

    #[test]
    fn test_parse_url_components() {
        let result = parse_url_components("https://github.com/stellar/rs-soroban-sdk");
        assert!(result.is_ok());
        let components = result.unwrap();
        assert_eq!(components.domain, "github.com");
        assert_eq!(components.path, "/stellar/rs-soroban-sdk");
    }

    #[test]
    fn test_parse_url_components_root_path() {
        let result = parse_url_components("https://docs.rs/");
        assert!(result.is_ok());
        let components = result.unwrap();
        assert_eq!(components.domain, "docs.rs");
        assert_eq!(components.path, "/");
    }
}
