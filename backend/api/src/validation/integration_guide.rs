//! Integration guide and example handlers for validation middleware
//!
//! This module demonstrates how to use the comprehensive validation system
//! in your API handlers.\n\n#[cfg(test)]
mod integration_tests {
    use crate::validation::{
        sanitizers::*, url_validation::*, validators::*, FieldError, Validatable, ValidationBuilder,
    };
    use serde::{Deserialize, Serialize};

    // ============================================================================
    // Example: Contract Request with Full Validation
    // ============================================================================

    #[derive(Debug, Deserialize, Serialize)]
    pub struct PublishContractRequest {
        pub contract_id: String,
        pub name: String,
        pub description: Option<String>,
        pub version: String,
        pub source_url: Option<String>,
        pub publisher_address: String,
        pub tags: Vec<String>,
    }

    impl Validatable for PublishContractRequest {
        fn sanitize(&mut self) {
            self.contract_id = normalize_contract_id(&self.contract_id);
            self.name = sanitize_name(&self.name);
            sanitize_description_optional(&mut self.description);
            self.publisher_address = normalize_stellar_address(&self.publisher_address);
            self.tags = sanitize_tags(&self.tags);
            if let Some(ref mut url) = self.source_url {
                *url = url.trim().to_string();
            }
        }

        fn validate(&self) -> Result<(), Vec<FieldError>> {
            let mut builder = ValidationBuilder::new();

            // Validate contract ID format
            builder.check("contract_id", || validate_contract_id(&self.contract_id));

            // Validate name: 1-255 chars, no HTML
            builder.check("name", || validate_required(&self.name, "name"));
            builder.check("name", || validate_length(&self.name, 1, 255));
            builder.check("name", || validate_no_html(&self.name));

            // Validate description if present: max 5000 chars
            if let Some(desc) = &self.description {
                builder.check("description", || validate_length(desc, 0, 5000));
                builder.check("description", || validate_no_html(desc));
            }

            // Validate version: strict semver
            builder.check("version", || validate_semver(&self.version));

            // Validate source URL if present: HTTPS + whitelist
            if let Some(url) = &self.source_url {
                builder.check("source_url", || validate_url_https_only_with_whitelist(url));
            }

            // Validate publisher address format
            builder.check("publisher_address", || {
                validate_stellar_address(&self.publisher_address)
            });

            // Validate tags: max 10, each max 50 chars
            builder.check("tags", || validate_tags(&self.tags, 10, 50));

            builder.build()
        }
    }

    // Example handler using the validated request:
    //
    // pub async fn publish_contract(
    //     State(state): State<AppState>,
    //     ValidatedJson(req): ValidatedJson<PublishContractRequest>,
    // ) -> Result<impl IntoResponse, ApiError> {
    //     // All validation and sanitization already done!
    //     // The request is guaranteed to be safe and valid
    //
    //     // req.contract_id is now normalized and validated
    //     // req.name has HTML stripped and length checked
    //     // req.source_url is HTTPS from whitelisted domain (if present)
    //     // All failures are logged with IP tracking
    //
    //     let contract = create_contract(&req).await?;
    //     Ok(Json(contract))
    // }

    // ============================================================================
    // Example: Update Contract Request
    // ============================================================================

    #[derive(Debug, Deserialize, Serialize)]
    pub struct UpdateContractRequest {
        pub name: Option<String>,
        pub description: Option<String>,
        pub tags: Option<Vec<String>>,
    }

    impl Validatable for UpdateContractRequest {
        fn sanitize(&mut self) {
            if let Some(ref mut name) = self.name {
                *name = sanitize_name(name);
            }
            sanitize_description_optional(&mut self.description);
            if let Some(ref mut tags) = self.tags {
                *tags = sanitize_tags(tags);
            }
        }

        fn validate(&self) -> Result<(), Vec<FieldError>> {
            let mut builder = ValidationBuilder::new();

            if let Some(name) = &self.name {
                builder.check("name", || validate_length(name, 1, 255));
                builder.check("name", || validate_no_html(name));
            }

            if let Some(desc) = &self.description {
                builder.check("description", || validate_length(desc, 0, 5000));
                builder.check("description", || validate_no_html(desc));
            }

            if let Some(tags) = &self.tags {
                builder.check("tags", || validate_tags(tags, 10, 50));
            }

            builder.build()
        }
    }

    // ============================================================================
    // Example: Verify Contract Request
    // ============================================================================

    #[derive(Debug, Deserialize, Serialize)]
    pub struct VerifyContractRequest {
        pub contract_id: String,
        pub publisher_address: String,
        pub source_url: String,
        pub checksum: String,
    }

    impl Validatable for VerifyContractRequest {
        fn sanitize(&mut self) {
            self.contract_id = normalize_contract_id(&self.contract_id);
            self.publisher_address = normalize_stellar_address(&self.publisher_address);
            self.source_url = self.source_url.trim().to_string();
            self.checksum = self.checksum.trim().to_uppercase();
        }

        fn validate(&self) -> Result<(), Vec<FieldError>> {
            let mut builder = ValidationBuilder::new();

            builder.check("contract_id", || validate_contract_id(&self.contract_id));
            builder.check("publisher_address", || {
                validate_stellar_address(&self.publisher_address)
            });
            builder.check("source_url", || {
                validate_url_https_only_with_whitelist(&self.source_url)
            });

            // Checksum validation: 64 hex chars (SHA-256)
            builder.check_condition(
                crate::validation::validators::validate_length(&self.checksum, 64, 64).is_err(),
                "checksum",
                "must be 64 hex characters (SHA-256)",
            );

            builder.build()
        }
    }

