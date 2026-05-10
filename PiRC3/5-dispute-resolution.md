# PiRC3 — Section 5: Dispute Resolution Protocol

## Overview

The Dispute Resolution Protocol provides **decentralized, community-driven arbitration** for commerce disputes. When a buyer and seller cannot agree, randomly selected jurors review evidence and render a ruling — with economic incentives ensuring honest judgments.

## Design Principles

1. **Decentralized adjudication** — No central authority decides outcomes
2. **Economic alignment** — Jurors are incentivized to rule honestly; dishonest rulings are penalized
3. **Fair selection** — Verifiable Random Function (VRF) ensures unbiased juror selection
4. **Efficient process** — Disputes resolve within 5–7 days maximum
5. **Appeal mechanism** — A first ruling can be appealed once with a larger jury

## Dispute Lifecycle

```
┌───────────┐     ┌───────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  FILED    │────►│  EVIDENCE │────►│  VOTING  │────►│  RULING  │────►│ EXECUTED │
└───────────┘     └───────────┘     └──────────┘     └──────────┘     └──────────┘
                                          │
                                          ▼
                                    ┌──────────┐     ┌──────────┐     ┌──────────┐
                                    │  APPEALED│────►│  VOTING  │────►│  FINAL   │
                                    └──────────┘     └──────────┘     └──────────┘
```

| Phase | Duration | Description |
|-------|----------|-------------|
| **Filed** | Instant | Dispute opened, escrow frozen |
| **Evidence** | 72 hours | Both parties submit evidence |
| **Voting** | 48 hours | Jurors review and vote |
| **Ruling** | Instant | Majority vote determines outcome |
| **Appealed** | 24 hours window | Either party may appeal once |
| **Final** | Instant | Appeal jury ruling is binding |

## Data Structures

### DisputeCase

```rust
struct DisputeCase {
    /// Unique dispute identifier
    dispute_id: u64,
    /// Associated escrow ID
    escrow_id: u64,
    /// Party filing the dispute
    filer: Address,
    /// Party being disputed
    respondent: Address,
    /// Dispute category
    category: DisputeCategory,
    /// Current phase
    phase: DisputePhase,
    /// Selected jurors (3 for initial, 5 for appeal)
    jurors: Vec<Address>,
    /// Juror votes
    votes: Vec<JurorVote>,
    /// Evidence submitted by filer (IPFS CIDs)
    filer_evidence: Vec<BytesN<32>>,
    /// Evidence submitted by respondent (IPFS CIDs)
    respondent_evidence: Vec<BytesN<32>>,
    /// Filing timestamp
    filed_at: u64,
    /// Evidence deadline
    evidence_deadline: u64,
    /// Voting deadline
    voting_deadline: u64,
    /// Ruling result
    ruling: Option<DisputeRuling>,
    /// Whether this dispute has been appealed
    is_appealed: bool,
    /// Appeal filing fee
    appeal_fee: i128,
}
```

### DisputeCategory

```rust
enum DisputeCategory {
    /// Goods not delivered
    NonDelivery,
    /// Goods delivered but not as described
    NotAsDescribed,
    /// Goods delivered but damaged/defective
    DamagedDefective,
    /// Seller confirmed delivery but buyer claims non-receipt
    DeliveryDispute,
    /// Subscription service not provided (PiRC2 linked)
    ServiceNotProvided,
    /// Unauthorized charge or billing dispute
    UnauthorizedCharge,
    /// Other (free-text description off-chain)
    Other,
}
```

### DisputeRuling

```rust
enum DisputeRuling {
    /// Full refund to buyer, seller penalized
    FullRefund,
    /// Partial refund to buyer (percentage specified)
    PartialRefund { buyer_percentage: u32 },
    /// Funds released to seller, buyer claim rejected
    SellerFavored,
    /// Split evenly between parties
    Split,
    /// Dismissed (insufficient evidence or frivolous)
    Dismissed,
}
```

### JurorVote

```rust
struct JurorVote {
    /// Juror's Stellar address
    juror: Address,
    /// The juror's ruling vote
    vote: DisputeRuling,
    /// Confidence level (1–5, used for incentive calculation)
    confidence: u8,
    /// Timestamp of vote submission
    voted_at: u64,
    /// Justification hash (IPFS CID of written reasoning)
    justification_hash: BytesN<32>,
}
```

## Contract Interface

### `open_dispute`

Opens a dispute against an escrow transaction.

```rust
fn open_dispute(
    env: Env,
    filer: Address,
    escrow_id: u64,
    category: DisputeCategory,
    initial_evidence: BytesN<32>,  // IPFS CID
) -> u64;  // Returns dispute_id
```

**Preconditions:**
- Escrow is in `Funded` or `Delivered` state
- Filer is either the buyer or seller of the escrow
- Filer deposits 1.0 Pi dispute fee (refundable if ruling favors filer)
- No existing open dispute for this escrow

**Postconditions:**
- Dispute created in `Filed` phase
- Escrow transitions to `Disputed` state (funds frozen)
- Juror selection triggered
- `DISPUTE_OPENED` event emitted

