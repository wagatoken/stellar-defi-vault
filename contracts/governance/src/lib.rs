#![no_std]
use shared::{
    CommitteeMember, GovernanceProposal, LoanProposal, ProposalStatus, ProtocolParameter,
    TradeParams, REQUIRED_COMMITTEE_APPROVALS, TOTAL_COMMITTEE_SIZE,
};
use soroban_sdk::{
    contract, contractimpl, log, symbol_short, xdr::ToXdr, Address, BytesN, Env, IntoVal, Symbol,
    Vec,
};

// Storage Keys
const COMMITTEE_MEMBERS: Symbol = symbol_short!("MEMBERS");
const LOAN_PROPOSALS: Symbol = symbol_short!("LOANS");
const TRADE_PROPOSALS: Symbol = symbol_short!("TRADES");
const GOVERNANCE_PROPOSALS: Symbol = symbol_short!("GOV");
const PROPOSAL_COUNTER: Symbol = symbol_short!("COUNTER");
const ADMIN: Symbol = symbol_short!("ADMIN");
const YIELD_TOKEN: Symbol = symbol_short!("YIELD");
const MIN_PROPOSAL_TOKENS: Symbol = symbol_short!("MIN_TOK");

#[contract]
pub struct Governance;

#[contractimpl]
impl Governance {
    /// Initialize the governance contract
    pub fn initialize(
        env: Env,
        admin: Address,
        yield_token_contract: Address,
        initial_committee: Vec<CommitteeMember>,
        min_proposal_tokens: u128,
    ) {
        admin.require_auth();

        if initial_committee.len() != TOTAL_COMMITTEE_SIZE {
            panic!(
                "Committee must have exactly {} members",
                TOTAL_COMMITTEE_SIZE
            );
        }

        env.storage().instance().set(&ADMIN, &admin);
        env.storage()
            .instance()
            .set(&YIELD_TOKEN, &yield_token_contract);
        env.storage()
            .instance()
            .set(&COMMITTEE_MEMBERS, &initial_committee);
        env.storage()
            .instance()
            .set(&MIN_PROPOSAL_TOKENS, &min_proposal_tokens);
        env.storage().instance().set(&PROPOSAL_COUNTER, &0u64);

        log!(
            &env,
            "Governance contract initialized with {} committee members",
            initial_committee.len()
        );
    }

    /// Submit a loan proposal (committee members only)
    pub fn submit_loan_proposal(
        env: Env,
        proposer: Address,
        borrower: Address,
        loan_amount: u128,
        collateral_asset: Address,
        interest_rate: u128,
        duration_days: u64,
    ) -> BytesN<32> {
        proposer.require_auth();
        Self::verify_committee_member(&env, &proposer);
        // Generate proposal ID using serialization
        let mut proposal_bytes = soroban_sdk::Bytes::new(&env);
        let borrower_xdr = borrower.clone().to_xdr(&env);
        for b in borrower_xdr.iter() {
            proposal_bytes.push_back(b);
        }
        proposal_bytes.extend_from_array(&loan_amount.to_be_bytes());
        proposal_bytes.extend_from_array(&env.ledger().timestamp().to_be_bytes());
        let proposal_id: BytesN<32> = env.crypto().sha256(&proposal_bytes).into();
        let proposal = LoanProposal {
            id: proposal_id.clone(),
            borrower,
            amount: loan_amount,
            collateral: collateral_asset,
            interest_rate,
            duration: duration_days,
            approvals: 0,
            status: ProposalStatus::Pending,
            created_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&(LOAN_PROPOSALS.clone(), proposal_id.clone()), &proposal);

        log!(
            &env,
            "Loan proposal {} submitted by {} for borrower {} amount: ${}",
            proposal_id,
            proposer,
            proposal.borrower,
            loan_amount
        );

        proposal_id
    }

    /// Approve a loan proposal (committee members only)
    pub fn approve_loan(env: Env, proposal_id: BytesN<32>, approver: Address) {
        approver.require_auth();
        Self::verify_committee_member(&env, &approver);
        let mut proposal: LoanProposal = env
            .storage()
            .persistent()
            .get(&(LOAN_PROPOSALS.clone(), proposal_id.clone()))
            .unwrap_or_else(|| panic!("Loan proposal not found"));
        if proposal.status != ProposalStatus::Pending {
            panic!("Proposal is not in pending status");
        }
        let approval_key = (
            Symbol::new(&env, "approval"),
            proposal_id.clone(),
            approver.clone(),
        );
        if env.storage().persistent().has(&approval_key) {
            panic!("Member has already approved this proposal");
        }
        env.storage().persistent().set(&approval_key, &true);
        proposal.approvals += 1;
        if proposal.approvals >= REQUIRED_COMMITTEE_APPROVALS {
            proposal.status = ProposalStatus::Approved;
        }
        env.storage()
            .persistent()
            .set(&(LOAN_PROPOSALS.clone(), proposal_id.clone()), &proposal);
        log!(
            &env,
            "Loan proposal {} approved by {}. Approvals: {}/{}",
            proposal_id,
            approver,
            proposal.approvals,
            REQUIRED_COMMITTEE_APPROVALS
        );
    }

