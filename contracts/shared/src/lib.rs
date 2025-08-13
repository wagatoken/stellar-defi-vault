use soroban_sdk::{contracttype, Address, BytesN};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum LockPeriod {
    ThreeMonths,  // Base yield rate
    SixMonths,    // 1.5x base rate
    TwelveMonths, // 2x base rate
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VaultType {
    USDC,
    PAXG,
    WisdomTreeGold,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct DepositInfo {
    pub amount: u128,
    pub deposit_time: u64,
    pub unlock_time: u64,
    pub lock_period: LockPeriod,
    pub vault_type: VaultType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct UserYieldInfo {
    pub principal: u128,
    pub yield_rate: u128, // Basis points (e.g., 500 = 5%)
    pub last_compound_time: u64,
    pub total_yield_earned: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum CollateralStatus {
    Active,
    Liquidated,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct CollateralInfo {
    pub asset_address: Address,
    pub quality_grade: u32,
    pub quantity_kg: u128,
    pub estimated_value_usd: u128,
    pub creation_time: u64,
    pub status: CollateralStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct LoanProposal {
    pub id: BytesN<32>,
    pub borrower: Address,
    pub amount: u128,
    pub collateral: Address,
    pub interest_rate: u128,
    pub duration: u64,
    pub approvals: u32,
    pub status: ProposalStatus,
    pub created_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ExpertiseArea {
    CoffeeIndustry,
    RiskManagement,
    Trading,
    Agriculture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct CommitteeMember {
    pub address: Address,
    pub expertise: ExpertiseArea,
    pub vote_weight: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ProtocolParameter {
    MinimumLockPeriod,
    MaximumYieldRate,
    CollateralRatio,
    ProtocolFeeRate,
    EmergencyWithdrawFee,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct GovernanceProposal {
    pub id: BytesN<32>,
    pub proposer: Address,
    pub parameter: ProtocolParameter,
    pub new_value: u128,
    pub votes_for: u128,
    pub votes_against: u128,
    pub voting_deadline: u64,
    pub status: ProposalStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ProfitReport {
    pub total_profit: u128,
    pub coffee_lending_profit: u128,
    pub trading_profit: u128,
    pub yield_distributed: u128,
    pub protocol_fee: u128,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct TradeParams {
    pub asset_in: Address,
    pub asset_out: Address,
    pub amount_in: u128,
    pub min_amount_out: u128,
    pub deadline: u64,
}

// Storage keys
pub const STORAGE_INSTANCE_PERSISTENT: u64 = 86400 * 365; // 1 year
pub const REBASE_INTERVAL: u64 = 86400; // 24 hours in seconds

// Protocol constants
pub const REQUIRED_COMMITTEE_APPROVALS: u32 = 3;
pub const TOTAL_COMMITTEE_SIZE: u32 = 5;
pub const PROTOCOL_FEE_BASIS_POINTS: u128 = 2000; // 20%
pub const YIELD_DISTRIBUTION_BASIS_POINTS: u128 = 8000; // 80%
pub const COLLATERAL_RATIO_BASIS_POINTS: u128 = 15000; // 150%

// Asset addresses (placeholders - will need to be updated with actual addresses)
pub const USDC_ASSET: &str = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
pub const PAXG_ASSET: &str = "PAXG:PLACEHOLDER_ADDRESS_FOR_PAXG";
pub const WISDOMTREE_GOLD: &str = "WTGOLD:PLACEHOLDER_ADDRESS_FOR_WISDOMTREE";
