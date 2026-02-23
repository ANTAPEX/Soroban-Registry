#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

@contract
pub struct VulnerableContract;

@contractimpl
impl VulnerableContract {
    pub fn set_admin(env: Env, admin: Symbol) {
        // ❌ VULNERABLE: Using a raw string/macro for the storage key
        // The detector will flag this line.
        env.storage().instance().set(&symbol_short!("admin"), &admin);
    }

    pub fn get_admin(env: Env) -> Symbol {
        // ❌ VULNERABLE: Prone to typo errors 
        env.storage().instance().get(&symbol_short!("admin")).unwrap()
    }
}