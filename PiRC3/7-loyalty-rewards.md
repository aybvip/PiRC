# PiRC3 — Section 7: Loyalty & Reward Module

## Overview

The Loyalty & Reward Module incentivizes **consistent honest commerce behavior** by awarding points, tier upgrades, and fee discounts to Pioneers who actively participate in the ecosystem. This creates a positive feedback loop where integrity is economically rewarded.

## Design Principles

1. **Reward honesty, not volume** — Points are earned per successful transaction, not per Pi spent
2. **Progressive benefits** — Higher tiers unlock meaningful economic advantages
3. **Anti-gaming** — Sybil-resistant through reputation coupling
4. **Sustainable economics** — Rewards are funded by ecosystem fees, not inflation

## Loyalty Tiers

| Tier | Points Required | Fee Discount | Juror Weight | Other Benefits |
|------|----------------|-------------|--------------|----------------|
| **Starter** | 0 | 0% | 1.0x | Basic access |
| **Regular** | 100 | 10% | 1.0x | Priority support |
| **Trusted** | 500 | 25% | 1.2x | Early access to new features |
| **Elite** | 2,000 | 50% | 1.5x | Featured merchant badge, API access |
| **Legendary** | 10,000 | 75% | 2.0x | Governance voting power, exclusive events |

## Point Earning Rules

| Action | Points Earned | Frequency Limit |
|--------|--------------|-----------------|
| Complete escrow (buyer) | +3 | Per escrow |
| Complete escrow (seller) | +5 | Per escrow |
| Rate a merchant | +1 | Per rating |
| Serve as juror | +10 | Per dispute |
| Juror consensus bonus | +5 | Per dispute (if vote with majority) |
| Merchant verification (Basic) | +20 | One-time |
| Merchant verification (Standard) | +50 | One-time |
| Merchant verification (Premium) | +100 | One-time |
| Refer a new Pioneer who completes first escrow | +15 | Per referral |
| Maintain Gold+ reputation for 90 days | +25 | Quarterly |
| No disputes against you for 180 days | +30 | Semi-annually |

## Point Deduction Rules

| Action | Points Deducted |
|--------|----------------|
| Escrow expired (as seller) | -10 |
| Dispute ruling against you | -20 |
| Merchant verification revoked | -50 |
| Juror non-participation | -15 |
| Inactivity > 60 days | -5 per week |

## Data Structures

### LoyaltyProfile

```rust
struct LoyaltyProfile {
    /// Pioneer's Stellar address
    pioneer: Address,
    /// Current loyalty points
    points: u32,
    /// Current loyalty tier
    tier: LoyaltyTier,
    /// Lifetime points earned (never decreases, used for tier calculation)
    lifetime_points: u32,
    /// Points available for redemption
    redeemable_points: u32,
    /// Last point activity timestamp
    last_activity: u64,
    /// Referral code (hash)
    referral_code: BytesN<32>,
    /// Number of successful referrals
    referral_count: u32,
    /// Quarterly streak counter (consecutive quarters with activity)
    activity_streak: u32,
}
```

### LoyaltyTier

```rust
enum LoyaltyTier {
    Starter,    // 0 points
    Regular,    // 100 points
    Trusted,    // 500 points
    Elite,      // 2,000 points
    Legendary,  // 10,000 points
}
```

## Contract Interface

### `get_loyalty_profile`

Retrieves a Pioneer's full loyalty profile.

```rust
fn get_loyalty_profile(
    env: Env,
    pioneer: Address,
) -> LoyaltyProfile;
```

### `get_tier_benefits`

Retrieves the benefits for a given loyalty tier.

```rust
fn get_tier_benefits(
    env: Env,
    tier: LoyaltyTier,
) -> TierBenefits;
```

### `redeem_points`

Redeems loyalty points for a reward.

```rust
fn redeem_points(
    env: Env,
    pioneer: Address,
    reward: RewardType,
    amount: u32,
) -> void;
```

### `apply_fee_discount`

Calculates the fee discount for a Pioneer based on their loyalty tier. Called by the coordinator during escrow creation.

```rust
fn apply_fee_discount(
    env: Env,
    pioneer: Address,
    base_fee: i128,
) -> i128;  // Returns discounted fee
```

## Reward Types

| Reward | Points Cost | Description |
|--------|------------|-------------|
| **Fee Waiver** | 50 points | Waiver of next escrow creation fee |
| **Juror Priority** | 100 points | Priority selection for next 3 disputes |
| **Merchant Spotlight** | 200 points | Featured placement for 7 days |
| **Reputation Boost** | 500 points | +25 reputation points (one-time per quarter) |
| **Governance Vote** | 1,000 points | 1 vote in ecosystem governance proposals |

## Sustainability Model

Loyalty rewards are funded by the ecosystem's fee revenue:

| Revenue Source | Allocation to Loyalty Pool |
|----------------|---------------------------|
| Escrow creation fees | 40% |
| Dispute filing fees (non-refunded) | 30% |
| Merchant verification fees | 20% |
| Appeal fees (non-refunded) | 10% |

The loyalty pool has a **cap of 100,000 Pi** at any time. Excess revenue is burned, creating deflationary pressure on the Pi token supply.

## Anti-Gaming Measures

1. **Lifetime vs. Redeemable points** — Tier is calculated from lifetime points (which never decrease), but only redeemable points can be spent. This prevents "spend and re-earn" cycles.
2. **Referral limits** — Maximum 50 referral bonuses per Pioneer. Referred Pioneer must complete at least 1 escrow before referrer earns bonus.
3. **Streak decay** — Activity streak resets if no escrow activity for 60 days.
4. **Reputation coupling** — Loyalty tier cannot exceed reputation tier. A Bronze-reputation Pioneer cannot reach Elite loyalty tier regardless of points.
