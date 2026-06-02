use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct TypeConfusionRule;

impl LintRule for TypeConfusionRule {
    fn rule_id(&self) -> &'static str {
        "type_confusion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = TypeConfusionVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct TypeConfusionVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl TypeConfusionVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for TypeConfusionVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Cast(_cast) = node {
            let code_str = quote::quote!(#node).to_string();
            // Check for unsafe casts between types like Val
            if code_str.contains("as") && (code_str.contains("Val") || code_str.contains("u64")) {
                let diag = Diagnostic::new(
                    "type_confusion",
                    Severity::Error,
                    "Unsafe type cast detected - verify type compatibility",
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion("Use proper type conversion methods from Soroban SDK");

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
    fn rule_created() {
        let rule = TypeConfusionRule;
        assert_eq!(rule.rule_id(), "type_confusion");
    }
}
