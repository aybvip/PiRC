# PiRC3 — Section 8: Data Types

## Overview

This section defines all shared data types used across PiDCTP modules. These types ensure consistency and interoperability between the Coordinator, Escrow, Reputation, Dispute Resolution, Merchant Verification, and Loyalty contracts.

## Primitive Types

| Type | Soroban Type | Description |
|------|-------------|-------------|
| `PiAmount` | `i128` | Amount in stroops (1 Pi = 10,000,000 stroops) |
| `Timestamp` | `u64` | Unix timestamp in seconds |
| `Duration` | `u64` | Time duration in seconds |
| `EscrowId` | `u64` | Unique escrow identifier |
| `DisputeId` | `u64` | Unique dispute identifier |
| `Score` | `u32` | Reputation score (0–1000) |
| `Rating` | `u32` | Merchant rating (50–500, mapped to 0.5–5.0 stars) |
| `Points` | `u32` | Loyalty points |
| `Percentage` | `u32` | Percentage value (0–10000, where 10000 = 100.00%) |
| `Confidence` | `u8` | Juror confidence level (1–5) |
| `CountryCode` | `BytesN<2>` | ISO 3166-1 alpha-2 country code |
| `Hash` | `BytesN<32>` | SHA-256 hash or IPFS CID |
| `Nonce` | `u32` | Monotonic counter for replay protection |

## Enumerations

### EscrowState

```rust
enum EscrowState {
    Created,
    Funded,
    Delivered,
    Completed,
    Disputed,
    Resolved,
    Expired,
    Cancelled,
}
```

### DisputePhase

```rust
enum DisputePhase {
    Filed,
    Evidence,
    Voting,
    Ruling,
    Appealed,
    Final,
}
```

### DisputeCategory

```rust
enum DisputeCategory {
    NonDelivery,
    NotAsDescribed,
    DamagedDefective,
    DeliveryDispute,
    ServiceNotProvided,
    UnauthorizedCharge,
    Other,
}
```

### DisputeRuling

```rust
enum DisputeRuling {
    FullRefund,
    PartialRefund { buyer_percentage: u32 },
    SellerFavored,
    Split,
    Dismissed,
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

### VerificationLevel

```rust
enum VerificationLevel {
    None,
    Basic,
    Standard,
    Premium,
}
```

### VerificationStatus

```rust
enum VerificationStatus {
    NotApplied,
    Pending,
    UnderReview,
    InfoRequested,
    Approved,
    Suspended,
    Revoked,
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

### RewardType

```rust
enum RewardType {
    FeeWaiver,
    JurorPriority,
    MerchantSpotlight,
    ReputationBoost,
    GovernanceVote,
}
```

## Struct Types

### ModuleAddresses

```rust
struct ModuleAddresses {
    escrow: Address,
    reputation: Address,
    dispute: Address,
    merchant_verification: Address,
    loyalty: Address,
}
```

### EscrowAccount

```rust
struct EscrowAccount {
    escrow_id: EscrowId,
    buyer: Address,
    seller: Address,
    amount: PiAmount,
    token: Address,
    state: EscrowState,
    created_at: Timestamp,
    delivery_deadline: Timestamp,
    confirmation_deadline: Timestamp,
    auto_release_timeout: Duration,
    subscription_id: Option<u64>,
    order_metadata: Hash,
}
```

### ReputationProfile

```rust
struct ReputationProfile {
    pioneer: Address,
    score: Score,
    tier: ReputationTier,
    total_escrows: u32,
    completed_escrows: u32,
    expired_escrows: u32,
    disputes_as_buyer: u32,
    disputes_as_seller: u32,
    rulings_in_favor: u32,
    rulings_against: u32,
    is_verified_merchant: bool,
    created_at: Timestamp,
    last_active: Timestamp,
    history_root: Hash,
    score_nonce: Nonce,
}
```

### DisputeCase

```rust
struct DisputeCase {
    dispute_id: DisputeId,
    escrow_id: EscrowId,
    filer: Address,
    respondent: Address,
    category: DisputeCategory,
    phase: DisputePhase,
    jurors: Vec<Address>,
    votes: Vec<JurorVote>,
    filer_evidence: Vec<Hash>,
    respondent_evidence: Vec<Hash>,
    filed_at: Timestamp,
    evidence_deadline: Timestamp,
    voting_deadline: Timestamp,
    ruling: Option<DisputeRuling>,
    is_appealed: bool,
    appeal_fee: PiAmount,
}
```

### JurorVote

```rust
struct JurorVote {
    juror: Address,
    vote: DisputeRuling,
    confidence: Confidence,
    voted_at: Timestamp,
    justification_hash: Hash,
}
```

### MerchantProfile

```rust
struct MerchantProfile {
    merchant: Address,
    level: VerificationLevel,
    business_name_hash: Hash,
    category: MerchantCategory,
    status: VerificationStatus,
    jurisdiction: CountryCode,
    total_volume: PiAmount,
    total_orders: u32,
    avg_rating: Rating,
    verified_at: Option<Timestamp>,
    expires_at: Option<Timestamp>,
    location_count: u32,
    metadata_uri: Hash,
}
```

### LoyaltyProfile

```rust
struct LoyaltyProfile {
    pioneer: Address,
    points: Points,
    tier: LoyaltyTier,
    lifetime_points: Points,
    redeemable_points: Points,
    last_activity: Timestamp,
    referral_code: Hash,
    referral_count: u32,
    activity_streak: u32,
}
```

### TierBenefits

```rust
struct TierBenefits {
    tier: LoyaltyTier,
    fee_discount: Percentage,
    juror_weight: u32,  // Multiplier (100 = 1.0x)
    features: Vec<String>,
}
```

## Event Types

All events follow a standardized format for off-chain indexing:

```rust
struct PiDCTPEvent {
    event_type: Symbol,
    timestamp: Timestamp,
    data: Map<Symbol, Val>,  // Key-value pairs specific to event type
}
```

### Standard Events

| Event | Key Fields |
|-------|-----------|
| `ESCROW_CREATED` | escrow_id, buyer, seller, amount |
| `ESCROW_FUNDED` | escrow_id, amount |
| `ESCROW_DELIVERED` | escrow_id, seller |
| `ESCROW_COMPLETED` | escrow_id, buyer, seller |
| `ESCROW_CANCELLED` | escrow_id, initiator |
| `ESCROW_EXPIRED` | escrow_id |
| `DISPUTE_OPENED` | dispute_id, escrow_id, filer |
| `DISPUTE_EVIDENCE` | dispute_id, party |
| `DISPUTE_VOTE` | dispute_id, juror |
| `DISPUTE_RESOLVED` | dispute_id, ruling |
| `DISPUTE_APPEALED` | dispute_id, appellant |
| `REPUTATION_UPDATED` | pioneer, old_score, new_score |
| `MERCHANT_APPLIED` | merchant, level |
| `MERCHANT_APPROVED` | merchant, level |
| `MERCHANT_REVOKED` | merchant |
| `LOYALTY_POINTS_EARNED` | pioneer, points, action |
| `LOYALTY_POINTS_REDEEMED` | pioneer, points, reward |
| `LOYALTY_TIER_CHANGED` | pioneer, old_tier, new_tier |
