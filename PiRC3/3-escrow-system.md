# PiRC3 — Section 3: Escrow Payment System

## Overview

The Escrow System is the foundational protection mechanism of PiDCTP. It ensures that **no payment is released until both parties confirm transaction fulfillment**, eliminating the most common vectors for commerce fraud.

## Design Principles

1. **Funds are locked, not transferred** — Payment sits in escrow until explicit release
2. **Both parties must act** — Buyer confirms receipt; seller confirms delivery
3. **Timeout protection** — Auto-release or auto-refund if parties become unresponsive
4. **Dispute integration** — Funds are frozen if a dispute is opened
5. **Minimal gas** — Escrow operations are optimized for Soroban's fee model

## Escrow Lifecycle

```
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌──────────┐    ┌───────────┐
│ CREATED │───►│ FUNDED  │───►│ LOCKED   │───►│ RELEASE  │───►│ COMPLETED │
└─────────┘    └─────────┘    └──────────┘    └──────────┘    └───────────┘
     │              │              │
     │              │              ▼
     │              │         ┌──────────┐    ┌──────────┐
     │              │         │ DISPUTED │───►│ RESOLVED │
     │              │         └──────────┘    └──────────┘
     │              │
     ▼              ▼
┌──────────┐  ┌──────────┐
│CANCELLED │  │ EXPIRED  │
└──────────┘  └──────────┘
```

## Data Structures

### EscrowAccount

```rust
struct EscrowAccount {
    /// Unique escrow identifier
    escrow_id: u64,
    /// Buyer's Stellar address
    buyer: Address,
    /// Seller's Stellar address
    seller: Address,
    /// Amount in Pi (stroop units, 1 Pi = 10,000,000 stroops)
    amount: i128,
    /// Token contract address (Pi token)
    token: Address,
    /// Current state of the escrow
    state: EscrowState,
    /// Timestamp of creation
    created_at: u64,
    /// Deadline for seller delivery confirmation (unix timestamp)
    delivery_deadline: u64,
    /// Deadline for buyer confirmation after delivery (unix timestamp)
    confirmation_deadline: u64,
    /// Auto-release timeout after delivery (seconds, default: 7 days)
    auto_release_timeout: u64,
    /// Optional PiRC2 subscription ID if linked
    subscription_id: Option<u64>,
    /// Metadata hash (off-chain order details, IPFS CID)
    order_metadata: BytesN<32>,
}
```

### EscrowState

```rust
enum EscrowState {
    /// Escrow created, awaiting buyer funding
    Created,
    /// Buyer has deposited funds, awaiting seller delivery
    Funded,
    /// Seller confirmed delivery, awaiting buyer confirmation
    Delivered,
    /// Buyer confirmed, funds released to seller
    Completed,
    /// Dispute opened, funds frozen
    Disputed,
    /// Dispute resolved, funds distributed per ruling
    Resolved,
    /// Delivery deadline passed without seller action
    Expired,
    /// Cancelled before funding
    Cancelled,
}
```

## Contract Interface

### `create_escrow`

Creates a new escrow account. The buyer specifies the seller, amount, and deadlines.

```rust
fn create_escrow(
    env: Env,
    buyer: Address,
    seller: Address,
    amount: i128,
    token: Address,
    delivery_deadline: u64,
    auto_release_timeout: u64,
    order_metadata: BytesN<32>,
) -> u64;  // Returns escrow_id
```

**Preconditions:**
- `buyer != seller`
- `amount > 0`
- `delivery_deadline > current_time`
- `auto_release_timeout >= 86400` (minimum 1 day)

**Postconditions:**
- Escrow created in `Created` state
- `escrow_id` emitted in `ESCROW_CREATED` event

### `fund_escrow`

Buyer deposits the payment amount into escrow.

```rust
fn fund_escrow(
    env: Env,
    buyer: Address,
    escrow_id: u64,
) -> void;
```

**Preconditions:**
- Escrow is in `Created` state
- Caller is the buyer
- Buyer has sufficient Pi token allowance for the coordinator

