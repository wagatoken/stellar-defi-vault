#![no_std]
use soroban_sdk::{
    contract, contractimpl, log, symbol_short, Address, Env, Symbol, IntoVal
};
use soroban_sdk::token::TokenClient;
use shared::{DepositInfo, LockPeriod, VaultType};

// Storage Keys
const DEPOSIT: Symbol = symbol_short!("DEPOSIT");
const VAULT_BALANCE: Symbol = symbol_short!("BALANCE");
const YIELD_TOKEN: Symbol = symbol_short!("YIELD");
const ADMIN: Symbol = symbol_short!("ADMIN");
const USDC_CONTRACT: Symbol = symbol_short!("USDC");

#[contract]
pub struct USDCVault;

#[contractimpl]
impl USDCVault {
    /// Initialize the USDC vault
    pub fn initialize(
        env: Env,
        admin: Address,
        usdc_contract: Address,
        yield_token_contract: Address,
    ) {
        admin.require_auth();
        
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&USDC_CONTRACT, &usdc_contract);
        env.storage().instance().set(&YIELD_TOKEN, &yield_token_contract);
        env.storage().instance().set(&VAULT_BALANCE, &0u128);
        
        log!(&env, "USDC Vault initialized with admin: {}", admin);
    }

    /// Deposit USDC into the vault with time lock
    pub fn deposit(env: Env, user: Address, amount: u128, lock_period: LockPeriod) {
        user.require_auth();
        
        if amount == 0 {
            panic!("Deposit amount must be greater than 0");
        }

        let current_time = env.ledger().timestamp();
        let unlock_time = Self::calculate_unlock_time(current_time, &lock_period);
        
        // Check if user already has a deposit (for now, one deposit per user)
        if env.storage().persistent().has(&(DEPOSIT.clone(), user.clone())) {
            panic!("User already has an active deposit. Withdraw first to make a new deposit.");
        }

        // Transfer USDC from user to vault
        let usdc_contract: Address = env.storage().instance().get(&USDC_CONTRACT).unwrap();
        let usdc_client = TokenClient::new(&env, &usdc_contract);
        
        usdc_client.transfer(&user, &env.current_contract_address(), &(amount as i128));

        // Update vault balance
        let vault_balance: u128 = env.storage().instance().get(&VAULT_BALANCE).unwrap_or(0);
        env.storage().instance().set(&VAULT_BALANCE, &(vault_balance + amount));

        // Create deposit info
        let deposit_info = DepositInfo {
            amount,
            deposit_time: current_time,
            unlock_time,
            lock_period: lock_period.clone(),
            vault_type: VaultType::USDC,
        };

        // Store deposit info
        env.storage()
            .persistent()
            .set(&(DEPOSIT.clone(), user.clone()), &deposit_info);

        // Calculate yield rate and mint yield tokens
        let yield_rate = Self::calculate_yield_rate(env.clone(), lock_period.clone());
        let yield_token_contract: Address = env.storage().instance().get(&YIELD_TOKEN).unwrap();
        
        // Call yield token contract to mint tokens
        env.invoke_contract::<()>(
            &yield_token_contract,
            &Symbol::new(&env, "mint_for_deposit"),
            (
                env.current_contract_address(),
                user.clone(),
                amount,
                VaultType::USDC,
                yield_rate,
            ).into_val(&env),
        );
                log!(
            &env,
            "User {} deposited {} USDC with {:?} lock period. Unlock time: {}",
            user,
            amount,
            lock_period,
            unlock_time
        );
    }

    /// Withdraw USDC from the vault (only after lock period expires)
    pub fn withdraw(env: Env, user: Address) -> u128 {
        user.require_auth();

        let deposit_info: DepositInfo = env
            .storage()
            .persistent()
            .get(&(DEPOSIT.clone(), user.clone()))
            .unwrap_or_else(|| panic!("No deposit found for user"));

        let current_time = env.ledger().timestamp();
        
        if current_time < deposit_info.unlock_time {
            panic!(
                "Withdrawal not allowed. Lock period expires at: {}",
                deposit_info.unlock_time
            );
        }

        // Calculate final amount including yield
        let yield_token_contract: Address = env.storage().instance().get(&YIELD_TOKEN).unwrap();
        
        // Compound interest first
        env.invoke_contract::<()>(
            &yield_token_contract,
            &Symbol::new(&env, "compound_interest"),
            (user.clone(),).into_val(&env),
        );

        // Get final balance from yield token
        let final_amount: i128 = env.invoke_contract(
            &yield_token_contract,
            &Symbol::new(&env, "balance"),
            (user.clone(),).into_val(&env),
        );

        let withdrawal_amount = final_amount as u128;

        // Burn yield tokens
        env.invoke_contract::<()>(
            &yield_token_contract,
            &Symbol::new(&env, "burn_for_withdrawal"),
            (
                env.current_contract_address(),
                user.clone(),
                withdrawal_amount,
            ).into_val(&env),
        );

        // Transfer USDC back to user
        let usdc_contract: Address = env.storage().instance().get(&USDC_CONTRACT).unwrap();
        let usdc_client = TokenClient::new(&env, &usdc_contract);
        
        usdc_client.transfer(
            &env.current_contract_address(),
            &user,
            &(withdrawal_amount as i128),
        );

        // Update vault balance
        let vault_balance: u128 = env.storage().instance().get(&VAULT_BALANCE).unwrap();
        env.storage().instance().set(&VAULT_BALANCE, &(vault_balance - withdrawal_amount));

        // Remove deposit info
        env.storage()
            .persistent()
            .remove(&(DEPOSIT.clone(), user.clone()));

        log!(
            &env,
            "User {} withdrew {} USDC (including yield)",
            user,
            withdrawal_amount
        );

        withdrawal_amount
    }

    /// Get user's deposit information
    pub fn get_deposit_info(env: Env, user: Address) -> Option<DepositInfo> {
        env.storage()
            .persistent()
            .get(&(DEPOSIT.clone(), user.clone()))
    }

    /// Get lock expiry time for a user
    pub fn get_lock_expiry(env: Env, user: Address) -> u64 {
        let deposit_info: DepositInfo = env
            .storage()
            .persistent()
            .get(&(DEPOSIT.clone(), user.clone()))
            .unwrap_or_else(|| panic!("No deposit found for user"));
        
        deposit_info.unlock_time
    }

    /// Calculate yield rate based on lock period
    pub fn calculate_yield_rate(_env: Env, lock_period: LockPeriod) -> u128 {
        let base_rate = 500u128; // 5% base annual rate in basis points
        
        match lock_period {
            LockPeriod::ThreeMonths => base_rate,                    // 5% APY
            LockPeriod::SixMonths => (base_rate * 15) / 10,         // 7.5% APY (1.5x)
            LockPeriod::TwelveMonths => base_rate * 2,              // 10% APY (2x)
        }
    }

    /// Get current vault USDC balance
    pub fn get_vault_balance(env: Env) -> u128 {
        env.storage().instance().get(&VAULT_BALANCE).unwrap_or(0)
    }

    /// Emergency withdraw with penalty (admin only, for emergencies)
    pub fn emergency_withdraw(env: Env, admin: Address, user: Address) -> u128 {
        admin.require_auth();
        
        let stored_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if admin != stored_admin {
            panic!("Only admin can perform emergency withdrawal");
        }

        let deposit_info: DepositInfo = env
            .storage()
            .persistent()
            .get(&(DEPOSIT.clone(), user.clone()))
            .unwrap_or_else(|| panic!("No deposit found for user"));

        // Apply 10% penalty for early withdrawal
        let penalty_rate = 1000u128; // 10% in basis points
        let penalty = (deposit_info.amount * penalty_rate) / 10000;
        let withdrawal_amount = deposit_info.amount - penalty;

        // Transfer USDC back to user (minus penalty)
        let usdc_contract: Address = env.storage().instance().get(&USDC_CONTRACT).unwrap();
        let usdc_client = TokenClient::new(&env, &usdc_contract);
        
        usdc_client.transfer(
            &env.current_contract_address(),
            &user,
            &(withdrawal_amount as i128),
        );

        // Update vault balance
        let vault_balance: u128 = env.storage().instance().get(&VAULT_BALANCE).unwrap();
        env.storage().instance().set(&VAULT_BALANCE, &(vault_balance - deposit_info.amount));

        // Remove deposit info
        env.storage()
            .persistent()
            .remove(&(DEPOSIT.clone(), user.clone()));

        log!(
            &env,
            "Emergency withdrawal: User {} withdrew {} USDC with {} penalty",
            user,
            withdrawal_amount,
            penalty
        );

        withdrawal_amount
    }

    /// Internal helper functions
    fn calculate_unlock_time(current_time: u64, lock_period: &LockPeriod) -> u64 {
        match lock_period {
            LockPeriod::ThreeMonths => current_time + (90 * 24 * 60 * 60),   // 90 days
            LockPeriod::SixMonths => current_time + (180 * 24 * 60 * 60),    // 180 days  
            LockPeriod::TwelveMonths => current_time + (365 * 24 * 60 * 60), // 365 days
        }
    }
}
