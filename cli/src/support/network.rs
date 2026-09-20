//! The `Network` the CLI talks to, and the parsing shared by every command
//! that takes a network argument.

use crate::support::filters::normalize_filter_values;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Futurenet => write!(f, "futurenet"),
        }
    }
}

impl FromStr for Network {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            "futurenet" => Ok(Network::Futurenet),
            _ => anyhow::bail!(
                "Invalid network: {}. Allowed values: mainnet, testnet, futurenet",
                s
            ),
        }
    }
}

/// Canonical network name to the registry's `Network`. Unknown values are
/// rejected by `normalize_network_filters` before reaching here.
pub(crate) fn shared_network(value: &str) -> Option<registry_client::Network> {
    match value.trim().to_lowercase().as_str() {
        "mainnet" => Some(registry_client::Network::Mainnet),
        "testnet" => Some(registry_client::Network::Testnet),
        "futurenet" => Some(registry_client::Network::Futurenet),
        _ => None,
    }
}

pub(crate) fn shared_networks(values: &[String]) -> Result<Vec<registry_client::Network>> {
    values
        .iter()
        .map(|value| {
            shared_network(value).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid network: {value}. Allowed values: mainnet, testnet, futurenet"
                )
            })
        })
        .collect()
}

/// Parse a comma-separated `--networks` value into the registry's network type.
///
/// Shared with `contract list` so every command validates network filters
/// identically: unknown names fail here, locally, instead of being dropped
/// server-side and silently returning unfiltered results.
pub(crate) fn normalize_network_list(value: Option<&str>) -> Result<Vec<registry_client::Network>> {
    let values: Vec<String> = value.into_iter().map(str::to_string).collect();
    let normalized = normalize_network_filters(&values)?;
    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    shared_networks(&normalized)
}

/// Normalize network filters to their canonical lowercase names, rejecting
/// unknown values before any request is sent so invalid input fails clearly
/// and locally instead of silently returning unfiltered results.
pub(crate) fn normalize_network_filters(values: &[String]) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for value in normalize_filter_values(values) {
        let canonical = Network::from_str(&value)?.to_string();
        if seen.insert(canonical.clone()) {
            normalized.push(canonical);
        }
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_parsing() {
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("futurenet".parse::<Network>().unwrap(), Network::Futurenet);
        assert_eq!("Mainnet".parse::<Network>().unwrap(), Network::Mainnet); // Case insensitive
        assert!("invalid".parse::<Network>().is_err());
    }
}
