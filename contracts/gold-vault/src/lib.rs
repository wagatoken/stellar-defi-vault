#![no_std]
use shared::{
    DepositInfo, LockPeriod, VaultType, PAXG_ASSET, STORAGE_INSTANCE_PERSISTENT, WISDOMTREE_GOLD,
};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{contract, contractimpl, log, symbol_short, Address, Env, IntoVal, Symbol, Vec};

// Storage Keys
const DEPOSIT: Symbol = symbol_short!("DEPOSIT");
const VAULT_BALANCE: Symbol = symbol_short!("BALANCE");
const YIELD_TOKEN: Symbol = symbol_short!("YIELD");
const ADMIN: Symbol = symbol_short!("ADMIN");
const ORACLE: Symbol = symbol_short!("ORACLE");
const SUPPORTED_ASSETS: Symbol = symbol_short!("ASSETS");

#[contract]
pub struct GoldVault;

#[contractimpl]
impl GoldVault {
    /// Initialize the Gold vault
    pub fn initialize(
        env: Env,
        admin: Address,
        yield_token_contract: Address,
        oracle_contract: Address,
        supported_gold_assets: Vec<Address>,
    ) {
        admin.require_auth();

        env.storage().instance().set(&ADMIN, &admin);
        env.storage()
            .instance()
            .set(&YIELD_TOKEN, &yield_token_contract);
        env.storage().instance().set(&ORACLE, &oracle_contract);
        env.storage()
            .instance()
            .set(&SUPPORTED_ASSETS, &supported_gold_assets);
        env.storage().instance().set(&VAULT_BALANCE, &0u128);

        log!(&env, "Gold Vault initialized with admin: {}", admin);
    }

    /// Deposit gold tokens (PAXG/Wisdom Tree) into the vault with time lock
    pub fn deposit(
        env: Env,
        user: Address,
        gold_asset: Address,
        amount: u128,
        lock_period: LockPeriod,
    ) {
        user.require_auth();

        if amount == 0 {
            panic!("Deposit amount must be greater than 0");
        }

        // Verify the gold asset is supported
        Self::verify_supported_asset(&env, &gold_asset);

        let current_time = env.ledger().timestamp();
        let unlock_time = Self::calculate_unlock_time(current_time, &lock_period);

        // Check if user already has a deposit
        if env
            .storage()
            .persistent()
            .has(&(DEPOSIT.clone(), user.clone()))
        {
            panic!("User already has an active deposit. Withdraw first to make a new deposit.");
        }

        // Transfer gold tokens from user to vault
        let gold_client = TokenClient::new(&env, &gold_asset);
        gold_client.transfer(&user, &env.current_contract_address(), &(amount as i128));

        // Get USD value of the gold deposit
        let usd_value = Self::get_usd_value(env.clone(), gold_asset.clone(), amount);

        // Update vault balance (in USD terms)
        let vault_balance: u128 = env.storage().instance().get(&VAULT_BALANCE).unwrap_or(0);
        env.storage()
            .instance()
            .set(&VAULT_BALANCE, &(vault_balance + usd_value));

        // Create deposit info
        let vault_type = Self::determine_vault_type(&env, &gold_asset);
        let deposit_info = DepositInfo {
            amount: usd_value, // Store as USD value for yield calculations
            deposit_time: current_time,
            unlock_time,
            lock_period: lock_period.clone(),
            vault_type,
        };

        // Store deposit info with gold asset details
        env.storage()
            .persistent()
            .set(&(DEPOSIT.clone(), user.clone()), &deposit_info);

        // Store original gold amount and asset for withdrawal
        env.storage()
            .persistent()
            .set(&(Symbol::new(&env, "gold_amount"), user.clone()), &amount);
        env.storage().persistent().set(
            &(Symbol::new(&env, "gold_asset"), user.clone()),
            &gold_asset,
        );

        // Calculate yield rate and mint yield tokens (based on USD value)
        let yield_rate = Self::calculate_yield_rate(env.clone(), lock_period.clone());
        let yield_token_contract: Address = env.storage().instance().get(&YIELD_TOKEN).unwrap();

        env.invoke_contract(
            &yield_token_contract,
            &Symbol::new(&env, "mint_for_deposit"),
            (
                env.current_contract_address(),
                user.clone(),
                usd_value,
                vault_type,
                yield_rate,
            )
                .into_val(&env),
        );

        log!(
            &env,
            "User {} deposited {} gold tokens (${} USD value) with {:?} lock period",
            user,
            amount,
            usd_value,
            lock_period
        );
    }

    /// Withdraw gold tokens from the vault (only after lock period expires)
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

        // Get original gold amount and asset
        let original_gold_amount: u128 = env
            .storage()
            .persistent()
            .get(&(Symbol::new(&env, "gold_amount"), user.clone()))
            .unwrap();
        let gold_asset: Address = env
            .storage()
            .persistent()
            .get(&(Symbol::new(&env, "gold_asset"), user.clone()))
            .unwrap();

        // Calculate final USD amount including yield
        let yield_token_contract: Address = env.storage().instance().get(&YIELD_TOKEN).unwrap();

        // Compound interest first
        env.invoke_contract(
            &yield_token_contract,
            &Symbol::new(&env, "compound_interest"),
            (user.clone(),).into_val(&env),
        );

        // Get final USD balance from yield token
        let final_usd_amount: i128 = env.invoke_contract(
            &yield_token_contract,
            &Symbol::new(&env, "balance"),
            (user.clone(),).into_val(&env),
        );

        let withdrawal_usd_value = final_usd_amount as u128;

