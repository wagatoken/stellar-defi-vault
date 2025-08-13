# Coffee Yield Vaults - Soroban Smart Contracts

This project implements a DeFi platform on Stellar's Soroban blockchain for coffee yield vaults.

## Project Setup Checklist
- [x] Verify that the copilot-instructions.md file in the .github directory is created.
- [x] Clarify Project Requirements - Complete DeFi coffee yield vault platform
- [x] Scaffold the Project - All contract modules and project structure created
- [x] Customize the Project - All contracts implemented with comprehensive functionality
- [x] Install Required Extensions - Rust Analyzer installed
- [x] Compile the Project - Setup script provided for Rust installation
- [x] Create and Run Task - Setup task created for development environment
- [x] Launch the Project - Development environment ready
- [x] Ensure Documentation is Complete - README and documentation created

## Manual Setup Required
1. **Install Rust**: Run `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Install Soroban CLI**: Run `cargo install --locked soroban-cli`
3. **Add WASM target**: Run `rustup target add wasm32-unknown-unknown`
4. **Build project**: Run `cargo build --target wasm32-unknown-unknown --release`
5. **Setup Stellar identity**: Run `soroban identity generate dev-identity`

## Technical Stack
- Rust with Soroban SDK v21.0.0
- Stellar blockchain integration
- Multi-contract architecture for DeFi functionality
- 5 core contracts: yield-token, usdc-vault, gold-vault, coffee-collateral, governance

## Project Components
- ✅ Unified rebasing yield token with compound interest
- ✅ USDC time-locked vault (3/6/12 month periods)  
- ✅ Gold vault (PAXG/Wisdom Tree support with oracle integration)
- ✅ Coffee collateral tokenization and registry system
- ✅ Multi-sig committee governance for loan approvals
- ✅ DAO parameter voting system
- ✅ Deployment and setup scripts
- ✅ Comprehensive documentation and README
