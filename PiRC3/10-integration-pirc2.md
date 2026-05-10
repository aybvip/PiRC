# PiRC3 — Section 10: Integration with PiRC2

## Overview

PiRC3 is designed to integrate seamlessly with **PiRC2 (Subscription Contract API)**, extending subscription-based services with escrow protection, reputation tracking, and dispute resolution. This section defines the integration points and cross-contract interactions.

## Why Integrate with PiRC2?

PiRC2 enables merchants to offer subscription services with recurring payments. However, it currently lacks:

- **Buyer protection** — No escrow for subscription payments
- **Service quality enforcement** — No mechanism to dispute non-delivery of subscribed services
- **Merchant accountability** — No reputation system for subscription providers
- **Loyalty incentives** — No rewards for long-term subscribers

PiRC3 fills these gaps by wrapping PiRC2 subscriptions in the trust infrastructure layer.

## Integration Architecture

```
┌─────────────────────────────────────────────────┐
│                  PiDCTP Coordinator               │
├──────────┬──────────┬───────────┬───────────────┤
│  Escrow  │Reputation│  Dispute  │   Loyalty     │
│  System  │ Engine   │Resolution │   Rewards     │
├──────────┴──────────┴───────────┴───────────────┤
│              Integration Layer                     │
├──────────────────────────────────────────────────┤
│              PiRC2 Subscription API               │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │ Service  │ │Subscribe │ │  Process Charge  │ │
│  │Management│ │Lifecycle │ │  (Recurring)     │ │
│  └──────────┘ └──────────┘ └──────────────────┘ │
└──────────────────────────────────────────────────┘
```

## Integration Points

### 1. Subscription-Linked Escrow

When a buyer subscribes to a PiRC2 service, PiRC3 can optionally create an escrow for each billing period.

```rust
fn create_subscription_escrow(
    env: Env,
    buyer: Address,
    subscription_id: u64,  // PiRC2 subscription ID
    amount: i128,          // Subscription period charge
) -> EscrowId;
```

**Flow:**
1. PiRC2 `process_charge` triggers a notification to PiDCTP Coordinator
2. Coordinator creates an escrow for the subscription period amount
3. Funds are held in escrow until buyer confirms service delivery
4. If service not delivered, buyer can open dispute instead of auto-release

**Key Difference from Standard Escrow:**
- Auto-release is **disabled** for subscription escrows (prevents auto-payment without service confirmation)
- Delivery confirmation is implicit: if buyer does not dispute within 3 days of charge, escrow auto-completes
- Dispute follows PiRC2-specific error codes for service-related issues

### 2. Subscription Reputation Tracking

PiRC3 Reputation Engine tracks subscription-specific metrics alongside standard escrow metrics.

**Additional Reputation Factors for Subscriptions:**

| Factor | Impact | Description |
|--------|--------|-------------|
| Subscription retention rate | +2 per retained month | Merchant keeps subscribers |
| Subscription cancellation rate | -5 per cancellation | Subscribers leave |
| Service dispute rate | -10 per dispute | Subscribers file disputes |
| Long-term subscriber bonus | +15 per 6-month subscriber | Merchant retains long-term |

**New Data Points in ReputationProfile:**

```rust
struct SubscriptionReputationData {
    /// Total subscriptions offered (as merchant)
    total_subscriptions_offered: u32,
    /// Active subscribers currently
    active_subscribers: u32,
    /// Average subscription duration (days)
    avg_subscription_duration: u32,
    /// Subscription-specific disputes
    subscription_disputes: u32,
    /// Subscription retention rate (0–10000, where 10000 = 100%)
    retention_rate: u32,
}
```

### 3. Subscription Dispute Categories

PiRC3 extends the dispute system with subscription-specific categories:

```rust
enum SubscriptionDisputeCategory {
    /// Service not accessible after payment
    ServiceInaccessible,
    /// Service quality below advertised standard
    QualityBelowStandard,
    /// Unauthorized recurring charge after cancellation
    ChargeAfterCancellation,
    /// Billing amount differs from agreed subscription price
    BillingDiscrepancy,
    /// Service features missing compared to subscription tier
    FeaturesMissing,
}
```

**Dispute Resolution for Subscriptions:**

| Ruling | Effect on Subscription | Refund |
|--------|----------------------|--------|
| `FullRefund` | Subscription cancelled, full period refund | 100% |
| `PartialRefund` | Subscription continues, partial refund | buyer_percentage% |
| `SellerFavored` | Subscription continues, no refund | 0% |
| `ServiceInaccessible` | Subscription cancelled + full refund + penalty | 100% + 10% penalty |
| `ChargeAfterCancellation` | Full refund + cancellation confirmation | 100% |

