use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct UnsafeUnwrapRule;

impl LintRule for UnsafeUnwrapRule {
    fn rule_id(&self) -> &'static str {
        "unsafe_unwrap"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = UnsafeUnwrapVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }

    fn supports_fix(&self) -> bool {
        true
    }
}

struct UnsafeUnwrapVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
    in_test: bool,
    in_public_fn: bool,
}

impl UnsafeUnwrapVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
            in_test: false,
            in_public_fn: false,
        }
    }
}

impl<'ast> Visit<'ast> for UnsafeUnwrapVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let is_test = node
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("test") || attr.path().is_ident("tokio::test"));

        let is_public = matches!(node.vis, syn::Visibility::Public(_));

        let prev_test = self.in_test;
        let prev_public = self.in_public_fn;

        self.in_test = is_test;
        self.in_public_fn = is_public;

        self.visit_block(&node.block);

        self.in_test = prev_test;
        self.in_public_fn = prev_public;

        syn::visit::visit_item_fn(self, node);
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::MethodCall(method_call) = node {
            let method_name = &method_call.method;
            if method_name == "unwrap" && self.in_public_fn && !self.in_test {
                let diag = Diagnostic::new(
                    "unsafe_unwrap",
                    Severity::Error,
                    "Public function uses .unwrap() on Option/Result which can panic",
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion("Use result?.operator or proper error handling")
                .with_fix("Replace .unwrap() with ? or match statement");

                self.diagnostics.push(diag);
            }
        }
        syn::visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_public_fn_unwrap() {
        let code = r#"
            pub fn get_value() {
                let x = Some(5).unwrap();
            }
        "#;
        let syntax: syn::File = syn::parse_str(code).unwrap();
        let rule = UnsafeUnwrapRule;
        let diags = rule.check("test.rs", &syntax);
        assert!(!diags.is_empty());
    }

    #[test]
    fn supports_fix() {
        let rule = UnsafeUnwrapRule;
        assert!(rule.supports_fix());
    }
}
