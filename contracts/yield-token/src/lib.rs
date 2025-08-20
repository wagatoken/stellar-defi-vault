#![no_std]
use shared::{UserYieldInfo, VaultType, REBASE_INTERVAL};
use soroban_sdk::token::TokenInterface;
use soroban_sdk::{
    contract, contractimpl, log, symbol_short, Address, Env, String, Symbol,
};
use soroban_token_sdk::metadata::TokenMetadata;

// Storage Keys
const BALANCE: Symbol = symbol_short!("BALANCE");
const USER_YIELD: Symbol = symbol_short!("YIELD");
const TOTAL_SUPPLY: Symbol = symbol_short!("SUPPLY");
const GLOBAL_YIELD_RATE: Symbol = symbol_short!("RATE");
const LAST_REBASE: Symbol = symbol_short!("REBASE");
const METADATA: Symbol = symbol_short!("METADATA");

#[contract]
pub struct YieldToken;

#[contractimpl]
impl YieldToken {
    /// Initialize the yield token contract
    pub fn initialize(env: Env, admin: Address, metadata: TokenMetadata) {
        admin.require_auth();

        // Set initial metadata
        env.storage().instance().set(&METADATA, &metadata);

        // Initialize global state
        env.storage().instance().set(&TOTAL_SUPPLY, &0u128);
        env.storage().instance().set(&GLOBAL_YIELD_RATE, &500u128); // 5% base rate
        env.storage()
            .instance()
            .set(&LAST_REBASE, &env.ledger().timestamp());

        log!(&env, "YieldToken initialized with admin: {}", admin);
    }

    /// Get user's current interest rate based on their holdings and lock periods
    pub fn get_user_interest_rate(env: Env, user: Address) -> u128 {
        let yield_info = Self::get_user_yield_info(&env, &user);
        yield_info.yield_rate
    }

    /// Update global yield rate (only callable by authorized contracts)
    pub fn update_global_yield_rate(env: Env, caller: Address, new_rate: u128) {
        caller.require_auth();
        // TODO: Add authorization check for vault contracts

        env.storage().instance().set(&GLOBAL_YIELD_RATE, &new_rate);
        log!(&env, "Global yield rate updated to: {}", new_rate);
    }

    /// Compound interest for a specific user
    pub fn compound_interest(env: Env, user: Address) -> u128 {
        let mut yield_info = Self::get_user_yield_info(&env, &user);
        let current_time = env.ledger().timestamp();

        if current_time > yield_info.last_compound_time {
            let time_elapsed = current_time - yield_info.last_compound_time;
            let new_yield = Self::calculate_compound_yield(
                &env,
                yield_info.principal,
                yield_info.yield_rate,
                time_elapsed,
            );

            yield_info.total_yield_earned += new_yield - yield_info.principal;
            yield_info.principal = new_yield;
            yield_info.last_compound_time = current_time;

            Self::set_user_yield_info(&env, &user, &yield_info);
            Self::set_balance(&env, &user, new_yield);

            log!(
                &env,
                "Compounded interest for user: {}, new balance: {}",
                user,
                new_yield
            );
        }

        yield_info.principal
    }

    /// Mint tokens for vault deposits
    pub fn mint_for_deposit(
        env: Env,
        vault_contract: Address,
        user: Address,
        amount: u128,
        _vault_type: VaultType,
        yield_rate: u128,
    ) {
        vault_contract.require_auth();
        // TODO: Add vault contract authorization check

        let current_time = env.ledger().timestamp();
        let current_balance = Self::balance(env.clone(), user.clone()) as u128;
        let new_balance = current_balance + amount;

        // Update user's yield info
        let yield_info = UserYieldInfo {
            principal: new_balance,
            yield_rate,
            last_compound_time: current_time,
            total_yield_earned: 0,
        };

        Self::set_user_yield_info(&env, &user, &yield_info);
        Self::set_balance(&env, &user, new_balance);

        // Update total supply
        let total_supply = Self::total_supply(env.clone()) as u128;
        env.storage()
            .instance()
            .set(&TOTAL_SUPPLY, &(total_supply + amount));

        log!(
            &env,
            "Minted {} tokens for user: {} from vault: {}",
            amount,
            user,
            vault_contract
        );
    }

    /// Burn tokens for vault withdrawals
    pub fn burn_for_withdrawal(env: Env, vault_contract: Address, user: Address, amount: u128) {
        vault_contract.require_auth();

        let current_balance = Self::balance(env.clone(), user.clone()) as u128;
        if current_balance < amount {
            panic!("Insufficient balance for burn");
        }

        let new_balance = current_balance - amount;
        Self::set_balance(&env, &user, new_balance);

        // Update total supply
        let total_supply = Self::total_supply(env.clone()) as u128;
        env.storage()
            .instance()
            .set(&TOTAL_SUPPLY, &(total_supply - amount));

        log!(
            &env,
            "Burned {} tokens for user: {} from vault: {}",
            amount,
            user,
            vault_contract
        );
    }