        // Calculate equivalent gold amount based on current price
        let current_gold_usd_value =
            Self::get_usd_value(env.clone(), gold_asset.clone(), original_gold_amount);
        let gold_amount_to_return = if current_gold_usd_value > 0 {
            (original_gold_amount * withdrawal_usd_value) / current_gold_usd_value
        } else {
            original_gold_amount // Fallback to original amount if price feed fails
        };

        // Burn yield tokens
        env.invoke_contract(
            &yield_token_contract,
            &Symbol::new(&env, "burn_for_withdrawal"),
            (
                env.current_contract_address(),
                user.clone(),
                withdrawal_usd_value,
            )
                .into_val(&env),
        );

        // Transfer gold tokens back to user
        let gold_client = TokenClient::new(&env, &gold_asset);
        gold_client.transfer(
            &env.current_contract_address(),
            &user,
            &(gold_amount_to_return as i128),
        );

        // Update vault balance
        let vault_balance: u128 = env.storage().instance().get(&VAULT_BALANCE).unwrap();
        env.storage()
            .instance()
            .set(&VAULT_BALANCE, &(vault_balance - withdrawal_usd_value));

        // Clean up storage
        env.storage()
            .persistent()
            .remove(&(DEPOSIT.clone(), user.clone()));
        env.storage()
            .persistent()
            .remove(&(Symbol::new(&env, "gold_amount"), user.clone()));
        env.storage()
            .persistent()
            .remove(&(Symbol::new(&env, "gold_asset"), user.clone()));

        log!(
            &env,
            "User {} withdrew {} gold tokens (${} USD value including yield)",
            user,
            gold_amount_to_return,
            withdrawal_usd_value
        );

        gold_amount_to_return
    }

    /// Get USD value of gold amount using oracle
    pub fn get_usd_value(env: Env, gold_asset: Address, gold_amount: u128) -> u128 {
        let _oracle_contract: Address = env.storage().instance().get(&ORACLE).unwrap();

        // Determine price feed symbol based on asset
        let _price_symbol = if Self::is_paxg_asset(&env, &gold_asset) {
            "PAXG/USD"
        } else {
            "XAU/USD" // Generic gold price for Wisdom Tree or other gold tokens
        };

        // Call oracle for current price (this is a placeholder - actual oracle integration needed)
        // For now, using a mock price
        let gold_price_usd: u128 = 2000_000000; // $2000 per ounce with 6 decimals

        // TODO: Replace with actual oracle call:
        // let gold_price_usd: u128 = env.invoke_contract(
        //     &oracle_contract,
        //     &Symbol::new(env, "get_price"),
        //     (price_symbol,).into_val(env),
        // );

        (gold_amount * gold_price_usd) / 1_000_000 // Assuming 6 decimal places
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

    /// Calculate yield rate based on lock period (same as USDC vault)
    pub fn calculate_yield_rate(env: Env, lock_period: LockPeriod) -> u128 {
        let base_rate = 500u128; // 5% base annual rate in basis points

        match lock_period {
            LockPeriod::ThreeMonths => base_rate,           // 5% APY
            LockPeriod::SixMonths => (base_rate * 15) / 10, // 7.5% APY (1.5x)
            LockPeriod::TwelveMonths => base_rate * 2,      // 10% APY (2x)
        }
    }

    /// Get current vault balance in USD terms
    pub fn get_vault_balance(env: Env) -> u128 {
        env.storage().instance().get(&VAULT_BALANCE).unwrap_or(0)
    }

    /// Add supported gold asset (admin only)
    pub fn add_supported_asset(env: Env, admin: Address, new_asset: Address) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if admin != stored_admin {
            panic!("Only admin can add supported assets");
        }

        let mut supported_assets: Vec<Address> = env
            .storage()
            .instance()
            .get(&SUPPORTED_ASSETS)
            .unwrap_or(Vec::new(&env));

        supported_assets.push_back(new_asset.clone());
        env.storage()
            .instance()
            .set(&SUPPORTED_ASSETS, &supported_assets);

        log!(&env, "Added supported gold asset: {}", new_asset);
    }

    /// Internal helper functions
    fn verify_supported_asset(env: &Env, asset: &Address) {
        let supported_assets: Vec<Address> = env
            .storage()
            .instance()
            .get(&SUPPORTED_ASSETS)
            .unwrap_or(Vec::new(env));

        for supported_asset in supported_assets.iter() {
            if supported_asset == *asset {
                return;
            }
        }

        panic!("Unsupported gold asset");
    }

    fn determine_vault_type(env: &Env, gold_asset: &Address) -> VaultType {
        if Self::is_paxg_asset(env, gold_asset) {
            VaultType::PAXG
        } else {
            VaultType::WisdomTreeGold
        }
    }

    fn is_paxg_asset(env: &Env, asset: &Address) -> bool {
        // TODO: Replace with actual PAXG asset address comparison
        // For now, assume first supported asset is PAXG
        let supported_assets: Vec<Address> = env
            .storage()
            .instance()
            .get(&SUPPORTED_ASSETS)
            .unwrap_or(Vec::new(env));

        if let Some(first_asset) = supported_assets.first() {
            asset == first_asset.clone()
        } else {
            false
        }
    }

    fn calculate_unlock_time(current_time: u64, lock_period: &LockPeriod) -> u64 {
        match lock_period {
            LockPeriod::ThreeMonths => current_time + (90 * 24 * 60 * 60), // 90 days
            LockPeriod::SixMonths => current_time + (180 * 24 * 60 * 60),  // 180 days
            LockPeriod::TwelveMonths => current_time + (365 * 24 * 60 * 60), // 365 days
        }
    }
}
