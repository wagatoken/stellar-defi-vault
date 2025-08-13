#!/bin/bash

# Coffee Yield Vaults - Deployment Script
# This script deploys all contracts to Stellar Soroban testnet

set -e

echo "🚀 Starting Coffee Yield Vaults deployment..."

# Network configuration
NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org:443"

# Build all contracts
echo "📦 Building contracts..."
cd contracts

# Build shared library first
echo "Building shared types..."
cd shared && cargo build --target wasm32-unknown-unknown --release && cd ..

# Build all contract modules
for contract in yield-token usdc-vault gold-vault coffee-collateral governance; do
    echo "Building $contract..."
    cd $contract
    cargo build --target wasm32-unknown-unknown --release
    cd ..
done

cd ..

echo "✅ All contracts built successfully!"

# Deploy contracts (placeholder addresses - actual deployment would require proper setup)
echo "🌐 Deploying to Stellar testnet..."

# Note: Actual deployment would require:
# 1. Soroban CLI properly configured with identity
# 2. Network funding for deployment costs
# 3. Proper contract optimization and wasm files

echo "📋 Deployment checklist:"
echo "  [ ] Install Soroban CLI: cargo install --locked soroban-cli"
echo "  [ ] Configure identity: soroban identity generate deploy-key"
echo "  [ ] Fund deployer account: soroban account fund <address>"
echo "  [ ] Deploy contracts in order:"
echo "      1. shared (if needed as library)"
echo "      2. yield-token"
echo "      3. usdc-vault"
echo "      4. gold-vault" 
echo "      5. coffee-collateral"
echo "      6. governance"
echo "  [ ] Initialize all contracts with proper parameters"
echo "  [ ] Set up inter-contract permissions"

echo "⚠️  Remember to update asset addresses in shared/src/lib.rs with actual Stellar asset addresses"
echo "⚠️  Update oracle contracts with real price feed addresses"
echo "⚠️  Set up proper multi-sig committee members"

echo "🎉 Deployment script completed. Please follow the manual checklist above."
