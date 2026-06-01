// tests/search_filter_tests.rs
//
// Issue #948: Add unified contract search filters for network, category, and verification.
// Verifies mixed filter combinations and empty result sets on /api/contracts.

use reqwest::StatusCode;
use serde_json::Value;

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_mixed_filter_combinations_and_empty_results() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    // 1. Test a mixed filter combination (category + network + verified_only + query)
    let mixed_url = format!(
        "{}/api/contracts?networks=testnet&categories=DeFi&verified_only=true&query=token",
        base
    );
    let res = client
        .get(&mixed_url)
        .send()
        .await
        .expect("Failed to call contracts list with mixed filters");

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Mixed filter request should return 200 OK"
    );

    let body: Value = res
        .json()
        .await
        .expect("Failed to deserialize response body");

    // Check pagination metadata structure
    assert!(body.get("items").is_some(), "Response must include items");
    assert!(body.get("total").is_some(), "Response must include total count");
    
    // Check that response filters metadata is populated correctly
    let filters = body.get("filters").expect("Response must include active filter metadata");
    assert!(
        filters.get("verified_only").and_then(Value::as_bool).unwrap_or(false),
        "verified_only should be true in response metadata"
    );
    assert_eq!(
        filters.get("query").and_then(Value::as_str),
        Some("token"),
        "query should match search term in response metadata"
    );

    // 2. Test empty result set (filtering with something that doesn't exist)
    let empty_url = format!(
        "{}/api/contracts?networks=mainnet&query=nonexistent_contract_name_search_12345",
        base
    );
    let res_empty = client
        .get(&empty_url)
        .send()
        .await
        .expect("Failed to call contracts list with empty criteria");

    assert_eq!(
        res_empty.status(),
        StatusCode::OK,
        "Request resulting in empty set should return 200 OK"
    );

    let body_empty: Value = res_empty
        .json()
        .await
        .expect("Failed to deserialize empty response body");

    let items = body_empty
        .get("items")
        .and_then(Value::as_array)
        .expect("Response must include items array");

    let total = body_empty
        .get("total")
        .and_then(Value::as_i64)
        .expect("Response must include total count");

    assert_eq!(
        items.len(),
        0,
        "Expected empty result set items array length to be 0"
    );
    assert_eq!(
        total,
        0,
        "Expected empty result set total count to be 0"
    );
}
