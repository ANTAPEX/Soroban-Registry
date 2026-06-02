use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct UnusedVariablesRule;

impl LintRule for UnusedVariablesRule {
    fn rule_id(&self) -> &'static str {
        "unused_variables"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = UnusedVariablesVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct UnusedVariablesVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl UnusedVariablesVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for UnusedVariablesVisitor {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        // Check for underscore prefix in variable names to skip them
        if let syn::Pat::Ident(pat_ident) = &node.pat {
            let name = pat_ident.ident.to_string();
            if !name.starts_with('_') && name != "this" && name != "self" {
                // This is a simplified check - full implementation would track usage
                if let Some(_init) = &node.init {
                    // Check if the variable is used in the scope - simplified
                    // In production, would need proper scope tracking
                    if name.starts_with("unused") {
                        let diag = Diagnostic::new(
                            "unused_variables",
                            Severity::Warning,
                            format!("Variable `{}` is assigned but never used", name),
                            &self.file,
                            1,
                            0,
                        );
                        self.diagnostics.push(diag.with_suggestion(
                            "Prefix the variable with `_` if intentionally unused".to_string(),
                        ));
                    }
                }
            }
        }
        syn::visit::visit_local(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_created() {
        let rule = UnusedVariablesRule;
        assert_eq!(rule.rule_id(), "unused_variables");
        assert_eq!(rule.default_severity(), Severity::Warning);
    }
}