### 4. Subscription Merchant Verification

PiRC3 Merchant Verification integrates with PiRC2's service management:

- **Subscription merchants** must meet Standard verification level (not just Basic)
- **Service description hash** from PiRC2 is cross-referenced with merchant profile
- **Subscription pricing** must match merchant's advertised service rates
- **Service uptime** is tracked as a verification criterion

**Additional Verification Requirements for Subscription Merchants:**

| Requirement | Standard Level | Premium Level |
|-------------|---------------|---------------|
| Minimum active subscriptions | 5 | 20 |
| Service uptime (30-day) | ≥ 95% | ≥ 99% |
| Subscription dispute rate | ≤ 5% | ≤ 2% |
| Average subscriber retention | ≥ 30 days | ≥ 90 days |

### 5. Subscription Loyalty Integration

PiRC3 Loyalty module awards points for subscription-specific activities:

| Action | Points | Limit |
|--------|--------|-------|
| Subscribe to a verified service | +5 | Per subscription |
| Complete 3 billing cycles | +10 | Per subscription |
| Complete 6 billing cycles | +25 | Per subscription |
| Complete 12 billing cycles | +50 | Per subscription |
| Rate a subscription service | +2 | Per rating |
| Refer a subscriber | +10 | Per referral |

**Subscriber Loyalty Tiers:**

| Tier | Subscription Discount | Benefit |
|------|----------------------|---------|
| Starter | 0% | — |
| Regular | 5% | Priority support |
| Trusted | 10% | Early access to new services |
| Elite | 15% | Exclusive service tiers |
| Legendary | 20% | Free trial periods |

## Cross-Contract Call Flow

### Subscription Charge with Escrow Protection

```
PiRC2                Coordinator              Escrow              Reputation
  │                      │                      │                    │
  │──process_charge─────►│                      │                    │
  │                      │──create_escrow──────►│                    │
  │                      │                      │──lock_funds───────►│
  │                      │                      │                    │
  │              [3-day confirmation window]   │                    │
  │                      │                      │                    │
  │         [Buyer confirms OR auto-complete]   │                    │
  │                      │──release_funds──────►│                    │
  │                      │──update_reputation───────────────────────►│
  │                      │──award_loyalty──────►│                    │
  │◄──charge_confirmed───│                      │                    │
```

### Subscription Cancellation with Dispute

```
PiRC2                Coordinator          Dispute              Escrow
  │                      │                    │                    │
  │──cancel_request─────►│                    │                    │
  │                      │──check_escrow─────►│                    │
  │                      │                    │──freeze_funds─────►│
  │                      │                    │                    │
  │              [Dispute process follows standard flow]           │
  │                      │                    │                    │
  │                      │──execute_ruling───►│──release──────────►│
  │                      │──cancel_sub───────►│                    │
  │◄──subscription_cancelled─│               │                    │
```

## PiRC2 Contract Interface Extensions

PiRC3 defines the following interface that PiRC2 contracts should implement for integration:

```rust
trait PiRC2Integration {
    /// Get subscription details by ID
    fn get_subscription(env: Env, subscription_id: u64) -> SubscriptionInfo;
    
    /// Get service details by service ID
    fn get_service(env: Env, service_id: u64) -> ServiceInfo;
    
    /// Verify that a charge corresponds to an active subscription
    fn verify_charge(env: Env, subscription_id: u64, amount: i128) -> bool;
    
    /// Cancel a subscription (called by PiDCTP after dispute ruling)
    fn cancel_subscription(env: Env, subscription_id: u64, reason: Symbol) -> void;
    
    /// Get merchant address for a service
    fn get_service_merchant(env: Env, service_id: u64) -> Address;
}
```

## Migration Path

For existing PiRC2 deployments, integration with PiRC3 is **opt-in**:

1. **Phase 1**: PiDCTP deploys alongside PiRC2, reading subscription data read-only
2. **Phase 2**: PiRC2 adds PiDCTP callback hooks (upgradeable contract)
3. **Phase 3**: Full integration with escrow-protected subscription charges

Merchants can choose to enable PiDCTP protection for their services, gaining:
- Trust badge on their service listings
- Access to the loyalty program subscriber pool
- Dispute resolution infrastructure
- Enhanced reputation visibility

Subscribers benefit from:
- Escrow protection for each billing period
- Dispute resolution for service issues
- Loyalty points for subscription activity
- Verified merchant confidence
