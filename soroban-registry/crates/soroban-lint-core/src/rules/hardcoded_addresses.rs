use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct HardcodedAddressesRule;

impl LintRule for HardcodedAddressesRule {
    fn rule_id(&self) -> &'static str {
        "hardcoded_addresses"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = HardcodedAddressVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct HardcodedAddressVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl HardcodedAddressVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for HardcodedAddressVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Lit(expr_lit) = node {
            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                let value = lit_str.value();
                // Check for hardcoded addresses (very simplified heuristic)
                if (value.starts_with("C") || value.starts_with("G")) && value.len() > 50 {
                    let diag = Diagnostic::new(
                        "hardcoded_addresses",
                        Severity::Warning,
                        "Hardcoded address detected - consider using configuration",
                        &self.file,
                        1,
                        0,
                    )
                    .with_suggestion(
                        "Move hardcoded address to environment variable or config file",
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
        let rule = HardcodedAddressesRule;
        assert_eq!(rule.rule_id(), "hardcoded_addresses");
    }
}
