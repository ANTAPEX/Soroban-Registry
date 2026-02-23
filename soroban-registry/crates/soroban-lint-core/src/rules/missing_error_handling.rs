use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct MissingErrorHandlingRule;

impl LintRule for MissingErrorHandlingRule {
    fn rule_id(&self) -> &'static str {
        "missing_error_handling"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = ErrorHandlingVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct ErrorHandlingVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
    in_test: bool,
}

impl ErrorHandlingVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
            in_test: false,
        }
    }
}

impl<'ast> Visit<'ast> for ErrorHandlingVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // Check if in test function
        let is_test = node.attrs.iter().any(|attr| {
            attr.path().is_ident("test") || attr.path().is_ident("tokio::test")
        });
        
        let prev_test = self.in_test;
        self.in_test = is_test || self.in_test;

        // Let syn::visit handle visiting the block - don't visit manually to avoid double-visiting
        syn::visit::visit_item_fn(self, node);

        self.in_test = prev_test;
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        // Check for .unwrap() and .expect() calls
        if let syn::Expr::MethodCall(method_call) = node {
            let method_name = &method_call.method;
            if (method_name == "unwrap" || method_name == "expect") && !self.in_test {
                let diag = Diagnostic::new(
                    "missing_error_handling",
                    Severity::Error,
                    format!(
                        "Calling .{}() on Result or Option without proper error handling",
                        method_name
                    ),
                    &self.file,
                    1,
                    0,
                );
                self.diagnostics.push(
                    diag.with_suggestion(format!(
                        "Use `?` operator or match statement instead of .{}()",
                        method_name
                    )),
                );
            }
        }
        syn::visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unwrap_in_public_fn() {
        let code = r#"
            pub fn transfer(env: Env) {
                let val = env.storage().get("key").unwrap();
            }
        "#;
        let syntax: syn::File = syn::parse_str(code).unwrap();
        let rule = MissingErrorHandlingRule;
        let diags = rule.check("test.rs", &syntax);
        assert!(!diags.is_empty(), "Should detect unwrap");
    }

    #[test]
    fn no_false_positive_for_test_code() {
        let code = r#"
            #[test]
            fn my_test() {
                let val = some_fn().unwrap();
            }
        "#;
        let syntax: syn::File = syn::parse_str(code).unwrap();
        let rule = MissingErrorHandlingRule;
        let diags = rule.check("test.rs", &syntax);
        assert!(diags.is_empty(), "Should not error in test code");
    }
}
