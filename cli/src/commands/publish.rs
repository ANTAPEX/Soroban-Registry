//! `soroban-registry publish` — publish a contract to the registry.

use crate::commands::test::run_contract_tests;
use crate::support::network::shared_network;
use crate::support::network::Network;
use anyhow::Result;
use colored::Colorize;

/// A key that is stable for the same contract identity on the same network, so
/// re-running `publish` after a dropped response does not register twice.
fn publish_idempotency_key(request: &registry_client::PublishRequest) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(
        format!(
            "{}:{}:{}",
            request.contract_id, request.network, request.publisher_address
        )
        .as_bytes(),
    );
    format!("cli-publish-{}", hex::encode(&digest[..16]))
}

#[allow(clippy::too_many_arguments)]
pub async fn publish(
    api_url: &str,
    contract_id: &str,
    name: &str,
    description: Option<&str>,
    network: Network,
    category: Option<&str>,
    tags: Vec<String>,
    publisher: &str,
    is_cicd: bool,
    contract_path: &str,
    test_command: Option<&str>,
    require_coverage: bool,
    coverage_threshold: f64,
    skip_tests: bool,
) -> Result<()> {
    if !skip_tests {
        run_contract_tests(
            contract_path,
            test_command,
            require_coverage,
            coverage_threshold,
            true,
        )
        .await?;
    }

    let network = shared_network(&network.to_string()).ok_or_else(|| {
        anyhow::anyhow!("Invalid network: {network}. Allowed values: mainnet, testnet, futurenet")
    })?;

    // `wasm_hash` is required by the API; the deploy and register flows compute
    // it from the artifact. `publish` registers an already-deployed contract
    // from metadata alone, so it sends the field empty as it always has.
    let request = registry_client::PublishRequest {
        contract_id: contract_id.to_string(),
        wasm_hash: String::new(),
        wasm_artifact_base64: None,
        name: name.to_string(),
        slug: None,
        description: description.map(str::to_string),
        network,
        category: category.map(str::to_string),
        tags,
        source_url: None,
        publisher_address: publisher.to_string(),
        dependencies: Vec::new(),
        is_cicd,
    };

    println!("\n{}", "Publishing contract...".bold().cyan());

    // Publishing is not idempotent by itself, so the CLI supplies a key derived
    // from the contract identity: a retried publish (or a re-run after a lost
    // response) replays the original result instead of conflicting.
    let idempotency_key = publish_idempotency_key(&request);
    let contract = crate::support::registry::uncached_client(api_url)
        .await?
        .publish_contract(&request, Some(idempotency_key))
        .await
        .map_err(|err| anyhow::anyhow!("Failed to publish: {err}"))?;

    println!("{}", "[OK] Contract published successfully!".green().bold());
    println!("\n{}: {}", "Name".bold(), contract.name);
    println!("{}: {}", "ID".bold(), contract.contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        contract.network.to_string().bright_blue()
    );
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {

    const VALID_CONTRACT_ID: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
    const VALID_PUBLISHER: &str = "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ";

    fn publish_request(
        contract_id: &str,
        network: registry_client::Network,
        publisher: &str,
    ) -> registry_client::PublishRequest {
        registry_client::PublishRequest {
            contract_id: contract_id.to_string(),
            wasm_hash: String::new(),
            wasm_artifact_base64: None,
            name: "My Contract".to_string(),
            slug: None,
            description: None,
            network,
            category: None,
            tags: Vec::new(),
            source_url: None,
            publisher_address: publisher.to_string(),
            dependencies: Vec::new(),
            is_cicd: false,
        }
    }

    #[test]
    fn publish_idempotency_key_is_stable_for_same_identity() {
        let key = super::publish_idempotency_key(&publish_request(
            VALID_CONTRACT_ID,
            registry_client::Network::Testnet,
            VALID_PUBLISHER,
        ));

        assert!(key.starts_with("cli-publish-"), "unexpected key: {key}");
        assert_eq!(
            key,
            super::publish_idempotency_key(&publish_request(
                VALID_CONTRACT_ID,
                registry_client::Network::Testnet,
                VALID_PUBLISHER,
            )),
            "the same contract identity must always produce the same key"
        );
    }

    /// The retry-replay guarantee depends on the key covering *only* the contract
    /// identity: a re-run that edits metadata must still replay the original publish
    /// rather than registering a second contract.
    #[test]
    fn publish_idempotency_key_ignores_non_identity_metadata() {
        let base = publish_request(
            VALID_CONTRACT_ID,
            registry_client::Network::Testnet,
            VALID_PUBLISHER,
        );

        let mut edited = base.clone();
        edited.name = "Renamed Contract".to_string();
        edited.description = Some("now with a description".to_string());
        edited.category = Some("Token".to_string());
        edited.tags = vec!["defi".to_string()];
        edited.is_cicd = true;

        assert_eq!(
            super::publish_idempotency_key(&base),
            super::publish_idempotency_key(&edited),
            "metadata outside the contract identity must not change the key"
        );
    }

    #[test]
    fn publish_idempotency_key_separates_distinct_identities() {
        let base = super::publish_idempotency_key(&publish_request(
            VALID_CONTRACT_ID,
            registry_client::Network::Testnet,
            VALID_PUBLISHER,
        ));

        let other_network = super::publish_idempotency_key(&publish_request(
            VALID_CONTRACT_ID,
            registry_client::Network::Mainnet,
            VALID_PUBLISHER,
        ));
        let other_publisher = super::publish_idempotency_key(&publish_request(
            VALID_CONTRACT_ID,
            registry_client::Network::Testnet,
            "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
        ));
        let other_contract = super::publish_idempotency_key(&publish_request(
            "CBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
            registry_client::Network::Testnet,
            VALID_PUBLISHER,
        ));

        assert_ne!(base, other_network, "network must be part of the identity");
        assert_ne!(
            base, other_publisher,
            "publisher must be part of the identity"
        );
        assert_ne!(
            base, other_contract,
            "contract id must be part of the identity"
        );
    }
}
