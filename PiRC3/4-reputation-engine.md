# PiRC3 — Section 4: Reputation Engine

## Overview

The Reputation Engine provides **portable, verifiable trust scores** derived from on-chain transaction history. Unlike centralized rating systems, PiDCTP reputation is:

- **Earned, not self-reported** — Only verified escrow completions and dispute outcomes contribute
- **Tamper-proof** — Stored on-chain with cryptographic integrity
- **Privacy-preserving** — Scores are visible without exposing transaction details
- **Portable** — A Pioneer's reputation follows them across all Pi ecosystem apps

## Reputation Score Design

### Score Composition

A Pioneer's reputation score (`rep_score`) is a composite value ranging from **0 to 1000**, calculated from multiple weighted factors:

```
rep_score = w1 × escrow_factor + w2 × dispute_factor + w3 × merchant_factor + w4 × tenure_factor
```

| Factor | Weight | Range | Description |
|--------|--------|-------|-------------|
| `escrow_factor` | 0.40 | 0–400 | Successful escrow completions vs. total |
| `dispute_factor` | 0.25 | 0–250 | Favorable dispute outcomes vs. total disputes |
| `merchant_factor` | 0.20 | 0–200 | Verified merchant status + customer ratings |
| `tenure_factor` | 0.15 | 0–150 | Account age and activity consistency |

### Score Tiers

| Tier | Score Range | Badge | Benefits |
|------|-------------|-------|----------|
| **Bronze** | 0–199 | 🥉 | Basic escrow access |
| **Silver** | 200–449 | 🥈 | Reduced escrow fees (50%) |
| **Gold** | 450–699 | 🥇 | Reduced escrow fees (75%), juror eligibility |
| **Platinum** | 700–899 | 💎 | Zero escrow fees, priority juror selection, merchant badge |
| **Diamond** | 900–1000 | 👑 | All Platinum benefits + dispute ruling weight bonus |

### Score Calculation Algorithm

```rust
fn calculate_escrow_factor(
    completed: u32,
    expired: u32,
    disputed_against: u32,
    total: u32,
) -> u32 {
    if total == 0 { return 200; } // Neutral starting point
    let success_rate = (completed as f64) / (total as f64);
    let penalty = (expired * 2 + disputed_against * 5) as f64;
    let raw = (success_rate * 400.0) - penalty;
    raw.max(0.0).min(400.0) as u32
}

fn calculate_dispute_factor(
    rulings_favor: u32,
    rulings_against: u32,
    total_disputes: u32,
) -> u32 {
    if total_disputes == 0 { return 125; } // Neutral
    let favor_rate = (rulings_favor as f64) / (total_disputes as f64);
    (favor_rate * 250.0) as u32
}

fn calculate_tenure_factor(
    account_age_days: u32,
    active_days: u32,
) -> u32 {
    let age_score = (account_age_days as f64 / 365.0).min(1.0) * 75.0;
    let activity_score = (active_days as f64 / 90.0).min(1.0) * 75.0;
    (age_score + activity_score) as u32
}
```

## Data Structures

### ReputationProfile

```rust
struct ReputationProfile {
    /// Pioneer's Stellar address
    pioneer: Address,
    /// Current composite reputation score (0–1000)
    score: u32,
    /// Current tier
    tier: ReputationTier,
    /// Total escrow transactions participated in
    total_escrows: u32,
    /// Successfully completed escrows
    completed_escrows: u32,
    /// Escrows that expired (seller failed to deliver)
    expired_escrows: u32,
    /// Disputes filed by this Pioneer (as buyer)
    disputes_as_buyer: u32,
    /// Disputes filed against this Pioneer (as seller)
    disputes_as_seller: u32,
    /// Dispute rulings in this Pioneer's favor
    rulings_in_favor: u32,
    /// Dispute rulings against this Pioneer
    rulings_against: u32,
    /// Whether this Pioneer is a verified merchant
    is_verified_merchant: bool,
    /// Account creation timestamp
    created_at: u64,
    /// Last activity timestamp
    last_active: u64,
    /// Merkle root of transaction history (for ZK proofs in future)
    history_root: BytesN<32>,
    /// Score recalculation nonce (incremented on each update)
    score_nonce: u32,
}
```

