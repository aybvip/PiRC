# PiRC3 — Section 12: Implementation Guide

## Overview

This section provides a practical guide for implementing PiDCTP, including development setup, contract implementation order, testing strategy, and deployment procedures.

## Development Environment Setup

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.70+ | Smart contract development |
| Soroban CLI | 20.0+ | Contract building, testing, deployment |
| Stellar CLI | 20.0+ | Network interaction |
| Node.js | 18+ | Frontend/SDK development |
| freight | latest | Rust package manager |

### Project Structure

```
pidctp/
├── contracts/
│   ├── coordinator/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── escrow_router.rs
│   │   │   ├── dispute_router.rs
│   │   │   └── access_control.rs
│   │   └── Cargo.toml
│   ├── escrow/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── escrow_account.rs
│   │   │   ├── lifecycle.rs
│   │   │   └── timeout.rs
│   │   └── Cargo.toml
│   ├── reputation/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── score.rs
│   │   │   ├── decay.rs
│   │   │   └── anti_gaming.rs
│   │   └── Cargo.toml
│   ├── dispute/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── juror_selection.rs
│   │   │   ├── voting.rs
│   │   │   ├── commit_reveal.rs
│   │   │   └── ruling.rs
│   │   └── Cargo.toml
│   ├── merchant/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── verification.rs
│   │   │   ├── rating.rs
│   │   │   └── agent.rs
│   │   └── Cargo.toml
│   └── loyalty/
│       ├── src/
│       │   ├── lib.rs
│       │   ├── points.rs
│       │   ├── tiers.rs
│       │   └── rewards.rs
│       └── Cargo.toml
├── shared/
│   ├── src/
│   │   ├── types.rs
│   │   ├── errors.rs
│   │   └── events.rs
│   └── Cargo.toml
├── tests/
│   ├── integration/
│   │   ├── escrow_flow_test.rs
│   │   ├── dispute_flow_test.rs
│   │   ├── reputation_update_test.rs
│   │   ├── merchant_verification_test.rs
│   │   └── loyalty_earning_test.rs
│   └── fuzz/
│       ├── escrow_edge_cases.rs
│       └── reputation_score_calc.rs
├── scripts/
│   ├── deploy_testnet.sh
│   ├── deploy_mainnet.sh
│   └── verify_deployment.sh
├── docs/
│   └── api_reference.md
├── Cargo.toml
└── README.md
```

## Implementation Order

Implement contracts in dependency order — each step builds on the previous:

```
Phase 1: Foundation (Weeks 1-3)
├── shared/types.rs
├── shared/errors.rs
├── shared/events.rs
└── escrow/ (standalone, no dependencies)

Phase 2: Core Modules (Weeks 4-7)
├── reputation/ (depends on escrow events)
├── dispute/ (depends on escrow + reputation)
└── loyalty/ (depends on escrow + reputation)

Phase 3: Extended Modules (Weeks 8-10)
├── merchant/ (depends on reputation + escrow)
└── coordinator/ (depends on all modules)

Phase 4: Integration & Testing (Weeks 11-14)
├── PiRC2 integration layer
├── Integration tests
├── Security audit
└── Testnet deployment
```

## Contract Implementation Templates

