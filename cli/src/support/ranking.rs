//! Search Ranking Transparency and Explainability (`--explain`)
//!
//! Provides granular factor attribution explaining why a given contract ranks where it does.
//! Adheres to the exact schema defined in Issue #1187:
//!
//! ```json
//! {
//!   "results": [
//!     {
//!       "contract_id": "...",
//!       "score": 8.42,
//!       "factors": {
//!         "text_relevance": 3.1,
//!         "verification_bonus": 2.0,
//!         "popularity": 1.8,
//!         "recency": 0.52,
//!         "deprecation_penalty": -1.0
//!       }
//!     }
//!   ]
//! }
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::models::{Contract, DeprecationStatus, VerificationStatus};

/// Breakdown of individual ranking factors contributing to the final score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingFactors {
    /// Text relevance matching score between query terms and contract metadata (0.10 to 5.00).
    pub text_relevance: f64,
    /// Verification status bonus (+2.00 for verified, 0.00 for unverified).
    pub verification_bonus: f64,
    /// Popularity signal computed logarithmically from cumulative usage count (0.00 to 2.50).
    pub popularity: f64,
    /// Recency decay score based on time elapsed since last contract update (0.05 to 1.00).
    pub recency: f64,
    /// Penalty applied to deprecated or superseded contracts (-1.00 if deprecated, 0.00 if active).
    pub deprecation_penalty: f64,
}

/// An individual contract entry within the explainability search output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainSearchResult {
    pub contract_id: String,
    pub score: f64,
    pub factors: RankingFactors,
}

/// Root JSON envelope matching Issue #1187's specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainSearchResponse {
    pub results: Vec<ExplainSearchResult>,
}

/// Rounds a float to 2 decimal places to guarantee clean JSON representation and exact factor summation.
#[inline]
pub fn round2(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
}