### `submit_evidence`

Submits evidence for a dispute case.

```rust
fn submit_evidence(
    env: Env,
    party: Address,
    dispute_id: u64,
    evidence_hash: BytesN<32>,  // IPFS CID
) -> void;
```

**Preconditions:**
- Dispute is in `Evidence` phase
- Caller is the filer or respondent
- Evidence deadline has not passed
- Maximum 5 evidence items per party

### `cast_vote`

Juror casts their vote on a dispute.

```rust
fn cast_vote(
    env: Env,
    juror: Address,
    dispute_id: u64,
    vote: DisputeRuling,
    confidence: u8,
    justification_hash: BytesN<32>,
) -> void;
```

**Preconditions:**
- Dispute is in `Voting` phase
- Caller is a selected juror for this dispute
- Juror has not already voted
- `confidence` is between 1 and 5

### `execute_ruling`

Executes the ruling after voting is complete.

```rust
fn execute_ruling(
    env: Env,
    caller: Address,
    dispute_id: u64,
) -> DisputeRuling;
```

**Preconditions:**
- All jurors have voted OR voting deadline has passed
- Dispute is in `Ruling` phase

**Postconditions:**
- Escrow funds distributed per ruling
- Reputation updates applied to both parties
- Juror rewards distributed
- `DISPUTE_RESOLVED` event emitted

### `appeal_ruling`

Appeals a first-round ruling.

```rust
fn appeal_ruling(
    env: Env,
    appellant: Address,
    dispute_id: u64,
    appeal_fee: i128,  // 2.0 Pi required
) -> void;
```

**Preconditions:**
- Dispute ruling has been issued (not yet final)
- 24-hour appeal window has not expired
- Appellant deposits 2.0 Pi appeal fee
- No prior appeal for this dispute

**Postconditions:**
- 5 new jurors selected (none from first round)
- New evidence/voting cycle begins
- `DISPUTE_APPEALED` event emitted

## Juror Selection Algorithm

Jurors are selected using a **Verifiable Random Function (VRF)** to ensure fairness and unpredictability.

### Eligibility Criteria

| Criterion | Requirement |
|-----------|-------------|
| Reputation Tier | Gold or above (score ≥ 450) |
| Account Age | ≥ 90 days |
| Recent Activity | At least 1 escrow in last 30 days |
| No Conflict | Not involved in the dispute as party |
| No Recent Duty | Not selected as juror in last 7 days |
| Staked | Must have staked 10 Pi as juror bond |

### Selection Process

```
1. Filter eligible jurors (meeting all criteria above)
2. Weight each eligible juror by reputation score:
   weight = reputation_score^2 (quadratic weighting favors higher-rep jurors)
3. Use VRF to generate random seed
4. Select 3 jurors using weighted random sampling without replacement
5. Notify selected jurors (on-chain event)
6. Jurors must accept within 24 hours or replacement is selected
```

### For Appeal Rounds

- 5 jurors selected (same algorithm)
- None from the first-round jury
- Minimum reputation requirement: Platinum (score ≥ 700)

## Juror Incentive Model

### Reward Structure

| Outcome | Juror Reward | Source |
|---------|-------------|--------|
| **Consensus reached** | 0.5 Pi per juror | Dispute filing fee |
| **Majority reached** | 0.4 Pi for majority jurors, 0.1 Pi for minority | Dispute filing fee |
| **No consensus** | 0.2 Pi per juror | Dispute filing fee |

### Honesty Incentive (Schelling Point)

Jurors who vote with the majority receive a **coherence bonus** of +0.1 Pi. This leverages the Schelling Point mechanism: when jurors independently evaluate the same evidence, honest evaluation converges, making the majority vote a strong signal of truth.

### Penalty for Non-Participation

- Jurors who accept but fail to vote lose 0.5 Pi from their staked bond
- After 2 non-participation events, juror is suspended for 30 days
- After 3 suspensions, juror is permanently removed from the jury pool

## Anti-Collusion Measures

1. **Juror identity hidden until ruling** — Parties cannot see which jurors are selected until after the ruling
2. **Vote encryption** — Votes are submitted encrypted and revealed simultaneously after voting deadline (commit-reveal scheme)
3. **Correlation detection** — If a juror consistently votes with the same party across multiple disputes, their weight is reduced
4. **Appeal mechanism** — Collusion in the first round can be overturned by a different jury in the appeal

## Dispute Fee Economics

| Fee | Amount | Refundable? |
|-----|--------|-------------|
| Filing fee | 1.0 Pi | Yes (if ruling favors filer) |
| Appeal fee | 2.0 Pi | Yes (if appeal ruling favors appellant) |
| Juror bond | 10.0 Pi (staked) | Yes (returned if no penalties) |

Total fees for a standard dispute: **1.0 Pi** (filer) + **1.5 Pi** (juror rewards from treasury surplus)
Total fees for an appealed dispute: **3.0 Pi** (filer + appellant) + **2.5 Pi** (juror rewards)
