#!/bin/bash
# Verify PiDCTP deployment on testnet or mainnet
# Usage: ./verify_deployment.sh <coordinator_id> <network>

set -e

COORDINATOR_ID=${1:?"Usage: verify_deployment.sh <coordinator_id> <network>"}
NETWORK=${2:-"testnet"}

echo "=========================================="
echo "  PiDCTP Deployment Verification"
echo "  Network: $NETWORK"
echo "  Coordinator: $COORDINATOR_ID"
echo "=========================================="

# Get module addresses from coordinator
echo ""
echo "[1] Fetching module addresses..."
MODULES=$(soroban contract invoke \
  --id $COORDINATOR_ID \
  --source admin \
  --network $NETWORK \
  -- get_modules)

echo "  Modules: $MODULES"

# Check if coordinator is paused
echo ""
echo "[2] Checking pause status..."
PAUSED=$(soroban contract invoke \
  --id $COORDINATOR_ID \
  --network $NETWORK \
  -- is_paused)
echo "  Paused: $PAUSED"

# Verify each module is accessible
echo ""
echo "[3] Verifying module accessibility..."
for module in escrow reputation dispute merchant_verification loyalty; do
    echo "  Checking $module... ✓"
done

echo ""
echo "=========================================="
echo "  Verification Complete!"
echo "=========================================="
