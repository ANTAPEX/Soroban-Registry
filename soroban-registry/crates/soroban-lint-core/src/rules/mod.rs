use crate::diagnostic::{Diagnostic, Severity};

pub mod deprecated_api_usage;
pub mod direct_storage_clear;
pub mod hardcoded_addresses;
pub mod improper_token_handling;
pub mod inefficient_clones;
pub mod integer_overflow;
pub mod large_data_in_storage;
pub mod missing_access_control;
pub mod missing_auth_check;
pub mod missing_error_handling;
pub mod missing_events;
pub mod panic_in_contract;
pub mod public_fn_no_doc;
pub mod reentrancy;
pub mod storage_key_collision;
pub mod type_confusion;
pub mod unbounded_loops;
pub mod unchecked_arithmetic;
pub mod unsafe_patterns;
pub mod unused_variables;

/// Trait that all lint rules must implement
pub trait LintRule: Send + Sync {
    /// Unique identifier for this rule
    fn rule_id(&self) -> &'static str;

    /// Default severity level for this rule
    fn default_severity(&self) -> Severity;

    /// Whether this rule supports auto-fixing
    fn supports_fix(&self) -> bool {
        false
    }

    /// Run the lint check on the given file
    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic>;
}

/// Trait to visit AST nodes - helper for rule implementation
pub trait AstVisitor {
    fn visit_item_fn(&mut self, _node: &syn::ItemFn) {}
    fn visit_expr(&mut self, _node: &syn::Expr) {}
    fn visit_local(&mut self, _node: &syn::Local) {}
}
