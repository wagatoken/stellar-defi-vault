#![no_std]
use shared::{
    CollateralInfo, CollateralStatus, COLLATERAL_RATIO_BASIS_POINTS,
};
// use soroban_sdk::token::TokenClient;
use soroban_sdk::{
    contract, contractimpl, log, symbol_short, Address, BytesN, Env, String, Symbol, Vec,
};

// Storage Keys
const COLLATERAL: Symbol = symbol_short!("COLLAT");
const LOAN_COLLATERAL: Symbol = symbol_short!("LOAN");
const ADMIN: Symbol = symbol_short!("ADMIN");
const COMMITTEE: Symbol = symbol_short!("COMMIT");
const ASSET_COUNTER: Symbol = symbol_short!("COUNTER");
const VALUATION_ORACLE: Symbol = symbol_short!("ORACLE");

#[contract]
pub struct CoffeeCollateral;

#[contractimpl]
impl CoffeeCollateral {
    /// Initialize the coffee collateral registry
    pub fn initialize(
        env: Env,
        admin: Address,
        committee_contract: Address,
        valuation_oracle: Address,
    ) {
        admin.require_auth();

        env.storage().instance().set(&ADMIN, &admin);
        env.storage()
            .instance()
            .set(&COMMITTEE, &committee_contract);
        env.storage()
            .instance()
            .set(&VALUATION_ORACLE, &valuation_oracle);
        env.storage().instance().set(&ASSET_COUNTER, &0u64);

        log!(
            &env,
            "Coffee Collateral Registry initialized with admin: {}",
            admin
        );
    }

    /// Create and register a new coffee asset as collateral
    pub fn create_coffee_asset(
        env: Env,
        issuer: Address,
        coffee_batch_id: String,
        quality_grade: u32,
        quantity_kg: u128,
        estimated_value_usd: u128,
        farm_location: String,
        harvest_date: String,
    ) -> Address {
        issuer.require_auth();

        // Validate inputs
        if quality_grade == 0 || quality_grade > 100 {
            panic!("Quality grade must be between 1 and 100");
        }

        if quantity_kg == 0 {
            panic!("Quantity must be greater than 0");
        }

        if estimated_value_usd == 0 {
            panic!("Estimated value must be greater than 0");
        }

        // Generate unique asset code
        let asset_counter: u64 = env.storage().instance().get(&ASSET_COUNTER).unwrap_or(0);
        let new_counter = asset_counter + 1;
        env.storage().instance().set(&ASSET_COUNTER, &new_counter);

        // Create simple asset code (COFFEE + counter will be handled differently)
        let _asset_code = String::from_str(&env, "COFFEE_ASSET");

        // Create Stellar asset (this is a placeholder - actual Stellar asset creation would need different approach)
        // For Soroban, we'll create a token contract instance
        let coffee_asset = env.current_contract_address(); // Placeholder - would be actual token address

        // Store collateral metadata
        let collateral_info = CollateralInfo {
            asset_address: coffee_asset.clone(),
            quality_grade,
            quantity_kg,
            estimated_value_usd,
            creation_time: env.ledger().timestamp(),
            status: CollateralStatus::Active,
        };

        env.storage().persistent().set(
            &(COLLATERAL.clone(), coffee_asset.clone()),
            &collateral_info,
        );

        // Store additional metadata
        env.storage().persistent().set(
            &(Symbol::new(&env, "batch_id"), coffee_asset.clone()),
            &coffee_batch_id,
        );
        env.storage().persistent().set(
            &(Symbol::new(&env, "farm_location"), coffee_asset.clone()),
            &farm_location,
        );
        env.storage().persistent().set(
            &(Symbol::new(&env, "harvest_date"), coffee_asset.clone()),
            &harvest_date,
        );
        env.storage().persistent().set(
            &(Symbol::new(&env, "issuer"), coffee_asset.clone()),
            &issuer,
        );

        log!(
            &env,
            "Created coffee asset: {} for batch: {} with value: ${}",
            coffee_asset,
            coffee_batch_id,
            estimated_value_usd
        );

        coffee_asset
    }

    /// Register existing coffee asset as collateral for a loan
    pub fn register_collateral(
        env: Env,
        committee: Address,
        coffee_asset: Address,
        loan_id: BytesN<32>,
        loan_amount: u128,
    ) {
        committee.require_auth();

        // Verify caller is authorized committee
        let stored_committee: Address = env.storage().instance().get(&COMMITTEE).unwrap();
        if committee != stored_committee {
            panic!("Only committee can register collateral for loans");
        }

        // Get collateral info
        let collateral_info: CollateralInfo = env
            .storage()
            .persistent()
            .get(&(COLLATERAL.clone(), coffee_asset.clone()))
            .unwrap_or_else(|| panic!("Coffee asset not found"));

        // Verify collateral is active
        if collateral_info.status != CollateralStatus::Active {
            panic!("Collateral is not active");
        }

        // Check collateralization ratio (150% requirement)
        let required_collateral_value = (loan_amount * COLLATERAL_RATIO_BASIS_POINTS) / 10000;
        if collateral_info.estimated_value_usd < required_collateral_value {
            panic!(
                "Insufficient collateral value. Required: ${}, Available: ${}",
                required_collateral_value, collateral_info.estimated_value_usd
            );
        }

        // Register collateral for loan
        env.storage()
            .persistent()
            .set(&(LOAN_COLLATERAL.clone(), loan_id.clone()), &coffee_asset);

        log!(
            &env,
            "Registered coffee asset {} as collateral for loan {} worth ${}",
            coffee_asset,
            loan_id,
            loan_amount
        );
    }