    /// Execute an approved loan
    pub fn execute_loan(env: Env, executor: Address, proposal_id: BytesN<32>) {
        executor.require_auth();
        Self::verify_committee_member(&env, &executor);
        let mut proposal: LoanProposal = env
            .storage()
            .persistent()
            .get(&(LOAN_PROPOSALS.clone(), proposal_id.clone()))
            .unwrap_or_else(|| panic!("Loan proposal not found"));
        if proposal.status != ProposalStatus::Approved {
            panic!("Proposal must be approved before execution");
        }
        // TODO: Implement actual loan execution logic
        // This would involve:
        // 1. Verifying collateral with coffee collateral contract
        // 2. Transferring funds from vault to borrower
        // 3. Recording loan terms and repayment schedule

        proposal.status = ProposalStatus::Executed;
        env.storage()
            .persistent()
            .set(&(LOAN_PROPOSALS.clone(), proposal_id.clone()), &proposal);
        log!(
            &env,
            "Loan proposal {} executed by {}. Amount: ${} to borrower: {}",
            proposal_id,
            executor,
            proposal.amount,
            proposal.borrower
        );
    }

    /// Submit a trade proposal (committee members only)
    pub fn submit_trade_proposal(
        env: Env,
        proposer: Address,
        trade_params: TradeParams,
    ) -> BytesN<32> {
        proposer.require_auth();
        Self::verify_committee_member(&env, &proposer);
        // Generate trade ID using serialization
        let mut trade_bytes = soroban_sdk::Bytes::new(&env);
        let asset_in_xdr = trade_params.asset_in.clone().to_xdr(&env);
        for b in asset_in_xdr.iter() {
            trade_bytes.push_back(b);
        }
        trade_bytes.extend_from_array(&trade_params.amount_in.to_be_bytes());
        trade_bytes.extend_from_array(&env.ledger().timestamp().to_be_bytes());
        let trade_id: BytesN<32> = env.crypto().sha256(&trade_bytes).into();

        env.storage()
            .persistent()
            .set(&(TRADE_PROPOSALS.clone(), trade_id.clone()), &trade_params);

        log!(
            &env,
            "Trade proposal {} submitted: {} {} for {} {}",
            trade_id,
            trade_params.amount_in,
            trade_params.asset_in,
            trade_params.min_amount_out,
            trade_params.asset_out
        );

        trade_id
    }

    /// Execute a trade (committee members only)
    pub fn execute_trade(env: Env, executor: Address, trade_id: BytesN<32>) {
        executor.require_auth();
        Self::verify_committee_member(&env, &executor);
        let trade_params: TradeParams = env
            .storage()
            .persistent()
            .get(&(TRADE_PROPOSALS.clone(), trade_id.clone()))
            .unwrap_or_else(|| panic!("Trade proposal not found"));
        if env.ledger().timestamp() > trade_params.deadline {
            panic!("Trade proposal has expired");
        }
        // TODO: Implement actual trade execution logic
        // This would involve:
        // 1. Calling external DEX or trading platform
        // 2. Verifying slippage constraints
        // 3. Recording profit/loss for yield distribution

        // Remove executed trade
        env.storage()
            .persistent()
            .remove(&(TRADE_PROPOSALS.clone(), trade_id.clone()));
        log!(&env, "Trade {} executed by {}", trade_id, executor);
    }

    /// DAO Governance: Propose parameter change
    pub fn propose_parameter_change(
        env: Env,
        proposer: Address,
        parameter: ProtocolParameter,
        new_value: u128,
    ) -> BytesN<32> {
        proposer.require_auth();
        let min_tokens: u128 = env.storage().instance().get(&MIN_PROPOSAL_TOKENS).unwrap();
        let proposer_balance = Self::get_voting_power(&env, &proposer);
        if proposer_balance < min_tokens {
            panic!(
                "Insufficient tokens to propose. Required: {}, Have: {}",
                min_tokens, proposer_balance
            );
        }
        // Generate proposal ID using serialization
        let mut prop_bytes = soroban_sdk::Bytes::new(&env);
        let proposer_xdr = proposer.clone().to_xdr(&env);
        for b in proposer_xdr.iter() {
            prop_bytes.push_back(b);
        }
        prop_bytes.extend_from_array(&(parameter.clone() as u32).to_be_bytes());
        prop_bytes.extend_from_array(&new_value.to_be_bytes());
        prop_bytes.extend_from_array(&env.ledger().timestamp().to_be_bytes());
        let proposal_id: BytesN<32> = env.crypto().sha256(&prop_bytes).into();
        let proposal = GovernanceProposal {
            id: proposal_id.clone(),
            proposer,
            parameter,
            new_value,
            votes_for: 0,
            votes_against: 0,
            voting_deadline: env.ledger().timestamp() + (7 * 24 * 60 * 60), // 7 days
            status: ProposalStatus::Pending,
        };
        env.storage().persistent().set(
            &(GOVERNANCE_PROPOSALS.clone(), proposal_id.clone()),
            &proposal,
        );

        log!(
            &env,
            "Governance proposal {} submitted by {} for parameter {:?}",
            proposal_id,
            proposal.proposer,
            proposal.parameter
        );

        proposal_id
    }

