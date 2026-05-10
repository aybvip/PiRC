# PiRC — Pi Decentralized Commerce & Trust Protocol (PiDCTP)

## Pi Requests for Comment

This repository contains PiRC proposals for the Pi Network ecosystem, focusing on building the **decentralized commerce and trust infrastructure** layer.

### Current Proposals

| PiRC | Title | Status | Description |
|------|-------|--------|-------------|
| **PiRC3** | PiDCTP — Commerce & Trust Protocol | Draft | Escrow, Reputation, Dispute Resolution, Merchant Verification, Loyalty |

### PiRC3: Pi Decentralized Commerce & Trust Protocol

PiRC3 introduces five interconnected modules that form a complete commerce trust layer for the Pi ecosystem:

1. **Escrow Payment System** — Multi-signature escrow contracts protecting both buyers and sellers
2. **Reputation Engine** — Privacy-preserving, portable reputation scores from verified transaction history
3. **Dispute Resolution Protocol** — Decentralized arbitration with VRF-selected jurors and commit-reveal voting
4. **Merchant Verification** — Lightweight KYB process establishing business legitimacy
5. **Loyalty & Reward Module** — Ecosystem incentives for consistent honest commerce

#### Relationship to Previous PiRCs

```
┌─────────────────────────────────┐
│     PiRC3: Commerce & Trust     │  ← Escrow, Reputation, Disputes, Verification, Loyalty
├─────────────────────────────────┤
│    PiRC2: Subscription API      │  ← Recurring Payments (integration supported)
├─────────────────────────────────┤
│    PiRC1: Token Design           │  ← Token Allocation & Economics
└─────────────────────────────────┘
```

#### Repository Structure

```
PiRC/
├── PiRC3/                          # PiRC3 Documentation
│   ├── ReadMe.md                   # Overview & Table of Contents
│   ├── 1-vision.md                 # Vision & Problem Statement
│   ├── 2-core-design.md            # Architecture & Module Interactions
│   ├── 3-escrow-system.md          # Escrow Payment System Specification
│   ├── 4-reputation-engine.md      # Reputation Score & Anti-Gaming
│   ├── 5-dispute-resolution.md     # Dispute Resolution Protocol
│   ├── 6-merchant-verification.md  # Merchant Verification & KYB
│   ├── 7-loyalty-rewards.md        # Loyalty Points & Tiers
│   ├── 8-data-types.md             # Shared Data Types
│   ├── 9-error-codes.md            # Error Code Reference
│   ├── 10-integration-pirc2.md     # PiRC2 Integration Guide
│   ├── 11-security-model.md        # Security & Threat Model
│   └── 12-implementation-guide.md  # Development & Deployment Guide
├── contracts/                      # Soroban Smart Contracts
│   ├── shared/                     # Shared types & events
│   ├── escrow/                     # Escrow module
│   ├── reputation/                 # Reputation module
│   ├── dispute/                    # Dispute resolution module
│   ├── merchant/                   # Merchant verification module
│   ├── loyalty/                    # Loyalty & rewards module
│   └── coordinator/               # Entry point & router
├── Cargo.toml                      # Workspace configuration
├── LICENSE                         # MIT License
└── ReadMe.md                       # This file
```

#### Quick Start

```bash
# Clone the repository
git clone https://github.com/YOUR_ORG/PiRC.git
cd PiRC

# Build all contracts
soroban contract build

# Run tests
cargo test

# Deploy to testnet
./scripts/deploy_testnet.sh
```

#### Technical Stack

- **Smart Contracts**: Rust + Soroban SDK (Stellar)
- **Token Standard**: Stellar Classic Asset (Pi)
- **Randomness**: Verifiable Random Function (VRF) for juror selection
- **Privacy** (future): Zero-Knowledge Proofs (ZK-SNARKs) for reputation verification
- **Events**: Standardized event emission for off-chain indexing

#### Community Feedback

Community feedback is an essential part of this process. Pioneers are encouraged to:
- Review the specification documents in `PiRC3/`
- Open GitHub Issues for specific concerns
- Submit Pull Requests with proposed changes
- Discuss with other community members

Pi will review and consider community input. As with any design process, not all suggestions will be adopted, but feedback will be evaluated to determine whether and what adjustments are appropriate.

#### License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
