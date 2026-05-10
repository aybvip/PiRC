#!/bin/bash
# PiDCTP Testnet Deployment Script
# Usage: ./deploy_testnet.sh

set -e

NETWORK="testnet"
SOROBAN_RPC="https://soroban-testnet.stellar.org:443"
FUTURENET_RPC="https://rpc-futurenet.stellar.org:443"

echo "=========================================="
echo "  PiDCTP Testnet Deployment"
echo "=========================================="

# Check prerequisites
echo "[1/8] Checking prerequisites..."
command -v soroban >/dev/null 2>&1 || { echo "Error: soroban CLI not installed"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "Error: cargo not installed"; exit 1; }

# Build all contracts
echo "[2/8] Building contracts..."
soroban contract build
echo "  ✓ All contracts built successfully"

# Generate identities for testing
echo "[3/8] Generating test identities..."
ADMIN=$(soroban keys generate admin --network $NETWORK 2>/dev/null || soroban keys address admin)
BUYER=$(soroban keys generate buyer --network $NETWORK 2>/dev/null || soroban keys address buyer)
SELLER=$(soroban keys generate seller --network $NETWORK 2>/dev/null || soroban keys address seller)
JUROR1=$(soroban keys generate juror1 --network $NETWORK 2>/dev/null || soroban keys address juror1)
JUROR2=$(soroban keys generate juror2 --network $NETWORK 2>/dev/null || soroban keys address juror2)
JUROR3=$(soroban keys generate juror3 --network $NETWORK 2>/dev/null || soroban keys address juror3)

echo "  Admin:   $ADMIN"
echo "  Buyer:   $BUYER"
echo "  Seller:  $SELLER"

# Deploy Escrow contract
echo "[4/8] Deploying Escrow contract..."
ESCROW_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_escrow.wasm \
  --source admin \
  --network $NETWORK \
  --rpc-url $SOROBAN_RPC)
echo "  ✓ Escrow: $ESCROW_ID"

# Deploy Reputation contract
echo "[5/8] Deploying Reputation contract..."
REPUTATION_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_reputation.wasm \
  --source admin \
  --network $NETWORK \
  --rpc-url $SOROBAN_RPC)
echo "  ✓ Reputation: $REPUTATION_ID"

# Deploy Dispute contract
echo "[6/8] Deploying Dispute contract..."
DISPUTE_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_dispute.wasm \
  --source admin \
  --network $NETWORK \
  --rpc-url $SOROBAN_RPC)
echo "  ✓ Dispute: $DISPUTE_ID"

# Deploy Merchant Verification contract
echo "[7/8] Deploying Merchant Verification contract..."
MERCHANT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_merchant.wasm \
  --source admin \
  --network $NETWORK \
  --rpc-url $SOROBAN_RPC)
echo "  ✓ Merchant: $MERCHANT_ID"

# Deploy Loyalty contract
LOYALTY_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_loyalty.wasm \
  --source admin \
  --network $NETWORK \
  --rpc-url $SOROBAN_RPC)
echo "  ✓ Loyalty: $LOYALTY_ID"

# Deploy Coordinator contract
echo "[8/8] Deploying Coordinator contract..."
COORDINATOR_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_coordinator.wasm \
  --source admin \
  --network $NETWORK \
  --rpc-url $SOROBAN_RPC)
echo "  ✓ Coordinator: $COORDINATOR_ID"

# Initialize all contracts
echo ""
echo "=========================================="
echo "  Initializing Contracts"
echo "=========================================="

# Initialize Escrow
soroban contract invoke \
  --id $ESCROW_ID \
  --source admin \
  --network $NETWORK \
  -- initialize \
  --coordinator $COORDINATOR_ID \
  --fee_recipient $ADMIN \
  --fee_bps 100

# Initialize Reputation
soroban contract invoke \
  --id $REPUTATION_ID \
  --source admin \
  --network $NETWORK \
  -- initialize \
  --coordinator $COORDINATOR_ID

# Initialize Dispute
soroban contract invoke \
  --id $DISPUTE_ID \
  --source admin \
  --network $NETWORK \
  -- initialize \
  --coordinator $COORDINATOR_ID

# Initialize Merchant
soroban contract invoke \
  --id $MERCHANT_ID \
  --source admin \
  --network $NETWORK \
  -- initialize \
  --coordinator $COORDINATOR_ID

# Initialize Loyalty
soroban contract invoke \
  --id $LOYALTY_ID \
  --source admin \
  --network $NETWORK \
  -- initialize \
  --coordinator $COORDINATOR_ID

# Initialize Coordinator with all module addresses
soroban contract invoke \
  --id $COORDINATOR_ID \
  --source admin \
  --network $NETWORK \
  -- initialize \
  --admin $ADMIN \
  --escrow $ESCROW_ID \
  --reputation $REPUTATION_ID \
  --dispute $DISPUTE_ID \
  --merchant_verification $MERCHANT_ID \
  --loyalty $LOYALTY_ID \
  --treasury $ADMIN

echo ""
echo "=========================================="
echo "  Deployment Complete!"
echo "=========================================="
echo ""
echo "  Coordinator: $COORDINATOR_ID"
echo "  Escrow:      $ESCROW_ID"
echo "  Reputation:  $REPUTATION_ID"
echo "  Dispute:     $DISPUTE_ID"
echo "  Merchant:    $MERCHANT_ID"
echo "  Loyalty:     $LOYALTY_ID"
echo ""
echo "  Network: $NETWORK"
echo "  RPC:     $SOROBAN_RPC"