/// Computes the ranking factors and composite score for a given contract.
///
/// Guaranteed property:
/// `score == round2(factors.text_relevance + factors.verification_bonus + factors.popularity + factors.recency + factors.deprecation_penalty)`
pub fn calculate_ranking_factors(query: &str, contract: &Contract) -> (f64, RankingFactors) {
    // 1. Text Relevance Factor (0.10 to 5.00)
    let q = query.trim().to_lowercase();
    let name_lower = contract.name.to_lowercase();
    let slug_lower = contract.slug.to_lowercase();
    let desc_lower = contract
        .description
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    let cat_lower = contract.category.as_deref().unwrap_or("").to_lowercase();

    let text_relevance = if q.is_empty() {
        1.00
    } else if let Some(base_rel) = contract.relevance_score.filter(|&s| s > 0.0) {
        // If the backend already provided a ts_rank or ES BM25 relevance score, normalize it.
        round2((base_rel * 10.0).clamp(0.10, 5.00))
    } else {
        let mut match_score: f64 = 0.0;
        let terms: Vec<&str> = q.split_whitespace().collect();

        // Exact name match bonus
        if name_lower == q || slug_lower == q {
            match_score += 3.10;
        } else if name_lower.contains(&q) || slug_lower.contains(&q) {
            match_score += 2.20;
        } else {
            for term in &terms {
                if name_lower.contains(term) {
                    match_score += 1.20;
                }
                if desc_lower.contains(term) {
                    match_score += 0.50;
                }
                if cat_lower.contains(term) {
                    match_score += 0.40;
                }
            }
        }

        // Add small baseline match
        if match_score == 0.0 {
            match_score = 0.50;
        }

        round2(match_score.clamp(0.10, 5.00))
    };

    // 2. Verification Bonus (+2.00 for verified, 0.00 otherwise)
    let verification_bonus = if contract.is_verified
        || matches!(contract.verification_status, VerificationStatus::Verified)
    {
        2.00
    } else {
        0.00
    };

    // 3. Popularity Signal (0.00 to 2.50)
    // Formula: ln(usage_count + 1) * 0.45
    let usage = contract.usage_count.max(0) as f64;
    let popularity = round2(((usage + 1.0).ln() * 0.45).clamp(0.00, 2.50));

    // 4. Recency Signal (0.05 to 1.00)
    // Formula: 1.0 / (1.0 + days_since_update / 60.0)
    let now = Utc::now();
    let days_old = (now - contract.updated_at).num_days().max(0) as f64;
    let recency = round2((1.0 / (1.0 + days_old / 60.0)).clamp(0.05, 1.00));

    // 5. Deprecation Penalty (-1.00 for deprecated/superseded, 0.00 for active)
    let is_dep = contract.is_deprecated
        || contract.deprecation_status == DeprecationStatus::Deprecated
        || contract.deprecation_status == DeprecationStatus::Superseded
        || contract.deprecated_at.is_some();

    let deprecation_penalty = if is_dep { -1.00 } else { 0.00 };

    // Final composite score (clamped to min 0.00)
    let score = round2(
        (text_relevance
            + verification_bonus
            + popularity
            + recency
            + deprecation_penalty)
            .max(0.0),
    );

    let factors = RankingFactors {
        text_relevance,
        verification_bonus,
        popularity,
        recency,
        deprecation_penalty,
    };

    (score, factors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use shared::models::{Network, VisibilityType};
    use uuid::Uuid;

    fn make_test_contract(
        is_verified: bool,
        usage_count: i64,
        days_ago: i64,
        is_deprecated: bool,
    ) -> Contract {
        let now = Utc::now();
        let updated_at = now - Duration::days(days_ago);
        let created_at = updated_at - Duration::days(10);

        Contract {
            id: Uuid::new_v4(),
            contract_id: "CDUMMYCONTRACT1234567890".to_string(),
            wasm_hash: "abcd1234efgh".to_string(),
            name: "Soroban Token Contract".to_string(),
            slug: "soroban-token-contract".to_string(),
            description: Some("Standard token implementation for Soroban".to_string()),
            publisher_id: Uuid::new_v4(),
            network: Network::Mainnet,
            is_verified,
            verification_status: if is_verified {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Unverified
            },
            category: Some("DeFi".to_string()),
            tags: vec![],
            created_at,
            updated_at,
            verified_at: if is_verified { Some(updated_at) } else { None },
            deployed_at: Some(created_at),
            verified_by: None,
            verification_notes: None,
            last_accessed_at: Some(now),
            health_score: 95,
            is_maintenance: false,
            logical_id: None,
            network_configs: None,
            relevance_score: None,
            organization_id: None,
            visibility: VisibilityType::Public,
            artifact_scan_status: "passed".to_string(),
            artifact_scan_findings: serde_json::json!({}),
            current_version: Some("1.0.0".to_string()),
            usage_count,
            deprecated_at: if is_deprecated { Some(now) } else { None },
            deprecation_reason: if is_deprecated {
                Some("Deprecated in favor of v2".to_string())
            } else {
                None
            },
            replacement_contract_id: None,
            is_deprecated,
            deprecation_status: if is_deprecated {
                DeprecationStatus::Deprecated
            } else {
                DeprecationStatus::Active
            },
        }
    }

    #[test]
    fn test_verified_vs_unverified_bonus() {
        let query = "token contract";
        let verified = make_test_contract(true, 10, 5, false);
        let unverified = make_test_contract(false, 10, 5, false);

        let (score_v, factors_v) = calculate_ranking_factors(query, &verified);
        let (score_u, factors_u) = calculate_ranking_factors(query, &unverified);

        assert_eq!(factors_v.verification_bonus, 2.00);
        assert_eq!(factors_u.verification_bonus, 0.00);
        assert!(score_v > score_u);
        assert_eq!(
            score_v,
            round2(
                factors_v.text_relevance
                    + factors_v.verification_bonus
                    + factors_v.popularity
                    + factors_v.recency
                    + factors_v.deprecation_penalty
            )
        );
    }

    #[test]
    fn test_deprecation_penalty() {
        let query = "token contract";
        let active = make_test_contract(true, 50, 10, false);
        let deprecated = make_test_contract(true, 50, 10, true);

        let (score_a, factors_a) = calculate_ranking_factors(query, &active);
        let (score_d, factors_d) = calculate_ranking_factors(query, &deprecated);

        assert_eq!(factors_a.deprecation_penalty, 0.00);
        assert_eq!(factors_d.deprecation_penalty, -1.00);
        assert!(score_a > score_d);
        assert_eq!(round2(score_a - score_d), 1.00);
    }

    #[test]
    fn test_popularity_scaling() {
        let query = "token contract";
        let low_pop = make_test_contract(true, 0, 10, false);
        let mid_pop = make_test_contract(true, 50, 10, false);
        let high_pop = make_test_contract(true, 500, 10, false);

        let (_, f_low) = calculate_ranking_factors(query, &low_pop);
        let (_, f_mid) = calculate_ranking_factors(query, &mid_pop);
        let (_, f_high) = calculate_ranking_factors(query, &high_pop);

        assert_eq!(f_low.popularity, 0.00);
        assert!(f_mid.popularity > f_low.popularity);
        assert!(f_high.popularity > f_mid.popularity);
        assert!(f_high.popularity <= 2.50);
    }

    #[test]
    fn test_recency_decay() {
        let query = "token contract";
        let brand_new = make_test_contract(true, 20, 0, false);
        let month_old = make_test_contract(true, 20, 60, false);
        let year_old = make_test_contract(true, 20, 365, false);

        let (_, f_new) = calculate_ranking_factors(query, &brand_new);
        let (_, f_month) = calculate_ranking_factors(query, &month_old);
        let (_, f_year) = calculate_ranking_factors(query, &year_old);

        assert_eq!(f_new.recency, 1.00);
        assert_eq!(f_month.recency, 0.50);
        assert!(f_year.recency < f_month.recency);
    }

    #[test]
    fn test_factors_exact_summation() {
        let query = "token contract";
        let contract = make_test_contract(true, 100, 25, false);
        let (score, factors) = calculate_ranking_factors(query, &contract);

        let sum = round2(
            factors.text_relevance
                + factors.verification_bonus
                + factors.popularity
                + factors.recency
                + factors.deprecation_penalty,
        );

        assert_eq!(score, sum);
    }
}
