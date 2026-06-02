//! Fixed companion example for `token_with_issues.rs`.
//! Uses an exhaustive typed key enum to avoid silent key collisions.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

/// Every persistent storage key for this contract lives in one typed enum.
///
/// Why this is superior to hardcoded strings:
/// 1. Different variants serialize differently, so unrelated values cannot collide.
/// 2. The compiler forces key definitions to stay centralized and explicit.
/// 3. Address fields scope values per account pair instead of one global bucket.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
}

#[contract]
pub struct TokenFixed;

#[contractimpl]
impl TokenFixed {
    pub fn set_balance(env: Env, owner: Address, amount: i128) {
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Balance(owner), &amount);
    }

    pub fn set_allowance(env: Env, owner: Address, spender: Address, amount: i128) {
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Allowance(owner, spender), &amount);
    }

    pub fn get_balance(env: Env, owner: Address) -> i128 {
        env.storage()
            .persistent()
            .get::<_, i128>(&DataKey::Balance(owner))
            .unwrap_or(0)
    }

    pub fn get_allowance(env: Env, owner: Address, spender: Address) -> i128 {
        env.storage()
            .persistent()
            .get::<_, i128>(&DataKey::Allowance(owner, spender))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::TokenFixed;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn typed_keys_prevent_balance_allowance_collision() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let spender = Address::generate(&env);

        TokenFixed::set_balance(env.clone(), owner.clone(), 100);
        TokenFixed::set_allowance(env.clone(), owner.clone(), spender.clone(), 7);

        assert_eq!(TokenFixed::get_balance(env.clone(), owner.clone()), 100);
        assert_eq!(TokenFixed::get_allowance(env, owner, spender), 7);
    }
}
