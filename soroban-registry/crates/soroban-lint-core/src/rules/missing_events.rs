use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct MissingEventsRule;

impl LintRule for MissingEventsRule {
    fn rule_id(&self) -> &'static str {
        "missing_events"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = MissingEventsVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct MissingEventsVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl MissingEventsVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for MissingEventsVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // Check if public state-changing function
        if matches!(node.vis, syn::Visibility::Public(_)) {
            let code_str = quote::quote!(#node).to_string();
            let fn_name = node.sig.ident.to_string();

            // Check if it modifies state but doesn't emit events
            if (code_str.contains(".set(") || code_str.contains("storage()."))
                && !code_str.contains("publish")
                && !fn_name.starts_with("get")
                && !fn_name.starts_with("view")
            {
                let diag = Diagnostic::new(
                    "missing_events",
                    Severity::Info,
                    format!("State-changing function `{}` does not emit events", fn_name),
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion(
                    "Consider emitting an event for state changes using env.events().publish()",
                );

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
        let rule = MissingEventsRule;
        assert_eq!(rule.rule_id(), "missing_events");
    }
}
