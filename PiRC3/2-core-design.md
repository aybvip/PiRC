# PiRC3 — Section 2: Core Design

## Architecture Overview

PiDCTP is composed of five interconnected modules, each implemented as an independent Soroban smart contract that communicates through a central coordinator contract.

```
┌─────────────────────────────────────────────────────────────────┐
│                     PiDCTP Coordinator                           │
│                   (Entry Point & Router)                         │
├─────────┬──────────┬──────────────┬────────────┬───────────────┤
│ Escrow  │Reputation│   Dispute    │  Merchant   │   Loyalty     │
│ System  │ Engine   │ Resolution   │ Verification│   Rewards     │
│         │          │              │             │               │
│ Hold &  │ Score &  │ Juror Select │ KYB Process │ Points &     │
│ Release │ History  │ & Arbitrate  │ & Badges    │ Tiers        │
│ Funds   │ Tracking │ Disputes     │ Verification│ Rewards      │
└─────────┴──────────┴──────────────┴────────────┴───────────────┘
         │           │              │             │
         └───────────┴──────────────┴─────────────┘
                        │
              ┌─────────┴─────────┐
              │  PiRC2 Subscription│
              │  (Integration)     │
              └────────────────────┘
```

## Module Interactions

### Transaction Flow (Happy Path)

```
Buyer                    Coordinator              Escrow              Seller
  │                          │                      │                   │
  │──Create Order──────────►│                      │                   │
  │                          │──Lock Funds────────►│                   │
  │                          │                      │──Notify──────────►│
  │                          │                      │                   │
  │                          │              [Seller Delivers Goods]    │
  │                          │                      │                   │
  │──Confirm Receipt───────►│                      │                   │
  │                          │──Release Funds──────►│──Transfer───────►│
  │                          │                      │                   │
  │                          │──Update Reputation──►│                   │
  │                          │──Award Loyalty──────►│                   │
  │◄──Transaction Complete──│                      │                   │
```

### Transaction Flow (Dispute Path)

```
Buyer                    Coordinator          Escrow           Dispute          Jurors
  │                          │                  │                │               │
  │──Open Dispute───────────►│                  │                │               │
  │                          │──Freeze Funds───►│                │               │
  │                          │                  │                │──Select Jurors►│
  │                          │                  │                │◄──Accept──────│
  │──Submit Evidence───────────────────────────────────────────►│               │
  │                          │                  │                │──Review──────►│
  │                          │                  │                │◄──Vote────────│
  │                          │                  │                │               │
  │                          │◄──Ruling─────────────────────────│               │
  │                          │──Execute Ruling─►│                │               │
  │                          │──Update Rep──────│                │               │
  │◄──Resolution─────────────│                  │                │               │
```

## Coordinator Contract

The `PiDCTPCoordinator` is the single entry point for all operations. It routes calls to the appropriate module and manages cross-module state transitions.

### Key Responsibilities

| Responsibility | Description |
|----------------|-------------|
| **Routing** | Directs external calls to the correct module contract |
| **State Machine** | Manages transaction lifecycle states |
| **Event Emission** | Emits standardized events for off-chain indexing |
| **Access Control** | Validates caller permissions before delegating |
| **Atomic Operations** | Ensures cross-module operations are atomic |

### Transaction States

```
┌───────────┐     ┌──────────┐     ┌───────────┐     ┌──────────┐
│  CREATED  │────►│  FUNDED  │────►│  DELIVERED│────►│ COMPLETED│
└───────────┘     └──────────┘     └───────────┘     └──────────┘
                       │                │
                       │                ▼
                       │          ┌───────────┐     ┌──────────┐
                       │          │  DISPUTED  │────►│ RESOLVED │
                       │          └───────────┘     └──────────┘
                       │
                       ▼
                  ┌──────────┐
                  │ EXPIRED  │
                  └──────────┘
                  ┌──────────┐
                  │ CANCELLED│
                  └──────────┘
```

| State | Description | Transition Trigger |
|-------|-------------|-------------------|
| `CREATED` | Order created, awaiting funding | Buyer creates order |
| `FUNDED` | Escrow funded, seller notified | Buyer deposits payment |
| `DELIVERED` | Seller marked as delivered | Seller confirms shipment |
| `COMPLETED` | Buyer confirmed receipt, funds released | Buyer confirms |
| `DISPUTED` | Buyer or seller opened dispute | Either party disputes |
| `RESOLVED` | Dispute resolved by jurors | Jurors reach majority |
| `EXPIRED` | Delivery timeout reached | Auto-expire after deadline |
| `CANCELLED` | Cancelled before funding | Buyer or mutual cancel |

## Contract Addresses & Deployment

Each module is deployed as an independent Soroban contract with a deterministic address. The coordinator holds references to all module addresses.

```rust
// Coordinator stores module references
struct ModuleAddresses {
    escrow: Address,
    reputation: Address,
    dispute: Address,
    merchant_verification: Address,
    loyalty: Address,
}
```

## Fee Structure

| Fee Type | Amount | Destination |
|----------|--------|-------------|
| **Escrow Creation** | 0.1 Pi | Coordinator treasury |
| **Dispute Filing** | 1.0 Pi (refundable if ruling favors filer) | Escrow hold |
| **Juror Reward** | 0.5 Pi per juror (3 jurors = 1.5 Pi) | From dispute fee |
| **Merchant Verification** | 5.0 Pi (one-time) | Burn address |
| **Loyalty Redemption** | 0 Pi (free) | N/A |

Fees are intentionally minimal to encourage adoption, especially for micro-commerce.

## Upgrade Strategy

All contracts implement Soroban's `__check_wasm_update` for safe upgrades:

1. **Coordinator** — Upgradeable by admin multi-sig (3-of-5)
2. **Escrow** — Upgradeable by coordinator reference
3. **Reputation** — Upgradeable by coordinator reference
4. **Dispute** — Upgradeable by coordinator reference
5. **Merchant Verification** — Upgradeable by coordinator reference
6. **Loyalty** — Upgradeable by coordinator reference

Upgrades require a 48-hour timelock before execution, allowing community review.
