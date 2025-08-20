#!/bin/bash

# Setup development environment for Coffee Yield Vaults

set -e

echo "🔧 Setting up Coffee Yield Vaults development environment..."

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env
else
    echo "✅ Rust found: $(rustc --version)"
fi

# Add wasm32 target for Soroban
echo "📦 Adding wasm32-unknown-unknown target..."
rustup target add wasm32-unknown-unknown

# Check if Soroban CLI is installed
if ! command -v stellar &> /dev/null; then
    echo "📥 Installing Soroban CLI..."
    cargo install --locked soroban-cli --version 23.0.0
else
    echo "✅ Soroban CLI found: $(stellar --version)"
fi

# Configure Stellar networks
echo "🌐 Configuring Stellar networks..."
stellar network add testnet --rpc-url https://soroban-testnet.stellar.org:443 --network-passphrase "Test SDF Network ; September 2015" || true
stellar network add mainnet --rpc-url https://soroban-mainnet.stellar.org:443 --network-passphrase "Public Global Stellar Network ; September 2015" || true

# Generate development identity
echo "🔑 Setting up development identity..."
stellar keys generate dev-identity || echo "Identity 'dev-identity' already exists"

# Fund the development account
echo "💰 Funding development account..."
DEV_ADDRESS=$(stellar keys address dev-identity)
echo "Development address: $DEV_ADDRESS"
curl -s "https://friendbot.stellar.org?addr=$DEV_ADDRESS" > /dev/null && echo "✅ Account funded successfully" || echo "❌ Failed to fund account. Visit https://friendbot.stellar.org?addr=$DEV_ADDRESS to fund manually"

# Create project structure validation
echo "📁 Validating project structure..."
EXPECTED_DIRS=("contracts/yield-token" "contracts/usdc-vault" "contracts/gold-vault" "contracts/coffee-collateral" "contracts/governance" "contracts/shared")

for dir in "${EXPECTED_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo "✅ $dir exists"
    else
        echo "❌ $dir missing"
    fi
done

echo "🔍 Checking Cargo.toml files..."
for contract in yield-token usdc-vault gold-vault coffee-collateral governance shared; do
    if [ -f "contracts/$contract/Cargo.toml" ]; then
        echo "✅ contracts/$contract/Cargo.toml exists"
    else
        echo "❌ contracts/$contract/Cargo.toml missing"
    fi
done

echo "🎯 Environment setup complete!"
echo ""
echo "📚 Next steps:"
echo "  1. Verify account funding: https://stellar.expert/explorer/testnet/account/$DEV_ADDRESS"
echo "  2. Fix compilation errors: cargo check (see PROJECT_STATUS_REPORT.md)"
echo "  3. Build contracts: cargo build --target wasm32-unknown-unknown --release"
echo "  4. Deploy contracts: ./scripts/deploy.sh"
echo ""
echo "📖 Useful commands:"
echo "  - Build all: cargo build"
echo "  - Test all: cargo test"
echo "  - Deploy: ./scripts/deploy.sh"
echo "  - Soroban CLI help: stellar --help"