use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct ImproperTokenHandlingRule;

impl LintRule for ImproperTokenHandlingRule {
    fn rule_id(&self) -> &'static str {
        "improper_token_handling"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = TokenHandlingVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct TokenHandlingVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl TokenHandlingVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for TokenHandlingVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let code_str = quote::quote!(#node).to_string();
        let fn_name = node.sig.ident.to_string();

        // Check for token transfer functions without proper validation
        if (fn_name.contains("transfer") || fn_name.contains("send")) && code_str.contains("invoke")
        {
            let missing_validations = !code_str.contains("require_auth")
                && !code_str.contains("assert")
                && !code_str.contains("check");

            if missing_validations {
                let diag = Diagnostic::new(
                    "improper_token_handling",
                    Severity::Error,
                    "Token transfer without proper sender/receiver validation",
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion(
                    "Validate sender authorization and receiver validity before transfer",
                );

                self.diagnostics.push(diag);
            }
        }
        syn::visit::visit_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_created() {
        let rule = ImproperTokenHandlingRule;
        assert_eq!(rule.rule_id(), "improper_token_handling");
    }
}
