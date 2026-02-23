
use crate::checklist::all_checks;
use shared::models::{CheckStatus, DetectionMethod};
use std::collections::HashMap;

#[derive(Debug)]
pub struct DetectionResult {
    pub status: CheckStatus,
    pub evidence: Option<String>,
}

pub fn detect_all(source: &str) -> HashMap<String, DetectionResult> {
    let checks = all_checks();
    let mut results = HashMap::new();
    let lines: Vec<&str> = source.lines().collect();

    for check in checks {
        let patterns = match &check.detection {
            DetectionMethod::Automatic { patterns } => patterns.clone(),
            DetectionMethod::SemiAutomatic { patterns } => patterns.clone(),
            DetectionMethod::Manual => continue,
        };

        let result = match check.id {
            "IV-001" => detect_unwrap(&lines),
            "IV-002" => detect_expect(&lines),
            "IV-006" => detect_panic_macro(&lines),
            "IV-009" => detect_direct_index(&lines),
            "AC-001" => detect_require_auth(&lines, &["admin", "owner", "operator"]),
            "AC-002" => detect_transfer_without_auth(&lines),
            "AC-007" => detect_init_guard(&lines),
            "AC-008" => detect_upgrade_guard(&lines),
            "NS-001" => detect_unchecked_arithmetic(&lines),
            "NS-002" => detect_division_by_zero_guard(&lines),
            "NS-005" => detect_truncating_cast(&lines),
            "AA-001" => detect_require_auth_present(&lines),
            "EH-002" => detect_silent_discard(&lines),
            "EH-004" => detect_storage_none_handled(&lines),
            "SM-001" => detect_ttl_extension(&lines),
            "SM-002" => detect_instance_ttl(&lines),
            "SM-003" => detect_state_before_call(&lines),
            "TS-001" => detect_token_transfer_error(&lines),
            "EL-001" => detect_events_on_transfers(&lines),
            "DS-001" => detect_contracttype(&lines),
            "SP-001" => detect_datakey_enum(&lines),
            "SP-005" => detect_raw_storage_keys(&lines), 
            "RL-001" => detect_bounded_loops(&lines),
            _ => detect_generic(&lines, &patterns),
        };

        results.insert(check.id.to_string(), result);
    }

    results
}