### Escrow Contract Skeleton

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec, BytesN, token};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn initialize(env: Env, coordinator: Address) {
        let storage = env.storage().persistent();
        assert!(!storage.has(&Symbol::new(&env, "coordinator")), "Already initialized");
        storage.set(&Symbol::new(&env, "coordinator"), &coordinator);
        storage.set(&Symbol::new(&env, "next_id"), &1u64);
    }

    pub fn create_escrow(
        env: Env,
        buyer: Address,
        seller: Address,
        amount: i128,
        token_addr: Address,
        delivery_deadline: u64,
        auto_release_timeout: u64,
        order_metadata: BytesN<32>,
    ) -> u64 {
        buyer.require_auth();
        assert!(buyer != seller, "Buyer and seller must differ");
        assert!(amount > 0, "Amount must be positive");
        assert!(delivery_deadline > env.ledger().timestamp(), "Deadline must be future");
        assert!(auto_release_timeout >= 86400, "Timeout minimum 1 day");

        let storage = env.storage().persistent();
        let escrow_id: u64 = storage.get(&Symbol::new(&env, "next_id")).unwrap_or(1);
        storage.set(&Symbol::new(&env, "next_id"), &(escrow_id + 1));

        let escrow = EscrowAccount {
            escrow_id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            amount,
            token: token_addr,
            state: EscrowState::Created,
            created_at: env.ledger().timestamp(),
            delivery_deadline,
            confirmation_deadline: 0,
            auto_release_timeout,
            subscription_id: None,
            order_metadata,
        };

        storage.set(&escrow_key(&env, escrow_id), &escrow);

        env.events().publish(
            (Symbol::new(&env, "ESCROW_CREATED"), escrow_id),
            (buyer, seller, amount),
        );

        escrow_id
    }

    pub fn fund_escrow(env: Env, buyer: Address, escrow_id: u64) {
        buyer.require_auth();
        let mut escrow: EscrowAccount = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Created, "Not in Created state");
        assert!(buyer == escrow.buyer, "Not the buyer");

        // Transfer tokens from buyer to this contract
        let client = token::Client::new(&env, &escrow.token);
        client.transfer(&buyer, &env.current_contract_address(), &escrow.amount);

        escrow.state = EscrowState::Funded;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new(&env, "ESCROW_FUNDED"), escrow_id),
            (escrow.amount,),
        );
    }

    pub fn confirm_delivery(env: Env, seller: Address, escrow_id: u64) {
        seller.require_auth();
        let mut escrow: EscrowAccount = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Funded, "Not in Funded state");
        assert!(seller == escrow.seller, "Not the seller");
        assert!(env.ledger().timestamp() <= escrow.delivery_deadline, "Deadline passed");

        escrow.state = EscrowState::Delivered;
        escrow.confirmation_deadline = env.ledger().timestamp() + escrow.auto_release_timeout;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new(&env, "ESCROW_DELIVERED"), escrow_id),
            (seller,),
        );
    }

    pub fn confirm_receipt(env: Env, buyer: Address, escrow_id: u64) {
        buyer.require_auth();
        let mut escrow: EscrowAccount = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Delivered, "Not in Delivered state");
        assert!(buyer == escrow.buyer, "Not the buyer");

        // Effects first
        escrow.state = EscrowState::Completed;
        set_escrow(&env, &escrow);

        // Then interactions
        let client = token::Client::new(&env, &escrow.token);
        let fee = escrow.amount / 100; // 1% fee
        let net = escrow.amount - fee;
        client.transfer(&env.current_contract_address(), &escrow.seller, &net);

        env.events().publish(
            (Symbol::new(&env, "ESCROW_COMPLETED"), escrow_id),
            (escrow.buyer, escrow.seller, net),
        );
    }
}
```

## Testing Strategy

### Unit Tests (Per Contract)

Each contract includes comprehensive unit tests:

```rust
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _};

    #[test]
    fn test_create_escrow() { /* ... */ }
    #[test]
    fn test_fund_escrow() { /* ... */ }
    #[test]
    fn test_confirm_delivery() { /* ... */ }
    #[test]
    fn test_confirm_receipt() { /* ... */ }
    #[test]
    fn test_auto_release() { /* ... */ }
    #[test]
    fn test_cancel_escrow() { /* ... */ }
    #[test]
    fn test_expire_escrow() { /* ... */ }
    #[test]
    #[should_panic(expected = "Not the buyer")]
    fn test_unauthorized_fund() { /* ... */ }
    #[test]
    #[should_panic(expected = "Deadline passed")]
    fn test_delivery_after_deadline() { /* ... */ }
}
```

### Integration Tests (Cross-Contract)

```rust
#[test]
fn test_full_commerce_flow() {
    let env = Env::default();
    // 1. Deploy all contracts
    // 2. Create and fund escrow
    // 3. Confirm delivery
    // 4. Confirm receipt
    // 5. Verify reputation updated
    // 6. Verify loyalty points awarded
}

#[test]
fn test_dispute_flow() {
    let env = Env::default();
    // 1. Create and fund escrow
    // 2. Open dispute
    // 3. Submit evidence
    // 4. Jurors vote
    // 5. Execute ruling
    // 6. Verify fund distribution
    // 7. Verify reputation impact
}

