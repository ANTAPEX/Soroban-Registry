use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct LargeDataInStorageRule;

impl LintRule for LargeDataInStorageRule {
    fn rule_id(&self) -> &'static str {
        "large_data_in_storage"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = LargeDataVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct LargeDataVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl LargeDataVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for LargeDataVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::MethodCall(method_call) = node {
            let method_name = &method_call.method;
            if method_name == "set" {
                let code_str = quote::quote!(#node).to_string();
                if code_str.contains("Vec") || code_str.contains("Map") {
                    let diag = Diagnostic::new(
                        "large_data_in_storage",
                        Severity::Info,
                        "Storing unbounded collection (Vec/Map) in persistent storage",
                        &self.file,
                        1,
                        0,
                    )
                    .with_suggestion(
                        "Consider adding size bounds or pagination for large datasets",
                    );

                    self.diagnostics.push(diag);
                }
            }
        }
        syn::visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_created() {
        let rule = LargeDataInStorageRule;
        assert_eq!(rule.rule_id(), "large_data_in_storage");
    }
}
