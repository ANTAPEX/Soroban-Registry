//! The JSON shape the CLI prints for a contract, shared by `list` and `search`
//! so the two agree.

/// The JSON projection both `list` and `search` emit for a contract.
pub(crate) fn contract_json(contract: &registry_client::Contract) -> serde_json::Value {
    let tags: Vec<&str> = contract.tags.iter().map(|tag| tag.name.as_str()).collect();
    serde_json::json!({
        "id": contract.id.to_string(),
        "name": contract.name,
        "contract_id": contract.contract_id,
        "network": contract.network.to_string(),
        "category": contract.category.as_deref().unwrap_or(""),
        "is_verified": contract.is_verified,
        "health_score": contract.health_score,
        "created_at": contract.created_at.to_rfc3339(),
        "tags": tags,
    })
}