**Postconditions:**
- Escrow transitions to `Funded` state
- Pi tokens transferred from buyer to escrow contract
- `ESCROW_FUNDED` event emitted
- Seller notified via event

### `confirm_delivery`

Seller confirms that goods/services have been delivered.

```rust
fn confirm_delivery(
    env: Env,
    seller: Address,
    escrow_id: u64,
) -> void;
```

**Preconditions:**
- Escrow is in `Funded` state
- Caller is the seller
- Current time <= delivery_deadline

**Postconditions:**
- Escrow transitions to `Delivered` state
- `confirmation_deadline` set to `current_time + auto_release_timeout`
- `ESCROW_DELIVERED` event emitted

### `confirm_receipt`

Buyer confirms receipt of goods/services, triggering fund release.

```rust
fn confirm_receipt(
    env: Env,
    buyer: Address,
    escrow_id: u64,
) -> void;
```

**Preconditions:**
- Escrow is in `Delivered` state
- Caller is the buyer

**Postconditions:**
- Escrow transitions to `Completed` state
- Pi tokens released to seller (minus coordinator fee)
- Reputation updates triggered for both parties
- Loyalty points awarded to both parties
- `ESCROW_COMPLETED` event emitted

### `auto_release`

Automatically releases funds to seller if buyer does not confirm within the timeout period.

```rust
fn auto_release(
    env: Env,
    caller: Address,
    escrow_id: u64,
) -> void;
```

**Preconditions:**
- Escrow is in `Delivered` state
- Current time > confirmation_deadline
- Any caller may invoke (incentivized by small reward)

**Postconditions:**
- Same as `confirm_receipt`, but buyer receives no reputation boost

### `cancel_escrow`

Cancels an escrow before it is funded, refunding any deposited funds.

```rust
fn cancel_escrow(
    env: Env,
    caller: Address,
    escrow_id: u64,
) -> void;
```

**Preconditions:**
- Escrow is in `Created` state (buyer cancels)
- OR Escrow is in `Funded` state with mutual consent (both signatures)

**Postconditions:**
- Escrow transitions to `Cancelled` state
- If funded, Pi tokens returned to buyer
- `ESCROW_CANCELLED` event emitted

### `expire_escrow`

Marks escrow as expired if delivery deadline passes without seller action.

```rust
fn expire_escrow(
    env: Env,
    caller: Address,
    escrow_id: u64,
) -> void;
```

**Preconditions:**
- Escrow is in `Funded` state
- Current time > delivery_deadline

**Postconditions:**
- Escrow transitions to `Expired` state
- Pi tokens refunded to buyer
- Seller reputation negatively impacted
- `ESCROW_EXPIRED` event emitted

## Timeout Parameters

| Parameter | Default | Min | Max | Configurable By |
|-----------|---------|-----|-----|-----------------|
| `delivery_deadline` | 7 days | 1 day | 30 days | Buyer (at creation) |
| `auto_release_timeout` | 7 days | 1 day | 14 days | Buyer (at creation) |
| `dispute_window` | 3 days | 1 day | 7 days | Protocol (upgradeable) |

## Security Considerations

### Reentrancy Protection
All state changes occur before external token transfers, following the Checks-Effects-Interactions pattern.

### Fund Safety
- Escrow contract holds funds in a dedicated Soroban account, not the coordinator
- Funds can only be released through explicit `confirm_receipt`, `auto_release`, or dispute resolution
- No admin key can override escrow state

### Timeout Enforcement
- Any party can trigger timeout-based actions (expire, auto-release)
- This prevents griefing where one party becomes unresponsive
- A small incentive (0.05 Pi) is awarded to the caller of `auto_release` or `expire_escrow` to encourage monitoring

### Linked Subscriptions
When an escrow is linked to a PiRC2 subscription:
- Escrow amount equals the subscription period payment
- Auto-release is disabled (subscription disputes follow PiRC2 rules)
- Dispute resolution integrates with PiRC2's subscription-specific error codes
