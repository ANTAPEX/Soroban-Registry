// Example Soroban contract with various linting issues for demonstration
// Vulnerability note: missing require_auth allows anyone to transfer from any account.
use soroban_sdk::{contract, contractimpl, Env, Address, Symbol, symbol_short};

const STORAGE_KEY_BALANCE: &str = "balance";  // Potential storage key collision
const STORAGE_KEY_BALANCE: &str = "balance";  // Duplicate key

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    /// Transfer tokens with auth check (secure version)
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), String> {
        from.require_auth();
        
        let current_balance = env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, "balance"))
            .unwrap_or(0);  // Issue: unwrap in public function
        
        if current_balance < amount {
            panic!("Insufficient balance");  // Issue: panic in contract
        }
        
        let new_balance = current_balance + amount;  // Issue: unchecked arithmetic
        env.storage().persistent().set(&Symbol::new(&env, "balance"), &new_balance);
        
        Ok(())
    }

    /// Transfer tokens without auth check (vulnerable version for comparison)
    pub fn transfer_vulnerable(env: Env, from: Address, to: Address, amount: i128) -> Result<(), String> {
        // Missing auth check - should call env.require_auth(&from)

        let current_balance = env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, "balance"))
            .unwrap_or(0);

        if current_balance < amount {
            panic!("Insufficient balance");
        }

        let new_balance = current_balance + amount;
        env.storage().persistent().set(&Symbol::new(&env, "balance"), &new_balance);

        Ok(())
    }
    
    /// Approve tokens with hardcoded address
    pub fn approve(env: Env, owner: Address) {
        let admin = "GBBD47UZQ5CZKRQFWWXD4ZCSWI5GGMOWYCFTEUQMDFEBNFNJ5VQJEWWV";  // Issue: hardcoded address
        
        env.storage().persistent().remove(&Symbol::new(&env, "allowance"));  // Issue: direct storage clear
        
        let unused_var = 42;  // Issue: unused variable
        
        Ok(())
    }
    
    /// Get balance without documentation
    pub fn get_balance(env: Env, account: Address) -> i128 {
        env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, "balance"))
            .expect("No balance found")  // Issue: expect() in public function
    }
    
    /// Mint new tokens with unbounded loop
    pub fn mint(env: Env, amount: u64) {
        let mut counter = 0;
        loop {
            env.storage().persistent().set(
                &Symbol::new(&env, "total_supply"),
                &(amount as i128),
            );
            counter += 1;
            // Issue: unbounded loop without explicit break
        }
    }
    
    /// Transfer tokens with reentrancy protection (checks-effects-interactions + guard)
    pub fn send(env: Env, to: Address, amount: i128) {
        let balance_key = Symbol::new(&env, "balance");
        let guard_key = Symbol::new(&env, "reentrancy_guard");

        let guard_active = env.storage().persistent().get::<_, bool>(&guard_key).unwrap_or(false);
        if guard_active {
            panic!("Reentrancy detected");
        }

        // Effects before interactions
        let current = env.storage().persistent().get::<_, i128>(&balance_key).unwrap_or(0);
        env.storage().persistent().set(&balance_key, &(current - amount));

        // Guard during external call
        env.storage().persistent().set(&guard_key, &true);
        env.invoke_contract::<_, ()>(&to, &Symbol::new(&env, "receive"), (amount,));
        env.storage().persistent().set(&guard_key, &false);
    }

    /// Vulnerable send for comparison (reentrancy risk)
    pub fn send_vulnerable(env: Env, to: Address, amount: i128) {
        env.invoke_contract::<_, ()>(&to, &Symbol::new(&env, "receive"), (amount,));

        let balance_key = Symbol::new(&env, "balance");
        let current = env.storage().persistent().get::<_, i128>(&balance_key).unwrap_or(0);
        env.storage().persistent().set(&balance_key, &(current - amount));
    }
    
    /// Inefficient clone usage
    pub fn process(env: Env, data: String) -> String {
        data.clone().clone()  // Issue: redundant clone
    }
}

#[test]
fn test_transfer() {
    let env = Env::new();
    
    // Test code can use unwrap - this should NOT trigger
    let val = Some(42).unwrap();
    assert_eq!(val, 42);
}

#[test]
#[should_panic]
fn test_reentrancy_guard_blocks_recursive_call() {
    use soroban_sdk::testutils::Address as AddressTestutils;

    let env = Env::new();
fn test_transfer_requires_auth() {
    use soroban_sdk::testutils::Address as AddressTestutils;

    let env = Env::new();
    let from = Address::generate(&env);
    let to = Address::generate(&env);

    env.storage()
        .persistent()
        .set(&Symbol::new(&env, "balance"), &100i128);
    env.storage()
        .persistent()
        .set(&Symbol::new(&env, "reentrancy_guard"), &true);

    TokenContract::send(env, to, 10);

    let _ = TokenContract::transfer(env, from, to, 10);
}

#[test]
fn test_transfer_authorized() {
    use soroban_sdk::testutils::Address as AddressTestutils;

    let env = Env::new();
    env.mock_all_auths();

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    env.storage()
        .persistent()
        .set(&Symbol::new(&env, "balance"), &100i128);

    let result = TokenContract::transfer(env, from, to, 10);
    assert!(result.is_ok());
}