    /// DAO Governance: Vote on parameter change
    pub fn vote_on_proposal(env: Env, voter: Address, proposal_id: BytesN<32>, support: bool) {
        voter.require_auth();
        let mut proposal: GovernanceProposal = env
            .storage()
            .persistent()
            .get(&(GOVERNANCE_PROPOSALS.clone(), proposal_id.clone()))
            .unwrap_or_else(|| panic!("Governance proposal not found"));
        if env.ledger().timestamp() > proposal.voting_deadline {
            panic!("Voting period has ended");
        }
        if proposal.status != ProposalStatus::Pending {
            panic!("Proposal is not active for voting");
        }
        let vote_key = (
            Symbol::new(&env, "vote"),
            proposal_id.clone(),
            voter.clone(),
        );
        if env.storage().persistent().has(&vote_key) {
            panic!("User has already voted on this proposal");
        }
        let voting_power = Self::get_voting_power(&env, &voter);
        if support {
            proposal.votes_for += voting_power;
        } else {
            proposal.votes_against += voting_power;
        }
        env.storage().persistent().set(&vote_key, &support);
        env.storage().persistent().set(
            &(GOVERNANCE_PROPOSALS.clone(), proposal_id.clone()),
            &proposal,
        );
        log!(
            &env,
            "User {} voted {} on proposal {} with {} voting power",
            voter,
            if support { "FOR" } else { "AGAINST" },
            proposal_id,
            voting_power
        );
    }

    /// Execute approved governance proposal
    pub fn execute_governance_proposal(env: Env, executor: Address, proposal_id: BytesN<32>) {
        executor.require_auth();
        let mut proposal: GovernanceProposal = env
            .storage()
            .persistent()
            .get(&(GOVERNANCE_PROPOSALS.clone(), proposal_id.clone()))
            .unwrap_or_else(|| panic!("Governance proposal not found"));
        if env.ledger().timestamp() <= proposal.voting_deadline {
            panic!("Voting period has not ended");
        }
        if proposal.votes_for <= proposal.votes_against {
            proposal.status = ProposalStatus::Rejected;
            env.storage().persistent().set(
                &(GOVERNANCE_PROPOSALS.clone(), proposal_id.clone()),
                &proposal,
            );
            panic!("Proposal was rejected by vote");
        }
        // TODO: Implement actual parameter update logic
        // This would involve updating the relevant protocol parameters

        proposal.status = ProposalStatus::Executed;
        env.storage().persistent().set(
            &(GOVERNANCE_PROPOSALS.clone(), proposal_id.clone()),
            &proposal,
        );
        log!(
            &env,
            "Governance proposal {} executed. Parameter {:?} updated to {}",
            proposal_id,
            proposal.parameter,
            proposal.new_value
        );
    }

    /// Get loan proposal details
    pub fn get_loan_proposal(env: Env, proposal_id: BytesN<32>) -> Option<LoanProposal> {
        env.storage()
            .persistent()
            .get(&(LOAN_PROPOSALS.clone(), proposal_id))
    }

    /// Get governance proposal details
    pub fn get_governance_proposal(
        env: Env,
        proposal_id: BytesN<32>,
    ) -> Option<GovernanceProposal> {
        env.storage()
            .persistent()
            .get(&(GOVERNANCE_PROPOSALS.clone(), proposal_id))
    }

    /// Get committee members
    pub fn get_committee_members(env: Env) -> Vec<CommitteeMember> {
        env.storage()
            .instance()
            .get(&COMMITTEE_MEMBERS)
            .unwrap_or(Vec::new(&env))
    }

    /// Update committee (admin only)
    pub fn update_committee(env: Env, admin: Address, new_committee: Vec<CommitteeMember>) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if admin != stored_admin {
            panic!("Only admin can update committee");
        }

        if new_committee.len() != TOTAL_COMMITTEE_SIZE {
            panic!(
                "Committee must have exactly {} members",
                TOTAL_COMMITTEE_SIZE
            );
        }

        env.storage()
            .instance()
            .set(&COMMITTEE_MEMBERS, &new_committee);

        log!(&env, "Committee updated by admin");
    }

    /// Internal helper functions
    fn verify_committee_member(env: &Env, member: &Address) {
        let committee: Vec<CommitteeMember> = env
            .storage()
            .instance()
            .get(&COMMITTEE_MEMBERS)
            .unwrap_or(Vec::new(env));

        for committee_member in committee.iter() {
            if committee_member.address == *member {
                return;
            }
        }

        panic!("Address is not a committee member");
    }

    fn get_voting_power(env: &Env, user: &Address) -> u128 {
        let yield_token_contract: Address = env.storage().instance().get(&YIELD_TOKEN).unwrap();

        // Get user's token balance as voting power
        let balance: i128 = env.invoke_contract(
            &yield_token_contract,
            &Symbol::new(env, "balance"),
            (user.clone(),).into_val(env),
        );

        balance as u128
    }
}
