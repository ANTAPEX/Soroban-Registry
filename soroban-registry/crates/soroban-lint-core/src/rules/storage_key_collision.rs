use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use std::collections::HashSet;
use syn::visit::Visit;

pub struct StorageKeyCollisionRule;

impl LintRule for StorageKeyCollisionRule {
    fn rule_id(&self) -> &'static str {
        "storage_key_collision"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = StorageKeyVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct StorageKeyVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
    storage_keys: HashSet<String>,
}

impl StorageKeyVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
            storage_keys: HashSet::new(),
        }
    }
}

impl<'ast> Visit<'ast> for StorageKeyVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Lit(expr_lit) = node {
            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                let key = lit_str.value();
                if key.len() > 3 && key.contains("key") {
                    if self.storage_keys.contains(&key) {
                        let diag = Diagnostic::new(
                            "storage_key_collision",
                            Severity::Error,
                            format!("Duplicate storage key literal: \"{}\"", key),
                            &self.file,
                            1,
                            0,
                        )
                        .with_suggestion("Use unique key constants or enums for storage keys");

                        self.diagnostics.push(diag);
                    }
                    self.storage_keys.insert(key);
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
        let rule = StorageKeyCollisionRule;
        assert_eq!(rule.rule_id(), "storage_key_collision");
    }
}