#[test]
fn test_appeal_flow() {
    let env = Env::default();
    // 1. Complete dispute flow
    // 2. Appeal ruling
    // 3. New jurors selected
    // 4. Final ruling executed
}
```

### Fuzz Testing

```rust
#[test]
fn fuzz_escrow_amounts() {
    // Test with random amounts from 1 stroop to 10M Pi
    // Verify no overflow/underflow
    // Verify fee calculation correctness
}

#[test]
fn fuzz_reputation_scores() {
    // Test with random transaction histories
    // Verify score stays within 0-1000
    // Verify tier transitions are correct
}
```

## Deployment Procedure

### Testnet Deployment

```bash
#!/bin/bash
# deploy_testnet.sh

NETWORK="testnet"
SOROBAN_RPC="https://soroban-testnet.stellar.org"

# 1. Build all contracts
soroban contract build

# 2. Deploy shared types (no-op, embedded in contracts)
echo "Deploying contracts to $NETWORK..."

# 3. Deploy escrow
ESCROW_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/escrow.wasm \
  --source admin \
  --network $NETWORK)
echo "Escrow: $ESCROW_ID"

# 4. Deploy reputation
REPUTATION_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/reputation.wasm \
  --source admin \
  --network $NETWORK)
echo "Reputation: $REPUTATION_ID"

# 5. Deploy dispute
DISPUTE_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/dispute.wasm \
  --source admin \
  --network $NETWORK)
echo "Dispute: $DISPUTE_ID"

# 6. Deploy merchant verification
MERCHANT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/merchant.wasm \
  --source admin \
  --network $NETWORK)
echo "Merchant: $MERCHANT_ID"

# 7. Deploy loyalty
LOYALTY_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/loyalty.wasm \
  --source admin \
  --network $NETWORK)
echo "Loyalty: $LOYALTY_ID"

# 8. Deploy coordinator (references all modules)
COORDINATOR_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/coordinator.wasm \
  --source admin \
  --network $NETWORK)
echo "Coordinator: $COORDINATOR_ID"

# 9. Initialize coordinator with module addresses
soroban contract invoke \
  --id $COORDINATOR_ID \
  --source admin \
  --network $NETWORK \
  -- initialize \
  --escrow $ESCROW_ID \
  --reputation $REPUTATION_ID \
  --dispute $DISPUTE_ID \
  --merchant $MERCHANT_ID \
  --loyalty $LOYALTY_ID

echo "Deployment complete!"
```

### Mainnet Deployment Checklist

- [ ] All contracts pass testnet testing (2+ weeks)
- [ ] Security audit completed by at least one firm
- [ ] All audit findings addressed
- [ ] Admin multi-sig keys distributed to 5 holders
- [ ] Timelock contract deployed and configured
- [ ] Emergency pause tested on testnet
- [ ] Pi Foundation approval received
- [ ] Community review period (2 weeks) completed
- [ ] Deployment transaction prepared and reviewed
- [ ] Post-deployment verification script run

## Monitoring & Operations

### Key Metrics to Monitor

| Metric | Alert Threshold | Action |
|--------|----------------|--------|
| Open escrows | > 10,000 | Scale indexer |
| Dispute rate | > 10% of escrows | Investigate ecosystem health |
| Juror participation | < 80% acceptance | Adjust juror incentives |
| Reputation score distribution | Skewed to Bronze | Review score algorithm |
| Loyalty point inflation | > 50,000 Pi pool | Adjust earning rates |
| Failed transactions | > 1% | Investigate contract issues |

### Operational Runbook

| Incident | Detection | Response | Recovery |
|----------|-----------|----------|----------|
| Contract bug | Failed tx spike | Emergency pause | Upgrade + fund recovery |
| Juror collusion | Correlated voting pattern | Suspend jurors | Appeal mechanism |
| Reputation manipulation | Rapid score changes | Rate limit + investigate | Score correction |
| Escrow fund stuck | Timeout not triggering | Manual expire trigger | Fund release |
| Network congestion | High gas fees | Fee adjustment | Normal operations |
