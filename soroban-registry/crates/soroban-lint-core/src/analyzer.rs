use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use anyhow::Result;

/// Main analyzer that runs all lint rules
pub struct Analyzer {
    rules: Vec<Box<dyn LintRule>>,
}

impl Analyzer {
    /// Create a new analyzer with all available rules
    pub fn new() -> Self {
        Self::with_default_rules()
    }

    /// Create analyzer with default rules
    pub fn with_default_rules() -> Self {
        let rules: Vec<Box<dyn LintRule>> = vec![
            Box::new(crate::rules::missing_error_handling::MissingErrorHandlingRule),
            Box::new(crate::rules::unused_variables::UnusedVariablesRule),
            Box::new(crate::rules::unsafe_patterns::UnsafeUnwrapRule),
            Box::new(crate::rules::integer_overflow::IntegerOverflowRule),
            Box::new(crate::rules::reentrancy::ReentrancyRule),
            Box::new(crate::rules::storage_key_collision::StorageKeyCollisionRule),
            Box::new(crate::rules::missing_auth_check::MissingAuthCheckRule),
            Box::new(crate::rules::unbounded_loops::UnboundedLoopsRule),
            Box::new(crate::rules::hardcoded_addresses::HardcodedAddressesRule),
            Box::new(crate::rules::deprecated_api_usage::DeprecatedApiUsageRule),
            Box::new(crate::rules::large_data_in_storage::LargeDataInStorageRule),
            Box::new(crate::rules::missing_events::MissingEventsRule),
            Box::new(crate::rules::inefficient_clones::InefficientClonesRule),
            Box::new(crate::rules::public_fn_no_doc::PublicFnNoDocRule),
            Box::new(crate::rules::unchecked_arithmetic::UncheckedArithmeticRule),
            Box::new(crate::rules::direct_storage_clear::DirectStorageClearRule),
            Box::new(crate::rules::panic_in_contract::PanicInContractRule),
            Box::new(crate::rules::missing_access_control::MissingAccessControlRule),
            Box::new(crate::rules::type_confusion::TypeConfusionRule),
            Box::new(crate::rules::improper_token_handling::ImproperTokenHandlingRule),
        ];
        Self { rules }
    }

    /// Create analyzer with specific rules
    pub fn with_rules(rules: Vec<Box<dyn LintRule>>) -> Self {
        Self { rules }
    }

    /// Analyze a rust file and return diagnostics
    pub fn analyze_file(&self, file_path: &str, content: &str) -> Result<Vec<Diagnostic>> {
        // Validate the file parses correctly before going parallel
        let _ = syn::parse_file(content)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", file_path, e))?;

        // IMPORTANT: syn::File is NOT Send+Sync (proc_macro2 uses Rc internally),
        // so we cannot parse once and share &syntax across rayon threads.
        // Instead, each thread receives the raw source string (which IS Send+Sync)
        // and parses its own local copy of the AST.
        use rayon::prelude::*;
        let diagnostics = self
            .rules
            .par_iter()
            .flat_map(|rule| match syn::parse_file(content) {
                Ok(syntax) => rule.check(file_path, &syntax),
                Err(_) => vec![],
            })
            .collect();

        Ok(diagnostics)
    }

    /// Analyze with specific rules only
    pub fn analyze_file_with_rules(
        &self,
        file_path: &str,
        content: &str,
        rule_ids: &[&str],
    ) -> Result<Vec<Diagnostic>> {
        // Validate parse before going parallel
        let _ = syn::parse_file(content)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", file_path, e))?;

        use rayon::prelude::*;
        let diagnostics = self
            .rules
            .par_iter()
            .filter(|rule| rule_ids.contains(&rule.rule_id()))
            .flat_map(|rule| match syn::parse_file(content) {
                Ok(syntax) => rule.check(file_path, &syntax),
                Err(_) => vec![],
            })
            .collect();

        Ok(diagnostics)
    }

    /// Filter diagnostics by severity
    pub fn filter_by_severity(
        diagnostics: Vec<Diagnostic>,
        min_severity: Severity,
    ) -> Vec<Diagnostic> {
        diagnostics
            .into_iter()
            .filter(|d| d.severity >= min_severity)
            .collect()
    }

    /// Sort diagnostics by file and line number
    pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
        diagnostics.sort_by(|a, b| {
            let file_cmp = a.span.file.cmp(&b.span.file);
            if file_cmp != std::cmp::Ordering::Equal {
                return file_cmp;
            }
            let line_cmp = a.span.line.cmp(&b.span.line);
            if line_cmp != std::cmp::Ordering::Equal {
                return line_cmp;
            }
            a.span.column.cmp(&b.span.column)
        });
    }

    /// Get list of all available rules
    pub fn list_rules(&self) -> Vec<(&'static str, Severity)> {
        self.rules
            .iter()
            .map(|rule| (rule.rule_id(), rule.default_severity()))
            .collect()
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = Analyzer::new();
        let rules = analyzer.list_rules();
        assert!(rules.len() >= 20, "Should have at least 20 rules");
    }

    #[test]
    fn test_analyze_clean_file() {
        let analyzer = Analyzer::new();
        let content = r#"
            pub fn clean_fn() -> u32 {
                let x: u32 = 1;
                x
            }
        "#;
        let result = analyzer.analyze_file("test.rs", content);
        assert!(result.is_ok(), "Clean file should parse without error");
    }

    #[test]
    fn test_analyze_invalid_rust() {
        let analyzer = Analyzer::new();
        let result = analyzer.analyze_file("bad.rs", "this is not valid rust @@@@");
        assert!(result.is_err(), "Invalid Rust should return an error");
    }
}
