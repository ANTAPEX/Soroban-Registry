#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Env, Symbol};

// ✅ SECURE REMEDIATION: Exhaustive Enum for all storage keys
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Balance(Symbol),
    Nonce(Symbol),
}

@contract
pub struct SecureContract;

@contractimpl
impl SecureContract {
    pub fn set_admin(env: Env, admin: Symbol) {
        // ✅ SECURE: Uses the exhaustive DataKey enum
        // The detector will ignore this safely.
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn get_admin(env: Env) -> Symbol {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }
}