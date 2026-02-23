// api/src/checklist.rs
// 50+ security audit checklist items for Soroban smart contracts

use crate::models::{CheckCategory, ChecklistItem, DetectionMethod, Severity};

/// Returns the full static checklist of 50+ security audit items
pub fn all_checks() -> Vec<ChecklistItem> {
    vec![
        // ─────────────────────────────────────────
        // INPUT VALIDATION (10 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "IV-001".into(),
            category: CheckCategory::InputValidation,
            title: "No raw unwrap() on user-controlled values".into(),
            description: "Using .unwrap() on values derived from user input will panic and abort \
                          the contract. All user-supplied Option/Result values must be handled \
                          explicitly with match, if let, or .ok_or().".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::Automatic {
                patterns: vec![".unwrap()".into(), ".unwrap_or_else".into()],
            },
            remediation: "Replace all .unwrap() calls with explicit error handling. Use \
                         `ok_or(ContractError::InvalidInput)?` pattern.".into(),
            references: vec!["https://docs.rs/soroban-sdk/latest/soroban_sdk/".into()],
        },
        ChecklistItem {
            id: "IV-002".into(),
            category: CheckCategory::InputValidation,
            title: "No unguarded .expect() calls".into(),
            description: ".expect() is equivalent to .unwrap() with a message — it still panics. \
                          Panics in Soroban contracts consume resources and abort execution.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::Automatic {
                patterns: vec![".expect(".into()],
            },
            remediation: "Remove all .expect() calls. Return a typed ContractError instead.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-003".into(),
            category: CheckCategory::InputValidation,
            title: "Integer inputs validated against domain bounds".into(),
            description: "User-supplied numeric inputs (amounts, counts, indices) must be \
                          validated against domain-specific min/max bounds before use.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["env.args".into(), "i128::MAX".into(), "u128::MAX".into()],
            },
            remediation: "Add explicit range checks: `require!(amount > 0 && amount <= MAX_AMOUNT)`".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-004".into(),
            category: CheckCategory::InputValidation,
            title: "Address inputs validated with require!".into(),
            description: "Contract addresses passed as arguments should be verified to be \
                          non-zero/non-default and ideally allowlisted before use.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["Address".into(), "require!".into()],
            },
            remediation: "Validate addresses are not default/zero. Use `require!(addr != Address::default())`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-005".into(),
            category: CheckCategory::InputValidation,
            title: "String/Bytes inputs have length limits enforced".into(),
            description: "Unbounded string or byte inputs can inflate ledger entry sizes, \
                          increasing storage fees and potentially DoS-ing the contract.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["String".into(), "Bytes".into(), ".len()".into()],
            },
            remediation: "Add `require!(input.len() <= MAX_LEN)` for all String/Bytes arguments.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-006".into(),
            category: CheckCategory::InputValidation,
            title: "No panic! macros in contract code".into(),
            description: "Direct panic! calls abort the contract and waste fees. All error \
                          conditions must return a typed error.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::Automatic {
                patterns: vec!["panic!(".into()],
            },
            remediation: "Replace `panic!()` with `return Err(ContractError::...)` or use the \
                         `require!` macro which panics with a known error code.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-007".into(),
            category: CheckCategory::InputValidation,
            title: "Vec/Map inputs have element count limits".into(),
            description: "Accepting arbitrarily large vectors or maps can exhaust instruction \
                          budget and cause transaction failures or resource exhaustion.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["Vec<".into(), "Map<".into()],
            },
            remediation: "Enforce `require!(input_vec.len() <= MAX_ITEMS)` on all collection inputs.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-008".into(),
            category: CheckCategory::InputValidation,
            title: "Timestamp inputs validated against reasonable bounds".into(),
            description: "Timestamps provided by callers should be sanity-checked against \
                          env.ledger().timestamp() to prevent invalid deadline/expiry logic.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["ledger().timestamp()".into(), "deadline".into(), "expiry".into()],
            },
            remediation: "Validate: `require!(deadline > env.ledger().timestamp())`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-009".into(),
            category: CheckCategory::InputValidation,
            title: "No index-out-of-bounds risk on Vec accesses".into(),
            description: "Direct indexing with vec[i] panics if out of bounds. Soroban contracts \
                          must use .get(i) and handle the None case.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Automatic {
                patterns: vec!["[i]".into(), "[idx]".into(), "[index]".into()],
            },
            remediation: "Replace `vec[i]` with `vec.get(i).ok_or(ContractError::OutOfBounds)?`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "IV-010".into(),
            category: CheckCategory::InputValidation,
            title: "Enum discriminants validated on deserialization".into(),
            description: "When deserializing user-provided enum values (e.g., from XDR), \
                          invalid discriminant values must be explicitly rejected.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Add a match-all arm that returns ContractError::InvalidInput for \
                         unknown enum variants.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // ACCESS CONTROL (8 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "AC-001".into(),
            category: CheckCategory::AccessControl,
            title: "Admin/owner functions require authorization".into(),
            description: "Functions that modify configuration, upgrade the contract, or transfer \
                          ownership must call require_auth() on the admin address.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["require_auth".into(), "admin".into()],
            },
            remediation: "Call `admin.require_auth()` at the top of every privileged function.".into(),
            references: vec!["https://soroban.stellar.org/docs/learn/authorization".into()],
        },
        ChecklistItem {
            id: "AC-002".into(),
            category: CheckCategory::AccessControl,
            title: "No missing require_auth() on fund-moving operations".into(),
            description: "Any operation that transfers tokens or modifies balances must \
                          authenticate the sender via require_auth().".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["transfer".into(), "require_auth".into()],
            },
            remediation: "Ensure every transfer/withdraw function calls `caller.require_auth()`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AC-003".into(),
            category: CheckCategory::AccessControl,
            title: "Admin key stored in persistent storage, not instance".into(),
            description: "Storing the admin address in instance storage ties it to contract \
                          lifetime incorrectly. Use persistent storage for the admin key.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["instance().set".into(), "DataKey::Admin".into()],
            },
            remediation: "Use `env.storage().persistent().set(&DataKey::Admin, &admin)`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AC-004".into(),
            category: CheckCategory::AccessControl,
            title: "Role-based access uses allowlist, not denylist".into(),
            description: "Access control should use explicit allowlists. Denylists are fragile \
                          because they require all dangerous addresses to be known in advance.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Implement allowlist-based RBAC. Deny by default, permit explicitly.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AC-005".into(),
            category: CheckCategory::AccessControl,
            title: "Two-step admin transfer implemented".into(),
            description: "Admin ownership transfer should be a two-step process (propose + accept) \
                          to prevent accidentally transferring control to a wrong address.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["transfer_admin".into(), "propose_admin".into(), "accept_admin".into()],
            },
            remediation: "Implement propose_admin(new_admin) + accept_admin() pattern.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AC-006".into(),
            category: CheckCategory::AccessControl,
            title: "No hardcoded addresses for admin/privileged roles".into(),
            description: "Hardcoded addresses in source code cannot be rotated if a key is \
                          compromised. All privileged addresses must be stored in contract state.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["G...".into(), "Address::from_str".into()],
            },
            remediation: "Read privileged addresses from storage, set during initialization only.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AC-007".into(),
            category: CheckCategory::AccessControl,
            title: "Initialization can only be called once".into(),
            description: "The init/constructor function must check for and set an initialized \
                          flag to prevent re-initialization attacks.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["initialize".into(), "DataKey::Initialized".into(), "is_initialized".into()],
            },
            remediation: "Add: `require!(!is_initialized(&env), ContractError::AlreadyInitialized)`".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AC-008".into(),
            category: CheckCategory::AccessControl,
            title: "Contract upgrade restricted to admin".into(),
            description: "The upgrade() function must be protected by admin require_auth(). \
                          Unprotected upgrades allow anyone to replace contract logic.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["upgrade".into(), "env.deployer()".into()],
            },
            remediation: "Gate upgrade: `admin.require_auth(); env.deployer().update_current_contract_wasm(hash)`".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // NUMERICAL SAFETY (8 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "NS-001".into(),
            category: CheckCategory::NumericalSafety,
            title: "No unchecked arithmetic (overflow/underflow risk)".into(),
            description: "In Rust, integer overflow in debug mode panics and in release mode \
                          wraps silently. Soroban contracts must use checked_add/sub/mul or \
                          saturating_ variants for all financial math.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["checked_add".into(), "checked_sub".into(), "checked_mul".into(), "overflow".into()],
            },
            remediation: "Use `a.checked_add(b).ok_or(ContractError::Overflow)?` for all arithmetic.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "NS-002".into(),
            category: CheckCategory::NumericalSafety,
            title: "Division by zero prevented before all divisions".into(),
            description: "Integer division by zero panics in Rust. Every division must be \
                          guarded with a denominator != 0 check.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["/ ".into(), "require!(".into()],
            },
            remediation: "Add `require!(denominator != 0, ContractError::DivisionByZero)` before each `/`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "NS-003".into(),
            category: CheckCategory::NumericalSafety,
            title: "Fixed-point math used for percentages/rates".into(),
            description: "Floating-point is not available in Soroban. Percentage calculations \
                          must use scaled integer math (e.g., basis points x 10_000) to avoid \
                          precision loss.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["BPS".into(), "BASIS_POINTS".into(), "10_000".into(), "1_000_000".into()],
            },
            remediation: "Use basis-point or ray math. Document the scale factor in constants.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "NS-004".into(),
            category: CheckCategory::NumericalSafety,
            title: "Token amount precision consistently handled".into(),
            description: "Mixing token amounts with different decimal precisions without \
                          normalization causes incorrect accounting.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Manual,
            remediation: "Define a canonical internal precision and convert at contract boundaries.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "NS-005".into(),
            category: CheckCategory::NumericalSafety,
            title: "No type casting that silently truncates values".into(),
            description: "Casting from i128 to i64 or u128 to u64 with `as` can silently \
                          truncate. Use TryFrom/TryInto with error handling.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Automatic {
                patterns: vec!["as i64".into(), "as u64".into(), "as i32".into(), "as u32".into()],
            },
            remediation: "Use `i64::try_from(val).map_err(|_| ContractError::Overflow)?`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "NS-006".into(),
            category: CheckCategory::NumericalSafety,
            title: "Rounding direction is intentional and documented".into(),
            description: "Rounding in favor of users can drain contract funds. Round in the \
                          contract's favor (floor for payouts, ceil for deposits).".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Document rounding direction at each calculation site. Prefer ceiling \
                         division for amounts the contract receives.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "NS-007".into(),
            category: CheckCategory::NumericalSafety,
            title: "Large-number multiplication does not intermediate-overflow".into(),
            description: "Multiplying two i64 values before dividing (as in fee calculations) \
                          can overflow i64 even if the final result fits. Upcast to i128 first.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["i128".into(), "as i128".into()],
            },
            remediation: "Cast operands to i128 before multiplication: `(a as i128) * (b as i128) / denom`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "NS-008".into(),
            category: CheckCategory::NumericalSafety,
            title: "Slippage/tolerance parameters validated".into(),
            description: "Slippage tolerances of 0 or >100% must be rejected. Zero slippage \
                          causes all trades to revert; >100% disables the protection entirely.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["slippage".into(), "tolerance".into(), "min_amount_out".into()],
            },
            remediation: "Validate: `require!(slippage_bps > 0 && slippage_bps <= 10_000)`.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // STATE MANAGEMENT (7 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "SM-001".into(),
            category: CheckCategory::StateManagement,
            title: "Storage TTL/expiry extended before access".into(),
            description: "Persistent storage entries expire. If a required entry has expired, \
                          reads return None unexpectedly. TTLs must be bumped on each access.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["extend_ttl".into(), "bump".into()],
            },
            remediation: "Call `env.storage().persistent().extend_ttl(&key, min_ttl, max_ttl)` \
                         on every read/write of persistent data.".into(),
            references: vec!["https://soroban.stellar.org/docs/learn/state-expiration".into()],
        },
        ChecklistItem {
            id: "SM-002".into(),
            category: CheckCategory::StateManagement,
            title: "Contract instance TTL extended in critical functions".into(),
            description: "The contract instance itself can expire. High-traffic contracts must \
                          periodically bump the instance TTL.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["instance().extend_ttl".into()],
            },
            remediation: "Add `env.storage().instance().extend_ttl(min, max)` in hot-path functions.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SM-003".into(),
            category: CheckCategory::StateManagement,
            title: "State changes committed before external calls".into(),
            description: "Writing state after an external/cross-contract call is reentrancy-prone. \
                          Update internal state before invoking other contracts.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["storage().set".into(), "client.".into()],
            },
            remediation: "Follow checks-effects-interactions: validate -> update storage -> call external.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SM-004".into(),
            category: CheckCategory::StateManagement,
            title: "Temporary storage used for transient/in-tx data only".into(),
            description: "Temporary storage is cleared at the end of each transaction. Using it \
                          for data that needs to persist across transactions is a logic error.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["storage().temporary()".into()],
            },
            remediation: "Use temporary() only for intra-tx scratch space. Use persistent() for \
                         any data needed in future transactions.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SM-005".into(),
            category: CheckCategory::StateManagement,
            title: "DataKey enum covers all storage keys exhaustively".into(),
            description: "Using raw string keys or scattered constants for storage keys makes \
                          collision and key-reuse bugs likely. A single exhaustive DataKey enum \
                          is the safe pattern.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["DataKey".into(), "#[contracttype]".into()],
            },
            remediation: "Define all storage keys in a single `#[contracttype] enum DataKey`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SM-006".into(),
            category: CheckCategory::StateManagement,
            title: "Storage migration handled on upgrade".into(),
            description: "When upgrading a contract with changed storage schemas, old data must \
                          be migrated. Unhandled schema changes cause deserialization panics.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["migrate".into(), "upgrade".into(), "version".into()],
            },
            remediation: "Implement a migrate() function called once after each upgrade that \
                         transforms storage from old schema to new schema.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SM-007".into(),
            category: CheckCategory::StateManagement,
            title: "No unbounded storage growth (maps/vecs not growing forever)".into(),
            description: "Maps and vectors stored on-chain that grow without bound will eventually \
                          make the contract too expensive or impossible to interact with.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Use pagination, pruning, or off-chain indexing for unbounded datasets. \
                         Set explicit max-size limits.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // REENTRANCY (5 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "RE-001".into(),
            category: CheckCategory::Reentrancy,
            title: "Checks-Effects-Interactions pattern followed".into(),
            description: "All state mutations must happen before external contract calls to \
                          prevent reentrancy-style double-spend vulnerabilities.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::Manual,
            remediation: "Order code as: 1. input checks, 2. state updates, 3. external calls.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "RE-002".into(),
            category: CheckCategory::Reentrancy,
            title: "No re-entrant call paths through callbacks".into(),
            description: "Contracts that accept callbacks from external contracts must guard \
                          against re-entering the same function through the callback.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["callback".into(), "hook".into()],
            },
            remediation: "Use a reentrancy guard flag stored in temporary storage, cleared on exit.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "RE-003".into(),
            category: CheckCategory::Reentrancy,
            title: "Balances updated before token transfers".into(),
            description: "Internal balance records must be decremented before calling token.transfer() \
                          to prevent reentrancy leading to double-withdrawal.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["transfer".into(), "balance".into()],
            },
            remediation: "Deduct from internal balance, then call `token_client.transfer()`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "RE-004".into(),
            category: CheckCategory::Reentrancy,
            title: "Flash-loan callback does not bypass access controls".into(),
            description: "Flash loan callbacks are invoked by external contracts mid-transaction. \
                          Access control checks must not be bypassable through the callback path.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["flash_loan".into(), "flash".into(), "loan_callback".into()],
            },
            remediation: "Validate the callback caller is the expected flash-loan pool address only.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "RE-005".into(),
            category: CheckCategory::Reentrancy,
            title: "No state-dependent logic after cross-contract calls".into(),
            description: "Reading state after an external call is reading potentially stale state \
                          if re-entry occurred. Cache needed state values before the call.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Manual,
            remediation: "Cache all needed state in local variables before any external call.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // AUTHENTICATION & AUTHORIZATION (5 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "AA-001".into(),
            category: CheckCategory::AuthenticationAuthorization,
            title: "require_auth() called with correct address".into(),
            description: "require_auth() must be called on the actual signer address, not on a \
                          contract-derived or computed address that could be forged.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::Automatic {
                patterns: vec!["require_auth()".into()],
            },
            remediation: "Call require_auth() on the exact address from contract arguments, \
                         not a computed/stored one unless verified to be the same.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AA-002".into(),
            category: CheckCategory::AuthenticationAuthorization,
            title: "Nonces used to prevent signature replay attacks".into(),
            description: "Off-chain signature schemes must include a nonce that is incremented \
                          on each use to prevent replay attacks.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["nonce".into(), "replay".into()],
            },
            remediation: "Store per-account nonces. Require correct nonce in signed payload and \
                         increment on success.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AA-003".into(),
            category: CheckCategory::AuthenticationAuthorization,
            title: "Signatures include contract address and chain ID".into(),
            description: "Signed messages that don't include the contract address and chain/network \
                          ID can be replayed on other deployments.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["domain_separator".into(), "chain_id".into(), "contract_address".into()],
            },
            remediation: "Include contract address, network passphrase, and nonce in signed hash.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AA-004".into(),
            category: CheckCategory::AuthenticationAuthorization,
            title: "Signature expiry checked (deadline in signed payload)".into(),
            description: "Signatures without expiry can be hoarded and submitted at an attacker-controlled \
                          time. All signed messages must include a deadline.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["deadline".into(), "expiry".into(), "valid_until".into()],
            },
            remediation: "Include `deadline: u64` in signed payload. Check: \
                         `require!(env.ledger().timestamp() <= deadline)`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "AA-005".into(),
            category: CheckCategory::AuthenticationAuthorization,
            title: "Authorization scope limited (sub-invocation auth tree)".into(),
            description: "Soroban's authorization model requires explicitly specifying the \
                          sub-invocation tree an account authorizes. Overly broad auth contexts \
                          are a security risk.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Use InvokerContractAuthEntry to scope what sub-invocations are \
                         authorized.".into(),
            references: vec!["https://soroban.stellar.org/docs/learn/authorization".into()],
        },

        // ─────────────────────────────────────────
        // ERROR HANDLING (4 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "EH-001".into(),
            category: CheckCategory::ErrorHandling,
            title: "All errors use typed ContractError enum".into(),
            description: "Returning opaque error codes or generic strings makes it impossible \
                          for callers to handle errors programmatically.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["ContractError".into(), "#[contracterror]".into()],
            },
            remediation: "Define `#[contracterror] enum ContractError` and return it everywhere.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "EH-002".into(),
            category: CheckCategory::ErrorHandling,
            title: "No silent error swallowing with let _ =".into(),
            description: "Assigning results to `_` discards errors silently. All Results from \
                          operations that can fail must be propagated.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Automatic {
                patterns: vec!["let _ =".into(), "let _result".into()],
            },
            remediation: "Use `?` operator or explicit match. Never discard Results silently.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "EH-003".into(),
            category: CheckCategory::ErrorHandling,
            title: "Error codes do not leak internal state information".into(),
            description: "Detailed error messages may expose internal balances, limits, or \
                          system state to external callers in unintended ways.".into(),
            severity: Severity::Low,
            detection: DetectionMethod::Manual,
            remediation: "Use generic error codes for public-facing errors. Log details via events.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "EH-004".into(),
            category: CheckCategory::ErrorHandling,
            title: "Storage get() missing-key cases handled explicitly".into(),
            description: "Calling `.get()` on storage returns `Option<T>`. Not handling the None \
                          case leads to panics or incorrect default-value behavior.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["storage().get".into(), ".unwrap_or".into(), ".ok_or".into()],
            },
            remediation: "Handle None: `storage.get(&key).ok_or(ContractError::NotFound)?`.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // TOKEN SAFETY (5 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "TS-001".into(),
            category: CheckCategory::TokenSafety,
            title: "Token transfers validated with return value check".into(),
            description: "SEP-41 token transfer may fail. The contract must handle and propagate \
                          transfer failures rather than assuming success.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["TokenClient".into(), "transfer".into()],
            },
            remediation: "Propagate errors from token.transfer(). Do not assume transfers succeed.".into(),
            references: vec!["https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md".into()],
        },
        ChecklistItem {
            id: "TS-002".into(),
            category: CheckCategory::TokenSafety,
            title: "No price oracle used without staleness check".into(),
            description: "Price oracle data can become stale. Using an outdated price without \
                          checking its timestamp enables price manipulation attacks.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["oracle".into(), "price".into(), "timestamp".into()],
            },
            remediation: "Check `require!(price_timestamp + MAX_STALENESS > env.ledger().timestamp())`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "TS-003".into(),
            category: CheckCategory::TokenSafety,
            title: "Contract does not hold excess native XLM".into(),
            description: "Contracts that accumulate more XLM than needed for storage reserves \
                          become targets for extraction attacks.".into(),
            severity: Severity::Low,
            detection: DetectionMethod::Manual,
            remediation: "Sweep or limit XLM holdings to the minimum required for ledger entries.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "TS-004".into(),
            category: CheckCategory::TokenSafety,
            title: "Fee calculations use exact amounts, not estimates".into(),
            description: "Estimating fees off-chain and hardcoding them leads to discrepancies \
                          that can be exploited to overpay or underpay fees.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Calculate fees on-chain using exact amounts at time of execution.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "TS-005".into(),
            category: CheckCategory::TokenSafety,
            title: "Token allowance set-then-transfer (TOCTOU) prevented".into(),
            description: "Setting an allowance and transferring in separate transactions creates \
                          a window for frontrunning. Use transferFrom after atomic approval.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["approve".into(), "allowance".into(), "transfer_from".into()],
            },
            remediation: "Set allowance to exact amount and transfer in the same transaction.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // CROSS-CONTRACT CALLS (4 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "CC-001".into(),
            category: CheckCategory::CrossContractCalls,
            title: "External contract addresses validated before calls".into(),
            description: "Calling an unvalidated contract address allows attackers to substitute \
                          a malicious contract.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["Client::new".into(), "token_client".into()],
            },
            remediation: "Only call contracts whose addresses are stored in persistent admin-set \
                         storage, never caller-supplied addresses.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "CC-002".into(),
            category: CheckCategory::CrossContractCalls,
            title: "Cross-contract call results always checked".into(),
            description: "Ignoring the return value of a cross-contract call means failures are \
                          silent and can leave the contract in an inconsistent state.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Manual,
            remediation: "Propagate all Results from cross-contract calls with `?`.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "CC-003".into(),
            category: CheckCategory::CrossContractCalls,
            title: "Gas budget not exhausted by recursive/deep call chains".into(),
            description: "Deep cross-contract call stacks consume instruction budget. Circular \
                          call graphs can exhaust the budget entirely, causing tx failure.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Limit call depth. Avoid designs where A calls B calls A.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "CC-004".into(),
            category: CheckCategory::CrossContractCalls,
            title: "Contract dependency addresses are upgradeable".into(),
            description: "Hardcoded addresses of dependency contracts prevent updating to patched \
                          versions if a dependency is found to be vulnerable.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Store all external contract addresses in admin-controlled persistent storage.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // EVENT LOGGING (3 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "EL-001".into(),
            category: CheckCategory::EventLogging,
            title: "All fund-moving operations emit events".into(),
            description: "Deposits, withdrawals, transfers, and swaps must emit events so that \
                          off-chain indexers can track all value flows.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["env.events().publish".into(), "events().publish".into()],
            },
            remediation: "Add `env.events().publish(topics, data)` to every fund-moving function.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "EL-002".into(),
            category: CheckCategory::EventLogging,
            title: "Admin/configuration changes emit events".into(),
            description: "Changes to admin, fees, or configuration must be logged so that \
                          monitoring systems can detect unexpected governance changes.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["set_admin".into(), "set_fee".into(), "env.events()".into()],
            },
            remediation: "Emit events from all governance-changing functions.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "EL-003".into(),
            category: CheckCategory::EventLogging,
            title: "Events include sufficient context (from, to, amount, timestamp)".into(),
            description: "Events that only log a type without contextual data are not useful \
                          for auditing or debugging.".into(),
            severity: Severity::Low,
            detection: DetectionMethod::Manual,
            remediation: "Include sender, recipient, amount, and ledger sequence in event data.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // STORAGE PATTERNS (4 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "SP-001".into(),
            category: CheckCategory::StoragePatterns,
            title: "No key collisions between different data types".into(),
            description: "Reusing the same storage key for different data types causes corruption \
                          when one overwrites the other.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["DataKey::".into(), "#[contracttype]".into()],
            },
            remediation: "Each logical storage slot must have a unique key in the DataKey enum.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SP-002".into(),
            category: CheckCategory::StoragePatterns,
            title: "User-specific storage keyed by address, not index".into(),
            description: "Keying user storage by sequential index instead of address allows \
                          index-guessing attacks and fragile iteration.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["DataKey::Balance".into(), "DataKey::UserData".into()],
            },
            remediation: "Key user data by (DataKey::Balance, user_address) composite keys.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SP-003".into(),
            category: CheckCategory::StoragePatterns,
            title: "Instance storage only used for contract-global data".into(),
            description: "Instance storage is lost when the contract is undeployed. It should \
                          only store truly global contract state, not per-user data.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["storage().instance()".into()],
            },
            remediation: "Move per-user and long-lived data to persistent storage.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SP-004".into(),
            category: CheckCategory::StoragePatterns,
            title: "Storage reads cached to avoid redundant ledger I/O".into(),
            description: "Reading the same storage key multiple times per invocation wastes \
                          CPU budget. Cache values in local variables.".into(),
            severity: Severity::Low,
            detection: DetectionMethod::Manual,
            remediation: "Read each storage key once per function and use the cached value.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "SP-005".into(),
            category: CheckCategory::StoragePatterns,
            title: "Raw Storage Key Usage",
            description: "Detects the use of raw strings or symbol macros for storage keys instead \
                          of an exhaustive DataKey enum. Raw keys are prone to typo collisions.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Automatic { 
                patterns: vec!["symbol_short!(".into(), "Symbol::new(".into(), "&\"".into()] 
            },
            remediation: "Migrate all hardcoded string keys to a centralized `#[contracttype] pub enum DataKey`.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // UPGRADEABILITY (3 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "UP-001".into(),
            category: CheckCategory::Upgradeability,
            title: "Upgrade function protected and emits event".into(),
            description: "Contract upgrades must be admin-gated and emit an event so that \
                          users can observe when logic changes.".into(),
            severity: Severity::Critical,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["upgrade".into(), "update_current_contract_wasm".into()],
            },
            remediation: "Require admin auth and emit UpgradeEvent in the upgrade function.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "UP-002".into(),
            category: CheckCategory::Upgradeability,
            title: "Timelock on admin operations for user protection".into(),
            description: "Sensitive operations like upgrades or fee changes should have a \
                          timelock so users can exit before changes take effect.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["timelock".into(), "delay".into(), "proposed_at".into()],
            },
            remediation: "Implement a propose/execute pattern with a minimum delay for critical ops.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "UP-003".into(),
            category: CheckCategory::Upgradeability,
            title: "Immutable functions documented and tested".into(),
            description: "Functions that should never change (e.g., total supply cap) must be \
                          documented as immutable and covered by invariant tests.".into(),
            severity: Severity::Low,
            detection: DetectionMethod::Manual,
            remediation: "Document immutability in code comments and enforce with contract-level tests.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // DATA SERIALIZATION (3 items)
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "DS-001".into(),
            category: CheckCategory::DataSerialization,
            title: "All custom types derive #[contracttype]".into(),
            description: "Custom types used in contract storage or return values must derive \
                          #[contracttype] for correct XDR serialization.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Automatic {
                patterns: vec!["#[contracttype]".into()],
            },
            remediation: "Add `#[contracttype]` to all types used in contract storage or interface.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "DS-002".into(),
            category: CheckCategory::DataSerialization,
            title: "Schema version field in stored structs".into(),
            description: "Stored structs without a version field cannot be safely migrated when \
                          fields are added or removed in upgrades.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Add `version: u32` to all persistently stored structs.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "DS-003".into(),
            category: CheckCategory::DataSerialization,
            title: "Deserialization of external data fully validated".into(),
            description: "Data arriving from cross-contract calls or external sources must be \
                          fully validated after deserialization.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Manual,
            remediation: "After deserialization, validate all fields are in expected ranges/values.".into(),
            references: vec![],
        },

        // ─────────────────────────────────────────
        // RESOURCE LIMITS (4 items)
        // Quick operator guidance:
        // - Measure CPU with Soroban CLI cost output on worst-case inputs (not only happy paths).
        // - Treat memory pressure as two checks: entry count growth + per-entry byte footprint.
        // - Measure WASM size from the release artifact and keep upgrade headroom (don't target 100%).
        // - Common failures: "instruction budget exceeded", oversized ledger entry writes,
        //   or upgrade/deploy rejection due to WASM byte limit.
        // ─────────────────────────────────────────
        ChecklistItem {
            id: "RL-001".into(),
            category: CheckCategory::ResourceLimits,
            title: "No unbounded loops over user-controlled data".into(),
            description: "Looping over a Vec or Map whose size is controlled by users can exhaust \
                          the instruction budget and DoS the contract.".into(),
            severity: Severity::High,
            detection: DetectionMethod::SemiAutomatic {
                patterns: vec!["for ".into(), "iter()".into(), ".len()".into()],
            },
            remediation: "Cap loop iterations. Use pagination for large datasets.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "RL-002".into(),
            category: CheckCategory::ResourceLimits,
            title: "Instruction budget not exceeded in worst-case path".into(),
            description: "The worst-case execution path must stay within Soroban's instruction \
                          budget. Test with maximum-size inputs and realistic batch sizes, not \
                          just nominal examples.".into(),
            severity: Severity::High,
            detection: DetectionMethod::Manual,
            remediation: "Measure current usage with Soroban CLI cost output (e.g. `--cost` or \
                         `--cost-snapshot`) for each high-risk entry point. Record both average \
                         and worst-case instruction counts. Practical baselines: simple registry \
                         lookups are often ~50k-400k instructions, token/escrow flows ~100k-1.5M, \
                         and AMM/orderbook or batch settlement paths ~500k-10M+. If you hit \
                         \"instruction budget exceeded\" errors, cap loop iterations, paginate \
                         collections, cache repeated reads, and split work across multiple txs.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "RL-003".into(),
            category: CheckCategory::ResourceLimits,
            title: "Ledger entry byte limits / memory footprint respected".into(),
            description: "Each ledger entry has a maximum byte size, and oversized serialized \
                          structs/maps behave like memory pressure during writes. Storing \
                          unbounded data in a single entry will fail at write time.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Measure current usage by serializing representative state objects and \
                         checking byte size (especially the largest map/vector entries), then test \
                         writes with worst-case payloads. As a rule of thumb, keep common records \
                         in the low hundreds of bytes to a few KB, and shard before approaching \
                         network entry limits. If writes fail with serialization/ledger-size \
                         errors, split data across multiple keys, add pagination, or store \
                         payloads off-chain with on-chain hashes.".into(),
            references: vec![],
        },
        ChecklistItem {
            id: "RL-004".into(),
            category: CheckCategory::ResourceLimits,
            title: "WASM binary size measured with upgrade headroom".into(),
            description: "Contract WASM size must remain below the network byte limit for deploy \
                          and future upgrades. Teams should track artifact size over time and \
                          preserve headroom for new features.".into(),
            severity: Severity::Medium,
            detection: DetectionMethod::Manual,
            remediation: "Measure current usage from the compiled artifact (`wc -c` on the \
                         release `.wasm`) and compare against the configured network limit. \
                         Practical baselines after optimization: small registry contracts are \
                         often ~80KB-200KB, token/escrow ~150KB-350KB, and feature-rich \
                         marketplace/AMM contracts ~300KB-800KB. If deploy/upgrade fails due to \
                         size, build with `opt-level = \"z\"`, run `wasm-opt -Oz`, remove dead \
                         code, and move large tables/constants out of the contract.".into(),
            references: vec![],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checklist_has_at_least_50_items() {
        assert!(all_checks().len() >= 50, "Checklist must have 50+ items");
    }

    #[test]
    fn all_ids_unique() {
        let checks = all_checks();
        let mut ids = std::collections::HashSet::new();
        for c in &checks {
            assert!(ids.insert(c.id.clone()), "Duplicate check ID: {}", c.id);
        }
    }

    #[test]
    fn all_categories_represented() {
        let checks = all_checks();
        let categories: std::collections::HashSet<_> =
            checks.iter().map(|c| c.category.clone()).collect();
        assert!(
            categories.len() >= 10,
            "Should cover at least 10 categories"
        );
    }
}
