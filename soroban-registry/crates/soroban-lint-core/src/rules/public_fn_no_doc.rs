use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct PublicFnNoDocRule;

impl LintRule for PublicFnNoDocRule {
    fn rule_id(&self) -> &'static str {
        "public_fn_no_doc"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = PublicFnDocVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct PublicFnDocVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl PublicFnDocVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for PublicFnDocVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if matches!(node.vis, syn::Visibility::Public(_)) {
            let has_doc = node.attrs.iter().any(|attr| attr.path().is_ident("doc"));

            if !has_doc {
                let fn_name = node.sig.ident.to_string();
                if !fn_name.starts_with("test_") {
                    let diag = Diagnostic::new(
                        "public_fn_no_doc",
                        Severity::Info,
                        format!("Public function `{}` lacks documentation", fn_name),
                        &self.file,
                        1,
                        0,
                    )
                    .with_suggestion(
                        "Add doc comments describing the function's purpose and parameters",
                    );

                    self.diagnostics.push(diag);
                }
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
        let rule = PublicFnNoDocRule;
        assert_eq!(rule.rule_id(), "public_fn_no_doc");
    }
}