    // ============================================================================
    // Example: Create Publisher Request
    // ============================================================================

    #[derive(Debug, Deserialize, Serialize)]
    pub struct CreatePublisherRequest {
        pub name: String,
        pub stellar_address: String,
        pub description: Option<String>,
        pub website_url: Option<String>,
    }

    impl Validatable for CreatePublisherRequest {
        fn sanitize(&mut self) {
            self.name = sanitize_name(&self.name);
            self.stellar_address = normalize_stellar_address(&self.stellar_address);
            sanitize_description_optional(&mut self.description);
            if let Some(ref mut url) = self.website_url {
                *url = url.trim().to_string();
            }
        }

        fn validate(&self) -> Result<(), Vec<FieldError>> {
            let mut builder = ValidationBuilder::new();

            builder.check("name", || validate_required(&self.name, "name"));
            builder.check("name", || validate_length(&self.name, 1, 255));
            builder.check("name", || validate_no_html(&self.name));

            builder.check("stellar_address", || {
                validate_stellar_address(&self.stellar_address)
            });

            if let Some(desc) = &self.description {
                builder.check("description", || validate_length(desc, 0, 5000));
            }

            if let Some(url) = &self.website_url {
                builder.check("website_url", || validate_https_url_only(url));
            }

            builder.build()
        }
    }

    // ============================================================================
    // Tests Demonstrating Request Validation
    // ============================================================================

    #[test]
    fn test_publish_contract_request_valid() {
        let req = PublishContractRequest {
            contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
            name: "My Token Contract".to_string(),
            description: Some("A token contract for Soroban".to_string()),
            version: "1.0.0".to_string(),
            source_url: Some("https://github.com/stellar/example".to_string()),
            publisher_address: "GBRPYHIL2CI3WHZSRXG5ZRAML54KVXVP5JUDLHTCHHYDAYCOPREA5Z5L"
                .to_string(),
            tags: vec!["token".to_string(), "defi".to_string()],
        };

        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_publish_contract_request_invalid_contract_id() {
        let req = PublishContractRequest {
            contract_id: "INVALID_ID".to_string(),
            name: "My Contract".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            source_url: None,
            publisher_address: "GBRPYHIL2CI3WHZSRXG5ZRAML54KVXVP5JUDLHTCHHYDAYCOPREA5Z5L"
                .to_string(),
            tags: vec![],
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "contract_id"));
    }

    #[test]
    fn test_publish_contract_request_html_stripped() {
        let mut req = PublishContractRequest {
            contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
            name: "<script>alert('xss')</script>Token".to_string(),
            description: Some("<b>Bold</b> description".to_string()),
            version: "1.0.0".to_string(),
            source_url: None,
            publisher_address: "GBRPYHIL2CI3WHZSRXG5ZRAML54KVXVP5JUDLHTCHHYDAYCOPREA5Z5L"
                .to_string(),
            tags: vec![],
        };

        req.sanitize();

        // HTML should be stripped
        assert!(!req.name.contains('<'));
        assert!(!req.name.contains('>'));
        assert!(!req.description.as_ref().unwrap().contains('<'));
    }

    #[test]
    fn test_publish_contract_request_invalid_url() {
        let req = PublishContractRequest {
            contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
            name: "My Contract".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            source_url: Some("http://untrusted.com/repo".to_string()), // HTTP not allowed
            publisher_address: "GBRPYHIL2CI3WHZSRXG5ZRAML54KVXVP5JUDLHTCHHYDAYCOPREA5Z5L"
                .to_string(),
            tags: vec![],
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "source_url"));
    }

    #[test]
    fn test_verify_contract_request_normalized() {
        let mut req = VerifyContractRequest {
            contract_id: "  cdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc  ".to_string(),
            publisher_address: "  gbrpyhil2ci3whzsrxg5zraml54kvxvp5judlhtchhydaycoprea5z5l  "
                .to_string(),
            source_url: "https://github.com/stellar/example".to_string(),
            checksum: "abc123def456".to_string(),
        };

        req.sanitize();

        // Should be normalized to uppercase and trimmed
        assert_eq!(
            req.contract_id,
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
        );
        assert_eq!(
            req.publisher_address,
            "GBRPYHIL2CI3WHZSRXG5ZRAML54KVXVP5JUDLHTCHHYDAYCOPREA5Z5L"
        );
        assert_eq!(req.checksum, "ABC123DEF456");
    }

    #[test]
    fn test_create_publisher_request_website_validation() {
        let req = CreatePublisherRequest {
            name: "My Publisher".to_string(),
            stellar_address: "GBRPYHIL2CI3WHZSRXG5ZRAML54KVXVP5JUDLHTCHHYDAYCOPREA5Z5L".to_string(),
            description: None,
            website_url: Some("http://example.com".to_string()), // HTTP not allowed
        };

        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_contract_request_partial_validation() {
        let req = UpdateContractRequest {
            name: Some("Updated Name".to_string()),
            description: None,
            tags: None,
        };

        // Validation should pass even though other fields are None
        assert!(req.validate().is_ok());
    }
}