    /// Perform global rebase if interval has passed
    pub fn rebase(env: Env) {
        let current_time = env.ledger().timestamp();
        let last_rebase = env.storage().instance().get(&LAST_REBASE).unwrap_or(0u64);

        if current_time >= last_rebase + REBASE_INTERVAL {
            // This would update all user balances based on global yield
            // For now, we'll mark the rebase time and let individual compound_interest calls handle the math
            env.storage().instance().set(&LAST_REBASE, &current_time);

            log!(
                &env,
                "Global rebase executed at timestamp: {}",
                current_time
            );
        }
    }

    /// Calculate compound yield using simplified formula
    fn calculate_compound_yield(
        _env: &Env,
        principal: u128,
        annual_rate: u128,
        time_elapsed: u64,
    ) -> u128 {
        // Daily compounding: A = P(1 + r/365)^(t/86400)
        // Simplified to avoid complex exponentiation in smart contract
        let days_elapsed = time_elapsed / 86400; // Convert seconds to days
        let daily_rate = annual_rate / 365; // Basis points per day

        let mut result = principal;
        for _ in 0..days_elapsed {
            // Apply daily compound interest
            result = result + (result * daily_rate) / 10000;
        }

        result
    }

    /// Internal helper functions
    fn get_user_yield_info(env: &Env, user: &Address) -> UserYieldInfo {
        env.storage()
            .persistent()
            .get(&(USER_YIELD.clone(), user.clone()))
            .unwrap_or(UserYieldInfo {
                principal: 0,
                yield_rate: 500, // Default 5%
                last_compound_time: env.ledger().timestamp(),
                total_yield_earned: 0,
            })
    }

    fn set_user_yield_info(env: &Env, user: &Address, info: &UserYieldInfo) {
        env.storage()
            .persistent()
            .set(&(USER_YIELD.clone(), user.clone()), info);
    }

    fn set_balance(env: &Env, user: &Address, amount: u128) {
        env.storage()
            .persistent()
            .set(&(BALANCE.clone(), user.clone()), &amount);
    }
}

// Implement standard token interface
#[contractimpl]
impl TokenInterface for YieldToken {
    fn allowance(_env: Env, _from: Address, _spender: Address) -> i128 {
        // Not implementing allowance for this rebasing token
        0
    }

    fn approve(_env: Env, _from: Address, _spender: Address, _amount: i128, _expiration_ledger: u32) {
        // Not implementing approve for this rebasing token
        panic!("Approve not supported for rebasing yield token");
    }

    fn balance(env: Env, id: Address) -> i128 {
        let balance: u128 = env
            .storage()
            .persistent()
            .get(&(BALANCE.clone(), id))
            .unwrap_or(0);
        balance as i128
    }

    fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // Basic transfer implementation - could be restricted based on lock periods
        from.require_auth();

        if amount < 0 {
            panic!("Transfer amount cannot be negative");
        }

        let amount = amount as u128;
        let from_balance = Self::balance(env.clone(), from.clone()) as u128;

        if from_balance < amount {
            panic!("Insufficient balance");
        }

        let to_balance = Self::balance(env.clone(), to.clone()) as u128;

        Self::set_balance(&env, &from, from_balance - amount);
        Self::set_balance(&env, &to, to_balance + amount);

        log!(&env, "Transferred {} from {} to {}", amount, from, to);
    }

    fn transfer_from(_env: Env, _spender: Address, _from: Address, _to: Address, _amount: i128) {
        // Not implementing transfer_from for this rebasing token
        panic!("TransferFrom not supported for rebasing yield token");
    }

    fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();

        if amount < 0 {
            panic!("Burn amount cannot be negative");
        }

        let amount = amount as u128;
        let balance = Self::balance(env.clone(), from.clone()) as u128;

        if balance < amount {
            panic!("Insufficient balance to burn");
        }

        Self::set_balance(&env, &from, balance - amount);

        let total_supply = env.storage().instance().get(&TOTAL_SUPPLY).unwrap_or(0u128);
        env.storage()
            .instance()
            .set(&TOTAL_SUPPLY, &(total_supply - amount));
    }

    fn burn_from(_env: Env, _spender: Address, _from: Address, _amount: i128) {
        // Not implementing burn_from for this rebasing token
        panic!("BurnFrom not supported for rebasing yield token");
    }

    fn decimals(_env: Env) -> u32 {
        6 // USDC-compatible decimals
    }

    fn name(env: Env) -> String {
        let metadata: TokenMetadata = env.storage().instance().get(&METADATA).unwrap();
        metadata.name
    }

    fn symbol(env: Env) -> String {
        let metadata: TokenMetadata = env.storage().instance().get(&METADATA).unwrap();
        metadata.symbol
    }
}

// Additional helper functions for this contract
#[contractimpl]
impl YieldToken {
    /// Get total supply (not part of TokenInterface)
    pub fn total_supply(env: Env) -> i128 {
        let supply: u128 = env.storage().instance().get(&TOTAL_SUPPLY).unwrap_or(0);
        supply as i128
    }
}