fn detect_unwrap(lines: &[&str]) -> DetectionResult {
    for (i, line) in lines.iter().enumerate() {
        if is_test_line(line) || line.trim_start().starts_with("//") {
            continue;
        }
        if line.contains(".unwrap()") {
            return DetectionResult {
                status: CheckStatus::Failed,
                evidence: Some(format!("Line {}: {}", i + 1, line.trim())),
            };
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_expect(lines: &[&str]) -> DetectionResult {
    for (i, line) in lines.iter().enumerate() {
        if is_test_line(line) || line.trim_start().starts_with("//") {
            continue;
        }
        if line.contains(".expect(") {
            return DetectionResult {
                status: CheckStatus::Failed,
                evidence: Some(format!("Line {}: {}", i + 1, line.trim())),
            };
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_panic_macro(lines: &[&str]) -> DetectionResult {
    for (i, line) in lines.iter().enumerate() {
        if is_test_line(line) || line.trim_start().starts_with("//") {
            continue;
        }
        if line.contains("panic!(") {
            return DetectionResult {
                status: CheckStatus::Failed,
                evidence: Some(format!("Line {}: {}", i + 1, line.trim())),
            };
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_direct_index(lines: &[&str]) -> DetectionResult {
    let risky = ["[i]", "[idx]", "[index]", "[n]", "[pos]", "]["];
    for (i, line) in lines.iter().enumerate() {
        if is_test_line(line) || line.trim_start().starts_with("//") {
            continue;
        }
        for pat in &risky {
            if line.contains(pat) {
                return DetectionResult {
                    status: CheckStatus::Failed,
                    evidence: Some(format!("Line {}: {}", i + 1, line.trim())),
                };
            }
        }
        if let Some(bracket_pos) = line.find('[') {
            let after = &line[bracket_pos + 1..];
            let digit_end = after.chars().take_while(|c| c.is_ascii_digit()).count();
            if digit_end > 0 && after.chars().nth(digit_end) == Some(']') {
                return DetectionResult {
                    status: CheckStatus::Failed,
                    evidence: Some(format!("Line {}: {}", i + 1, line.trim())),
                };
            }
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_require_auth(lines: &[&str], privilege_keywords: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    let has_auth = source.contains("require_auth()") || source.contains("require_auth_for_args");
    let has_privileged = privilege_keywords.iter().any(|kw| source.contains(kw));
    if has_privileged && !has_auth {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("Privileged function found but no require_auth() call detected".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_transfer_without_auth(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    let has_transfer = source.contains("transfer(") || source.contains("transfer_from(");
    let has_auth = source.contains("require_auth()") || source.contains("require_auth_for_args");
    if has_transfer && !has_auth {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("Transfer call found with no require_auth() in scope".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_init_guard(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    let has_init = source.contains("fn initialize") || source.contains("fn init(");
    let has_guard = source.contains("AlreadyInitialized")
        || source.contains("is_initialized")
        || source.contains("DataKey::Initialized");
    if has_init && !has_guard {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("initialize() found but no re-initialization guard detected".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_upgrade_guard(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    let has_upgrade =
        source.contains("fn upgrade") || source.contains("update_current_contract_wasm");
    let has_auth = source.contains("require_auth()");
    if has_upgrade && !has_auth {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("upgrade() found without require_auth() protection".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_unchecked_arithmetic(lines: &[&str]) -> DetectionResult {
    let safe_patterns = [
        "checked_add",
        "checked_sub",
        "checked_mul",
        "checked_div",
        "saturating_add",
        "saturating_sub",
        "saturating_mul",
    ];
    let source = lines.join("\n");
    let has_arithmetic = lines.iter().any(|l| {
        let t = l.trim();
        !t.starts_with("//")
            && (t.contains(" + ") || t.contains(" - ") || t.contains(" * "))
            && !t.contains("\"")
            && !t.contains("..")
    });
    let has_safe_ops = safe_patterns.iter().any(|p| source.contains(p));
    if has_arithmetic && !has_safe_ops {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("Arithmetic found without checked_add/sub/mul equivalents".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_division_by_zero_guard(lines: &[&str]) -> DetectionResult {
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("//") {
            continue;
        }
        if (t.contains("/ ") || t.contains("/=")) && !t.contains("//") && !t.contains("\"") {
            let start = i.saturating_sub(5);
            let end = (i + 5).min(lines.len());
            let window = lines[start..end].join("\n");
            if !window.contains("require!") && !window.contains("!= 0") && !window.contains("> 0") {
                return DetectionResult {
                    status: CheckStatus::Failed,
                    evidence: Some(format!(
                        "Line {}: Division without denominator guard: {}",
                        i + 1,
                        t
                    )),
                };
            }
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_truncating_cast(lines: &[&str]) -> DetectionResult {
    let dangerous = [
        "as i32", "as u32", "as i64", "as u64", "as i8", "as u8", "as usize",
    ];
    for (i, line) in lines.iter().enumerate() {
        if is_test_line(line) {
            continue;
        }
        for cast in &dangerous {
            if line.contains(cast) {
                return DetectionResult {
                    status: CheckStatus::Failed,
                    evidence: Some(format!(
                        "Line {}: Truncating cast `{}`: {}",
                        i + 1,
                        cast,
                        line.trim()
                    )),
                };
            }
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_require_auth_present(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    if source.contains("require_auth()") || source.contains("require_auth_for_args") {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    } else {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("No require_auth() found anywhere in the contract".into()),
        }
    }
}

fn detect_silent_discard(lines: &[&str]) -> DetectionResult {
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("//") {
            continue;
        }
        if t.starts_with("let _ =") || t.starts_with("let _r =") || t.contains("; let _") {
            return DetectionResult {
                status: CheckStatus::Failed,
                evidence: Some(format!("Line {}: Silent error discard: {}", i + 1, t)),
            };
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_storage_none_handled(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    if source.contains("storage().get") || source.contains("storage().persistent().get") {
        let has_handler = source.contains(".ok_or")
            || source.contains(".unwrap_or")
            || source.contains("if let Some")
            || source.contains("match ");
        if !has_handler {
            return DetectionResult {
                status: CheckStatus::Failed,
                evidence: Some("storage().get() found without None-case handling".into()),
            };
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_ttl_extension(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    let has_persistent = source.contains("storage().persistent()");
    let has_ttl = source.contains("extend_ttl");
    if has_persistent && !has_ttl {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("Persistent storage used but extend_ttl() never called".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_instance_ttl(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    if source.contains("storage().instance()") && !source.contains("instance().extend_ttl") {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("instance() storage used but extend_ttl never called on it".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_state_before_call(lines: &[&str]) -> DetectionResult {
    let mut in_fn = false;
    let mut saw_external_call = false;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("pub fn ") || t.starts_with("fn ") {
            in_fn = true;
            saw_external_call = false;
        }
        if t.starts_with('}') && in_fn {
            in_fn = false;
            saw_external_call = false;
        }
        if in_fn {
            if (t.contains("_client.") || t.contains("Client::"))
                && !t.contains("Client::new")
                && !t.contains("//")
            {
                saw_external_call = true;
            }
            if saw_external_call && (t.contains("storage().set") || t.contains(".set(")) {
                return DetectionResult {
                    status: CheckStatus::Failed,
                    evidence: Some(format!(
                        "Line {}: State written after external call — CEI violation: {}",
                        i + 1,
                        t
                    )),
                };
            }
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_token_transfer_error(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    if source.contains("TokenClient") && source.contains(".transfer(") {
        for (i, line) in lines.iter().enumerate() {
            if line.contains(".transfer(") && !line.contains("?") && !line.contains("match") {
                let next = lines.get(i + 1).unwrap_or(&"");
                if !next.contains("?") && !next.contains("match") && !next.contains("unwrap_or") {
                    return DetectionResult {
                        status: CheckStatus::Failed,
                        evidence: Some(format!(
                            "Line {}: Token transfer result may not be propagated: {}",
                            i + 1,
                            line.trim()
                        )),
                    };
                }
            }
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_events_on_transfers(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    let has_transfer = source.contains(".transfer(")
        || source.contains("fn withdraw")
        || source.contains("fn deposit");
    let has_events = source.contains("events().publish") || source.contains("env.events()");
    if has_transfer && !has_events {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("Transfer/deposit/withdraw found without event emissions".into()),
        }
    } else {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    }
}

fn detect_contracttype(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    if source.contains("#[contracttype]") {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    } else {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some(
                "#[contracttype] not found — custom types may not serialize correctly".into(),
            ),
        }
    }
}

fn detect_datakey_enum(lines: &[&str]) -> DetectionResult {
    let source = lines.join("\n");
    if source.contains("DataKey") && source.contains("#[contracttype]") {
        DetectionResult {
            status: CheckStatus::Passed,
            evidence: None,
        }
    } else {
        DetectionResult {
            status: CheckStatus::Failed,
            evidence: Some("DataKey enum not found or missing #[contracttype]".into()),
        }
    }
}


fn detect_raw_storage_keys(lines: &[&str]) -> DetectionResult {
    let vulnerable_patterns = ["symbol_short!(", "Symbol::new(", "&\""];
    
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if is_test_line(line) || t.starts_with("//") {
            continue;
        }
        
        if t.contains("storage()") && (t.contains(".set(") || t.contains(".get(") || t.contains(".has(")) {
            for pat in &vulnerable_patterns {
                if t.contains(pat) {
                    return DetectionResult {
                        status: CheckStatus::Failed,
                        evidence: Some(format!(
                            "Line {}: Raw string or Symbol used as storage key. Migrate to an exhaustive DataKey enum. Code: {}",
                            i + 1,
                            t
                        )),
                    };
                }
            }
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_bounded_loops(lines: &[&str]) -> DetectionResult {
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("//") {
            continue;
        }
        if (t.starts_with("for ") && t.contains(".iter()"))
            || (t.starts_with("for ") && t.contains("in &"))
        {
            let start = i.saturating_sub(10);
            let end = (i + 3).min(lines.len());
            let window = lines[start..end].join("\n");
            if !window.contains("MAX") && !window.contains("max_") && !window.contains(".len() <=")
            {
                return DetectionResult {
                    status: CheckStatus::Failed,
                    evidence: Some(format!(
                        "Line {}: Unbounded loop without MAX guard: {}",
                        i + 1,
                        t
                    )),
                };
            }
        }
    }
    DetectionResult {
        status: CheckStatus::Passed,
        evidence: None,
    }
}

fn detect_generic(lines: &[&str], good_patterns: &[String]) -> DetectionResult {
    let source = lines.join("\n");
    for pat in good_patterns {
        if source.contains(pat.as_str()) {
            return DetectionResult {
                status: CheckStatus::Passed,
                evidence: None,
            };
        }
    }
    DetectionResult {
        status: CheckStatus::Failed,
        evidence: Some(format!(
            "None of the expected patterns found: {}",
            good_patterns.join(", ")
        )),
    }
}

fn is_test_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("#[test]") || t.starts_with("#[cfg(test)]")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_SOURCE: &str = r#"
#[contracttype]
pub enum DataKey { Admin, Balance(Address) }
#[contractimpl]
impl MyContract {
    pub fn initialize(env: Env, admin: Address) {
        require!(!env.storage().persistent().has(&DataKey::Initialized), ContractError::AlreadyInitialized);
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().extend_ttl(&DataKey::Admin, 100, 200);
        env.storage().instance().extend_ttl(100, 200);
        env.events().publish((symbol_short!("init"),), (admin.clone(),));
    }
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let balance = env.storage().persistent().get(&DataKey::Balance(from.clone())).ok_or(ContractError::NotFound)?;
        let new_balance = balance.checked_sub(amount).ok_or(ContractError::Overflow)?;
        env.storage().persistent().set(&DataKey::Balance(from.clone()), &new_balance);
        env.events().publish((symbol_short!("transfer"),), (from, to, amount));
    }
}
"#;

    const BAD_SOURCE: &str = r#"
pub fn transfer(env: Env, from: Address, amount: i128) {
    let balance = env.storage().get(&DataKey::Balance(from.clone())).unwrap();
    let new_val = balance - amount;
    panic!("not implemented");
    let _ = do_something();
}
"#;

    #[test]
    fn good_source_has_fewer_failures() {
        let good = detect_all(GOOD_SOURCE);
        let bad = detect_all(BAD_SOURCE);
        let good_fails = good
            .values()
            .filter(|r| r.status == CheckStatus::Failed)
            .count();
        let bad_fails = bad
            .values()
            .filter(|r| r.status == CheckStatus::Failed)
            .count();
        assert!(
            bad_fails > good_fails,
            "bad({}) should exceed good({})",
            bad_fails,
            good_fails
        );
    }

    #[test]
    fn unwrap_detection_works() {
        assert_eq!(
            detect_unwrap(&["let x = foo.unwrap();"]).status,
            CheckStatus::Failed
        );
        assert_eq!(
            detect_unwrap(&["let x = foo.ok_or(Err::E)?"]).status,
            CheckStatus::Passed
        );
    }

    #[test]
    fn panic_detection_works() {
        assert_eq!(
            detect_panic_macro(&[r#"panic!("bad");"#]).status,
            CheckStatus::Failed
        );
    }

    #[test]
    fn raw_storage_keys_detection_works() {
        let bad_code = [
            "pub fn store(env: Env) {",
            "    env.storage().instance().set(&symbol_short!(\"admin\"), &1);",
            "}"
        ];
        assert_eq!(
            detect_raw_storage_keys(&bad_code).status,
            CheckStatus::Failed
        );

        let good_code = [
            "pub fn store(env: Env) {",
            "    env.storage().instance().set(&DataKey::Admin, &1);",
            "}"
        ];
        assert_eq!(
            detect_raw_storage_keys(&good_code).status,
            CheckStatus::Passed
        );
    }
}