### ReputationTier

```rust
enum ReputationTier {
    Bronze,   // 0–199
    Silver,   // 200–449
    Gold,     // 450–699
    Platinum, // 700–899
    Diamond,  // 900–1000
}
```

## Contract Interface

### `get_reputation`

Retrieves a Pioneer's reputation profile.

```rust
fn get_reputation(
    env: Env,
    pioneer: Address,
) -> ReputationProfile;
```

### `get_score`

Retrieves only the numeric score (gas-efficient for checks).

```rust
fn get_score(
    env: Env,
    pioneer: Address,
) -> u32;
```

### `get_tier`

Retrieves only the current tier.

```rust
fn get_tier(
    env: Env,
    pioneer: Address,
) -> ReputationTier;
```

### `verify_reputation_threshold`

Checks if a Pioneer meets a minimum score threshold (used by other modules).

```rust
fn verify_reputation_threshold(
    env: Env,
    pioneer: Address,
    minimum_score: u32,
) -> bool;
```

## Score Update Triggers

Reputation scores are **automatically updated** by the coordinator when specific events occur:

| Event | Score Impact | Direction |
|-------|-------------|-----------|
| Escrow completed (as seller) | +5 to escrow_factor | ↑ |
| Escrow completed (as buyer) | +3 to escrow_factor | ↑ |
| Escrow expired (as seller) | -15 to escrow_factor | ↓ |
| Dispute ruling in favor | +10 to dispute_factor | ↑ |
| Dispute ruling against | -20 to dispute_factor | ↓ |
| Merchant verification approved | +50 to merchant_factor | ↑ |
| Merchant verification revoked | -50 to merchant_factor | ↓ |
| Inactivity > 30 days | -1 per day to tenure_factor | ↓ |

## Decay Mechanism

To ensure reputation reflects **recent** behavior, a time-based decay is applied:

- **No decay** for accounts active within the last 30 days
- **1% decay per week** of inactivity after 30 days
- **Minimum floor** of 50 points (never decays below Bronze)
- **Reactivation bonus**: +10 points on first transaction after inactivity

This prevents "reputation squatting" where a Pioneer builds a high score then abandons the account.

## Privacy-Preserving Verification (Future)

The `history_root` field stores a Merkle root of the Pioneer's transaction history. In a future upgrade, this enables:

1. **Zero-Knowledge Proof of Score**: A Pioneer can prove their score exceeds a threshold without revealing the exact score or transaction history
2. **Selective Disclosure**: A Pioneer can prove they completed N successful escrows without revealing counterparty identities
3. **Cross-Ecosystem Portability**: Other Web3 platforms can verify Pi reputation without accessing Pi-specific data

This is achieved through a future integration of ZK-SNARKs (e.g., Groth16 or PLONK) with the Merkle root as the public input.

## Anti-Gaming Measures

### Sybil Resistance
- Each Stellar address maps to one Pi account (enforced at the Pi Network identity layer)
- New accounts start at score 200 (Silver floor) — not zero — to prevent "fresh account" exploitation
- Rapid score gains are rate-limited: maximum +50 points per 24-hour period

### Collusion Resistance
- Jurors in dispute resolution cannot see the reputation scores of parties before ruling
- Escrow completions with the same counterparty are weighted less: `weight = 1 / (1 + repeat_count * 0.2)`
- Merchant ratings from the same buyer are capped at 3 per 30-day period

### Score Manipulation Resistance
- All score updates must originate from the coordinator contract (verified by signature)
- Score nonce prevents replay attacks
- Emergency pause capability (3-of-5 admin multi-sig) if manipulation is detected
