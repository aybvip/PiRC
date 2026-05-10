#!/bin/bash
# PiDCTP Mainnet Deployment Script
# Usage: ./deploy_mainnet.sh
# WARNING: Ensure all audits and reviews are complete before mainnet deployment!

set -e

NETWORK="mainnet"
SOROBAN_RPC="https://mainnet.sorobanrpc.com"

echo "=========================================="
echo "  PiDCTP Mainnet Deployment"
echo "  ⚠️  PRODUCTION DEPLOYMENT"
echo "=========================================="

# Pre-deployment checklist
echo ""
echo "Pre-deployment checklist:"
echo "  [ ] Security audit completed"
echo "  [ ] All audit findings addressed"
echo "  [ ] Testnet testing (2+ weeks)"
echo "  [ ] Community review period completed"
echo "  [ ] Pi Foundation approval received"
echo "  [ ] Admin multi-sig keys distributed"
echo "  [ ] Timelock contract configured"
echo ""
read -p "Have all checklist items been completed? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Deployment aborted. Complete the checklist first."
    exit 1
fi

# Build optimized contracts
echo "[1/8] Building optimized contracts..."
soroban contract build --optimize
echo "  ✓ Contracts built with optimization"

# Verify build artifacts
echo "[2/8] Verifying build artifacts..."
for contract in escrow reputation dispute merchant loyalty coordinator; do
    WASM="target/wasm32-unknown-unknown/release/pidctp_${contract}.wasm"
    if [ ! -f "$WASM" ]; then
        echo "  ✗ Missing: $WASM"
        exit 1
    fi
    SIZE=$(stat -f%z "$WASM" 2>/dev/null || stat -c%s "$WASM" 2>/dev/null)
    echo "  ✓ ${contract}: ${SIZE} bytes"
done

# Deploy contracts (same order as testnet)
echo "[3/8] Deploying Escrow..."
ESCROW_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_escrow.wasm \
  --source admin \
  --network $NETWORK)
echo "  ✓ Escrow: $ESCROW_ID"

echo "[4/8] Deploying Reputation..."
REPUTATION_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_reputation.wasm \
  --source admin \
  --network $NETWORK)
echo "  ✓ Reputation: $REPUTATION_ID"

echo "[5/8] Deploying Dispute..."
DISPUTE_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_dispute.wasm \
  --source admin \
  --network $NETWORK)
echo "  ✓ Dispute: $DISPUTE_ID"

echo "[6/8] Deploying Merchant..."
MERCHANT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_merchant.wasm \
  --source admin \
  --network $NETWORK)
echo "  ✓ Merchant: $MERCHANT_ID"

echo "[7/8] Deploying Loyalty..."
LOYALTY_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_loyalty.wasm \
  --source admin \
  --network $NETWORK)
echo "  ✓ Loyalty: $LOYALTY_ID"

echo "[8/8] Deploying Coordinator..."
COORDINATOR_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/pidctp_coordinator.wasm \
  --source admin \
  --network $NETWORK)
echo "  ✓ Coordinator: $COORDINATOR_ID"

# Initialize all contracts
echo ""
echo "Initializing contracts..."

soroban contract invoke --id $ESCROW_ID --source admin --network $NETWORK \
  -- initialize --coordinator $COORDINATOR_ID --fee_recipient $TREASURY --fee_bps 100

soroban contract invoke --id $REPUTATION_ID --source admin --network $NETWORK \
  -- initialize --coordinator $COORDINATOR_ID

soroban contract invoke --id $DISPUTE_ID --source admin --network $NETWORK \
  -- initialize --coordinator $COORDINATOR_ID

soroban contract invoke --id $MERCHANT_ID --source admin --network $NETWORK \
  -- initialize --coordinator $COORDINATOR_ID

soroban contract invoke --id $LOYALTY_ID --source admin --network $NETWORK \
  -- initialize --coordinator $COORDINATOR_ID

soroban contract invoke --id $COORDINATOR_ID --source admin --network $NETWORK \
  -- initialize --admin $ADMIN --escrow $ESCROW_ID --reputation $REPUTATION_ID \
  --dispute $DISPUTE_ID --merchant_verification $MERCHANT_ID --loyalty $LOYALTY_ID \
  --treasury $TREASURY

echo ""
echo "=========================================="
echo "  Mainnet Deployment Complete!"
echo "=========================================="
echo ""
echo "  ⚠️  Record these addresses securely:"
echo "  Coordinator: $COORDINATOR_ID"
echo "  Escrow:      $ESCROW_ID"
echo "  Reputation:  $REPUTATION_ID"
echo "  Dispute:     $DISPUTE_ID"
echo "  Merchant:    $MERCHANT_ID"
echo "  Loyalty:     $LOYALTY_ID"