    /// Verify collateral for a loan
    pub fn verify_collateral(env: Env, loan_id: BytesN<32>) -> bool {
        let coffee_asset: Option<Address> = env
            .storage()
            .persistent()
            .get(&(LOAN_COLLATERAL.clone(), loan_id));

        match coffee_asset {
            Some(asset) => {
                let collateral_info: Option<CollateralInfo> =
                    env.storage().persistent().get(&(COLLATERAL.clone(), asset));

                match collateral_info {
                    Some(info) => info.status == CollateralStatus::Active,
                    None => false,
                }
            }
            None => false,
        }
    }

    /// Liquidate collateral for defaulted loan
    pub fn liquidate_collateral(env: Env, committee: Address, loan_id: BytesN<32>) {
        committee.require_auth();

        // Verify caller is authorized committee
        let stored_committee: Address = env.storage().instance().get(&COMMITTEE).unwrap();
        if committee != stored_committee {
            panic!("Only committee can liquidate collateral");
        }

        let coffee_asset: Address = env
            .storage()
            .persistent()
            .get(&(LOAN_COLLATERAL.clone(), loan_id.clone()))
            .unwrap_or_else(|| panic!("No collateral found for loan"));

        let mut collateral_info: CollateralInfo = env
            .storage()
            .persistent()
            .get(&(COLLATERAL.clone(), coffee_asset.clone()))
            .unwrap_or_else(|| panic!("Collateral info not found"));

        // Update status to liquidated
        collateral_info.status = CollateralStatus::Liquidated;
        env.storage().persistent().set(
            &(COLLATERAL.clone(), coffee_asset.clone()),
            &collateral_info,
        );

        // TODO: Implement actual liquidation logic (transfer to liquidator, auction, etc.)

        log!(
            &env,
            "Liquidated collateral {} for defaulted loan {}",
            coffee_asset,
            loan_id
        );
    }

    /// Update collateral valuation
    pub fn update_valuation(env: Env, oracle: Address, coffee_asset: Address, new_valuation: u128) {
        oracle.require_auth();

        // Verify caller is authorized oracle
        let stored_oracle: Address = env.storage().instance().get(&VALUATION_ORACLE).unwrap();
        if oracle != stored_oracle {
            panic!("Only valuation oracle can update valuations");
        }

        let mut collateral_info: CollateralInfo = env
            .storage()
            .persistent()
            .get(&(COLLATERAL.clone(), coffee_asset.clone()))
            .unwrap_or_else(|| panic!("Coffee asset not found"));

        let old_valuation = collateral_info.estimated_value_usd;
        collateral_info.estimated_value_usd = new_valuation;

        env.storage().persistent().set(
            &(COLLATERAL.clone(), coffee_asset.clone()),
            &collateral_info,
        );

        log!(
            &env,
            "Updated valuation for coffee asset {} from ${} to ${}",
            coffee_asset,
            old_valuation,
            new_valuation
        );
    }

    /// Get collateral information
    pub fn get_collateral_info(env: Env, coffee_asset: Address) -> Option<CollateralInfo> {
        env.storage()
            .persistent()
            .get(&(COLLATERAL.clone(), coffee_asset))
    }

    /// Get collateral for a loan
    pub fn get_loan_collateral(env: Env, loan_id: BytesN<32>) -> Option<Address> {
        env.storage()
            .persistent()
            .get(&(LOAN_COLLATERAL.clone(), loan_id))
    }

    /// Get coffee batch details
    pub fn get_coffee_details(
        env: Env,
        coffee_asset: Address,
    ) -> (String, String, String, Address) {
        let batch_id: String = env
            .storage()
            .persistent()
            .get(&(Symbol::new(&env, "batch_id"), coffee_asset.clone()))
            .unwrap_or(String::from_str(&env, ""));

        let farm_location: String = env
            .storage()
            .persistent()
            .get(&(Symbol::new(&env, "farm_location"), coffee_asset.clone()))
            .unwrap_or(String::from_str(&env, ""));

        let harvest_date: String = env
            .storage()
            .persistent()
            .get(&(Symbol::new(&env, "harvest_date"), coffee_asset.clone()))
            .unwrap_or(String::from_str(&env, ""));

        let issuer: Address = env
            .storage()
            .persistent()
            .get(&(Symbol::new(&env, "issuer"), coffee_asset.clone()))
            .unwrap_or_else(|| panic!("Issuer not found for coffee asset"));

        (batch_id, farm_location, harvest_date, issuer)
    }

    /// List all active collateral assets
    pub fn list_active_collateral(env: Env) -> Vec<Address> {
        // This is a simplified implementation
        // In practice, you'd want to maintain an index of active assets
        Vec::new(&env) // Placeholder - would need proper indexing
    }

    /// Mark collateral as expired (for time-sensitive coffee)
    pub fn mark_expired(env: Env, admin: Address, coffee_asset: Address) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if admin != stored_admin {
            panic!("Only admin can mark collateral as expired");
        }

        let mut collateral_info: CollateralInfo = env
            .storage()
            .persistent()
            .get(&(COLLATERAL.clone(), coffee_asset.clone()))
            .unwrap_or_else(|| panic!("Coffee asset not found"));

        collateral_info.status = CollateralStatus::Expired;
        env.storage().persistent().set(
            &(COLLATERAL.clone(), coffee_asset.clone()),
            &collateral_info,
        );

        log!(&env, "Marked coffee asset {} as expired", coffee_asset);
    }

    /// Calculate required collateral value for loan amount
    pub fn calculate_required_collateral(_env: Env, loan_amount: u128) -> u128 {
        (loan_amount * COLLATERAL_RATIO_BASIS_POINTS) / 10000
    }
}
