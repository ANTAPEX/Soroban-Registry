use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct MissingAccessControlRule;

impl LintRule for MissingAccessControlRule {
    fn rule_id(&self) -> &'static str {
        "missing_access_control"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = AccessControlVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct AccessControlVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl AccessControlVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for AccessControlVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if matches!(node.vis, syn::Visibility::Public(_)) {
            let code_str = quote::quote!(#node).to_string();
            let fn_name = node.sig.ident.to_string();

            // Check for admin functions without admin checks
            if (fn_name.contains("admin")
                || fn_name.contains("unpause")
                || fn_name.contains("withdraw"))
                && !code_str.contains("admin")
                && !code_str.contains("owner")
                && !code_str.contains("require_auth")
            {
                let diag = Diagnostic::new(
                    "missing_access_control",
                    Severity::Error,
                    format!(
                        "Admin-level function `{}` missing access control check",
                        fn_name
                    ),
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion("Add authorization check comparing caller to admin address");

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
        let rule = MissingAccessControlRule;
        assert_eq!(rule.rule_id(), "missing_access_control");
    }
}
