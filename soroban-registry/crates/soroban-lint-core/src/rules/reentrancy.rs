use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct ReentrancyRule;

impl LintRule for ReentrancyRule {
    fn rule_id(&self) -> &'static str {
        "reentrancy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = ReentrancyVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct ReentrancyVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl ReentrancyVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for ReentrancyVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let code_str = quote::quote!(#node).to_string();

        // Check for cross-contract calls before state writes
        // Note: quote::quote! may add spaces around parens, so we search for multiple patterns
        let call_idx = first_index(&code_str, &["invoke_contract", "invoke", ". call (", ".call("]);
        let write_idx = first_index(&code_str, &[". set (", ".set(", "storage ()", "storage()", "write"]);

        if let (Some(call_pos), Some(write_pos)) = (call_idx, write_idx) {
            if call_pos < write_pos {
                let diag = Diagnostic::new(
                    "reentrancy",
                    Severity::Error,
                    "Potential reentrancy vulnerability: cross-contract call before state modification",
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion("Perform state updates before external calls (Checks-Effects-Interactions pattern)");

                self.diagnostics.push(diag);
            }
        }

        syn::visit::visit_item_fn(self, node);
    }
}

fn first_index(haystack: &str, needles: &[&str]) -> Option<usize> {
    needles
        .iter()
        .filter_map(|needle| haystack.find(needle))
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_created() {
        let rule = ReentrancyRule;
        assert_eq!(rule.rule_id(), "reentrancy");
    }

    #[test]
    fn flags_call_before_state_write() {
        let source = r#"
            use soroban_sdk::{Env, Address, Symbol};
            pub fn send(env: Env, to: Address, amount: i128) {
                env.invoke_contract::<_, ()>(&to, &Symbol::new(&env, "receive"), (amount,));
                env.storage().persistent().set(&Symbol::new(&env, "balance"), &amount);
            }
        "#;
        let syntax = syn::parse_file(source).expect("valid syntax");
        let rule = ReentrancyRule;
        let diags = rule.check("test.rs", &syntax);
        assert!(!diags.is_empty());
    }

    #[test]
    fn ignores_state_write_before_call() {
        let source = r#"
            use soroban_sdk::{Env, Address, Symbol};
            pub fn send(env: Env, to: Address, amount: i128) {
                env.storage().persistent().set(&Symbol::new(&env, "balance"), &amount);
                env.invoke_contract::<_, ()>(&to, &Symbol::new(&env, "receive"), (amount,));
            }
        "#;
        let syntax = syn::parse_file(source).expect("valid syntax");
        let rule = ReentrancyRule;
        let diags = rule.check("test.rs", &syntax);
        assert!(diags.is_empty());
    }
}
