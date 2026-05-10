# PiRC3 — Section 6: Merchant Verification

## Overview

The Merchant Verification module provides a **lightweight KYB (Know Your Business)** process that establishes merchant legitimacy within the Pi ecosystem. Verified merchants receive a trust badge, enhanced visibility, and access to loyalty program features.

## Why Merchant Verification?

In a decentralized commerce ecosystem, Pioneers need a way to distinguish legitimate businesses from anonymous sellers. Merchant verification:

- **Reduces fraud risk** — Verified merchants have passed identity and business checks
- **Enables higher-value commerce** — Buyers are more willing to transact with verified merchants
- **Supports regulatory compliance** — Basic business information supports legal commerce frameworks
- **Builds ecosystem trust** — A visible merchant directory increases overall platform credibility

## Verification Levels

| Level | Requirements | Benefits | Fee |
|-------|-------------|----------|-----|
| **Basic** | Pi account ≥ 90 days, reputation ≥ 300, business description | Merchant badge, marketplace listing | 2.0 Pi |
| **Standard** | Basic + business registration document, 10+ completed escrows | Enhanced listing, priority search | 5.0 Pi |
| **Premium** | Standard + tax ID, physical address verification, 50+ completed escrows | Featured placement, loyalty tools, API access | 10.0 Pi |

## Data Structures

### MerchantProfile

```rust
struct MerchantProfile {
    /// Merchant's Stellar address
    merchant: Address,
    /// Verification level
    level: VerificationLevel,
    /// Business name (hashed on-chain, plaintext off-chain)
    business_name_hash: BytesN<32>,
    /// Business category
    category: MerchantCategory,
    /// Verification status
    status: VerificationStatus,
    /// Jurisdiction/country code
    jurisdiction: BytesN<2>,  // ISO 3166-1 alpha-2
    /// Total sales volume (in Pi stroops)
    total_volume: i128,
    /// Total completed orders
    total_orders: u32,
    /// Average customer rating (0–500, mapped to 0.0–5.0)
    avg_rating: u32,
    /// Verification timestamp
    verified_at: Option<u64>,
    /// Expiration timestamp (annual renewal)
    expires_at: Option<u64>,
    /// Number of verified physical locations
    location_count: u32,
    /// Off-chain metadata URI (IPFS CID)
    metadata_uri: BytesN<32>,
}
```

### VerificationLevel

```rust
enum VerificationLevel {
    /// Not verified
    None,
    /// Basic verification
    Basic,
    /// Standard verification with business documents
    Standard,
    /// Premium verification with full KYB
    Premium,
}
```

### VerificationStatus

```rust
enum VerificationStatus {
    /// Not applied
    NotApplied,
    /// Application submitted, pending review
    Pending,
    /// Under review by verification agents
    UnderReview,
    /// Additional information requested
    InfoRequested,
    /// Approved and active
    Approved,
    /// Temporarily suspended (under investigation)
    Suspended,
    /// Permanently revoked (serious violation)
    Revoked,
    /// Expired (needs renewal)
    Expired,
}
```

### MerchantCategory

```rust
enum MerchantCategory {
    DigitalGoods,
    PhysicalGoods,
    Services,
    FoodAndBeverage,
    Entertainment,
    Education,
    HealthAndWellness,
    ProfessionalServices,
    Retail,
    Other,
}
```

## Contract Interface

### `apply_verification`

Submits a merchant verification application.

```rust
fn apply_verification(
    env: Env,
    merchant: Address,
    level: VerificationLevel,
    business_name_hash: BytesN<32>,
    category: MerchantCategory,
    jurisdiction: BytesN<2>,
    metadata_uri: BytesN<32>,  // IPFS CID with documents
) -> void;
```

**Preconditions:**
- Caller is not already an approved merchant at the requested level or higher
- Reputation score meets minimum for requested level
- Fee deposited (2.0 / 5.0 / 10.0 Pi based on level)

### `approve_verification`

Approves a merchant's verification application (called by verification agent).

```rust
fn approve_verification(
    env: Env,
    agent: Address,
    merchant: Address,
    level: VerificationLevel,
) -> void;
```

**Preconditions:**
- Caller is an authorized verification agent
- Application is in `UnderReview` status
- All required documents verified off-chain

### `suspend_merchant`

Temporarily suspends a verified merchant.

```rust
fn suspend_merchant(
    env: Env,
    agent: Address,
    merchant: Address,
    reason_hash: BytesN<32>,  // IPFS CID with suspension reason
) -> void;
```

### `revoke_verification`

Permanently revokes a merchant's verification.

```rust
fn revoke_verification(
    env: Env,
    agent: Address,
    merchant: Address,
    reason_hash: BytesN<32>,
) -> void;
```

**Postconditions:**
- Merchant status set to `Revoked`
- Merchant badge removed
- Reputation score reduced by 50 points
- All active escrows flagged for buyer notification

### `renew_verification`

Renews an expiring or expired verification.

```rust
fn renew_verification(
    env: Env,
    merchant: Address,
    metadata_uri: BytesN<32>,  // Updated documents
) -> void;
```

**Preconditions:**
- Current verification is within 30 days of expiration OR expired within last 90 days
- Renewal fee deposited (50% of original fee)

### `rate_merchant`

Submits a customer rating for a merchant after a completed escrow.

```rust
fn rate_merchant(
    env: Env,
    buyer: Address,
    merchant: Address,
    escrow_id: u64,
    rating: u32,  // 1–500 (mapped to 0.5–5.0 stars)
    review_hash: BytesN<32>,  // IPFS CID of written review
) -> void;
```

**Preconditions:**
- Escrow is in `Completed` state
- Caller is the buyer of the escrow
- Buyer has not already rated this escrow
- Rating is between 50 and 500 (0.5 to 5.0 stars)

**Postconditions:**
- Merchant's `avg_rating` updated (exponential moving average)
- Rating stored off-chain, hash stored on-chain
- Buyer receives +1 loyalty point

## Verification Agent System

Verification agents are trusted community members who review merchant applications.

### Agent Requirements

| Requirement | Value |
|-------------|-------|
| Reputation Score | ≥ 700 (Platinum) |
| Account Age | ≥ 180 days |
| Completed Escrows | ≥ 100 |
| No Dispute Rulings Against | 0 in last 90 days |
| Stake | 50 Pi (agent bond) |

### Agent Selection

- Agents are approved by the Pi Foundation initially
- Future: Agents elected by community vote (Diamond-tier Pioneers)
- Agents rotate review assignments to prevent corruption
- Each application reviewed by 2 independent agents (both must approve)

### Agent Compensation

- 1.0 Pi per Basic review
- 2.0 Pi per Standard review
- 3.0 Pi per Premium review
- Paid from verification fees

## Annual Renewal

All verification levels require annual renewal:

| Level | Renewal Fee | Renewal Requirements |
|-------|------------|----------------------|
| Basic | 1.0 Pi | Reputation ≥ 300, ≥ 5 completed escrows in last year |
| Standard | 2.5 Pi | Reputation ≥ 400, ≥ 10 completed escrows in last year |
| Premium | 5.0 Pi | Reputation ≥ 500, ≥ 25 completed escrows in last year |

Failure to renew within 90 days of expiration results in `Expired` status. Re-verification requires a new full application.
