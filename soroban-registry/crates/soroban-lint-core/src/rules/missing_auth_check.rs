use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct MissingAuthCheckRule;

impl LintRule for MissingAuthCheckRule {
    fn rule_id(&self) -> &'static str {
        "missing_auth_check"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = AuthCheckVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct AuthCheckVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl AuthCheckVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for AuthCheckVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // Check if public function
        if matches!(node.vis, syn::Visibility::Public(_)) {
            let code_str = quote::quote!(#node).to_string();

            // Check if it has require_auth
            if !code_str.contains("require_auth") && !code_str.contains("env.require_auth") {
                // Check if it modifies state - handle quote::quote! spacing variations
                let has_set = code_str.contains(". set (") || code_str.contains(".set(");
                let has_storage = code_str.contains("storage ()") || code_str.contains("storage()");
                if has_set || has_storage {
                    let fn_name = node.sig.ident.to_string();
                    if !fn_name.starts_with("get") && !fn_name.starts_with("view") {
                        let diag = Diagnostic::new(
                            "missing_auth_check",
                            Severity::Error,
                            format!("Public function `{}` may lack authorization check", fn_name),
                            &self.file,
                            1,
                            0,
                        )
                        .with_suggestion("Add env.require_auth(&caller) to validate permissions");

                        self.diagnostics.push(diag);
                    }
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
        let rule = MissingAuthCheckRule;
        assert_eq!(rule.rule_id(), "missing_auth_check");
    }

    #[test]
    fn flags_missing_auth_on_transfer() {
        let source = r#"
            use soroban_sdk::{Env, Address, Symbol};
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                env.storage().persistent().set(&Symbol::new(&env, "balance"), &amount);
            }
        "#;
        let syntax = syn::parse_file(source).expect("valid syntax");
        let rule = MissingAuthCheckRule;
        let diags = rule.check("test.rs", &syntax);
        assert!(!diags.is_empty());
    }
}
