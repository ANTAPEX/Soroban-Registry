use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct UncheckedArithmeticRule;

impl LintRule for UncheckedArithmeticRule {
    fn rule_id(&self) -> &'static str {
        "unchecked_arithmetic"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = UncheckedArithmeticVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct UncheckedArithmeticVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl UncheckedArithmeticVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for UncheckedArithmeticVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Binary(bin_expr) = node {
            match bin_expr.op {
                syn::BinOp::Add(_)
                | syn::BinOp::Sub(_)
                | syn::BinOp::Mul(_)
                | syn::BinOp::Div(_) => {
                    let code_str = quote::quote!(#node).to_string();
                    // Only flag if not using checked variant
                    if !code_str.contains("checked_") && !code_str.contains("saturating_") {
                        let diag = Diagnostic::new(
                            "unchecked_arithmetic",
                            Severity::Error,
                            "Arithmetic operation without overflow check",
                            &self.file,
                            1,
                            0,
                        )
                        .with_suggestion(
                            "Use checked_add, checked_sub, checked_mul, or saturating_* variants",
                        );

                        self.diagnostics.push(diag);
                    }
                }
                _ => {}
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
        let rule = UncheckedArithmeticRule;
        assert_eq!(rule.rule_id(), "unchecked_arithmetic");
    }
}
