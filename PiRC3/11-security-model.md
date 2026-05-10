# PiRC3 — Section 11: Security Model

## Overview

This section defines the comprehensive security model for PiDCTP, covering threat analysis, mitigation strategies, access control, and audit considerations.

## Threat Model

### Threat Categories

| Category | Threat | Severity | Mitigation |
|----------|--------|----------|------------|
| **Economic** | Buyer refuses to confirm receipt | Medium | Auto-release timeout |
| **Economic** | Seller fails to deliver | High | Escrow expiry + reputation penalty |
| **Economic** | Juror collusion | High | Hidden selection + commit-reveal + appeal |
| **Economic** | Sybil reputation farming | High | Pi identity binding + rate limiting |
| **Economic** | Griefing via frivolous disputes | Medium | Non-refundable filing fee if dismissed |
| **Technical** | Reentrancy attack | Critical | Checks-Effects-Interactions pattern |
| **Technical** | Front-running escrow creation | Medium | Soroban's deterministic ordering |
| **Technical** | Integer overflow/underflow | High | Soroban's checked arithmetic |
| **Technical** | Unauthorized contract upgrade | Critical | 3-of-5 multi-sig + 48h timelock |
| **Privacy** | Transaction graph analysis | Medium | Future ZK proof integration |
| **Privacy** | Juror identity before ruling | Medium | Delayed juror reveal |
| **Governance** | Admin key compromise | Critical | Multi-sig + timelock + social recovery |

## Access Control Matrix

### Contract-Level Permissions

| Action | Buyer | Seller | Juror | Agent | Admin | Any |
|--------|-------|--------|-------|-------|-------|-----|
| `create_escrow` | ✅ | | | | | |
| `fund_escrow` | ✅ | | | | | |
| `confirm_delivery` | | ✅ | | | | |
| `confirm_receipt` | ✅ | | | | | |
| `auto_release` | | | | | | ✅ |
| `cancel_escrow` | ✅* | ✅* | | | | |
| `expire_escrow` | | | | | | ✅ |
| `open_dispute` | ✅ | ✅ | | | | |
| `submit_evidence` | ✅ | ✅ | | | | |
| `cast_vote` | | | ✅ | | | |
| `execute_ruling` | | | | | | ✅ |
| `appeal_ruling` | ✅ | ✅ | | | | |
| `apply_verification` | ✅** | | | | | |
| `approve_verification` | | | | ✅ | | |
| `suspend_merchant` | | | | ✅ | | |
| `revoke_verification` | | | | ✅ | | |
| `rate_merchant` | ✅*** | | | | | |
| `redeem_points` | ✅ | | | | | |
| `pause_protocol` | | | | | ✅ | |
| `upgrade_contract` | | | | | ✅**** | |

\* Buyer can cancel in Created state; both parties required for Funded state  
\** Any Pioneer can apply for merchant verification  
\*** Only buyer of a completed escrow can rate  
\**** Requires 3-of-5 multi-sig + 48-hour timelock

## Security Mechanisms

### 1. Escrow Fund Safety

**Problem:** Escrow holds Pi tokens that must be protected from unauthorized access.

**Solution:**
- Escrow contract holds funds in its own Soroban account (not the coordinator)
- Funds can only be released through four paths:
  1. `confirm_receipt` (buyer confirms)
  2. `auto_release` (timeout expires)
  3. `execute_ruling` (dispute resolution)
  4. `cancel_escrow` (mutual consent or pre-fund)
- No admin key can override escrow state
- All release paths are verifiable on-chain

### 2. Reentrancy Protection

**Problem:** Malicious contract could call back into PiDCTP during token transfer.

**Solution:**
- All state changes occur before external calls (Checks-Effects-Interactions)
- Soroban's transaction model provides natural reentrancy protection
- Additional guard: `non_reentrant` flag per escrow prevents double-execution

```rust
fn confirm_receipt(env: Env, buyer: Address, escrow_id: u64) {
    // CHECKS
    let mut escrow = get_escrow(&env, escrow_id);
    assert!(escrow.state == EscrowState::Delivered, "Invalid state");
    assert!(buyer == escrow.buyer, "Not buyer");
    
    // EFFECTS (state changes first)
    escrow.state = EscrowState::Completed;
    set_escrow(&env, &escrow);
    
    // INTERACTIONS (external calls last)
    token_transfer(&env, &escrow.token, &escrow_id, &escrow.seller, escrow.amount);
    update_reputation(&env, &escrow);
    award_loyalty(&env, &escrow);
}
```

### 3. Juror Selection Integrity

**Problem:** Juror selection could be manipulated to favor one party.

**Solution:**
- **VRF-based selection**: Uses Soroban's verifiable random function for unpredictable, verifiable selection
- **Hidden jurors**: Juror identities are not revealed until after ruling
- **Conflict filtering**: Jurors with prior transactions with either party are excluded
- **Appeal mechanism**: First-round collusion can be overturned by a different jury
- **Stake requirement**: Jurors must stake 10 Pi, which can be slashed for proven collusion

### 4. Commit-Reveal Voting

**Problem:** Early voters could influence later jurors.

**Solution:**
- **Commit phase**: Jurors submit a hash of their vote (vote + salt)
- **Reveal phase**: After voting deadline, jurors reveal their vote and salt
- **Invalid commits**: Jurors who commit but don't reveal lose their bond portion

```rust
struct CommitVote {
    /// Hash of (vote + salt)
    commitment: BytesN<32>,
    /// Whether vote has been revealed
    revealed: bool,
}

fn commit_vote(env: Env, juror: Address, dispute_id: u64, commitment: BytesN<32>) -> void;
fn reveal_vote(env: Env, juror: Address, dispute_id: u64, vote: DisputeRuling, salt: BytesN<32>) -> void;
```

### 5. Upgrade Safety

**Problem:** Malicious or compromised admin could upgrade contracts to steal funds.

**Solution:**
- **Multi-sig admin**: 3-of-5 key holders required for upgrade
- **48-hour timelock**: Upgrade is queued but not executed for 48 hours
- **Emergency pause**: 2-of-5 can pause the protocol immediately (but cannot upgrade)
- **Social recovery**: If 3-of-5 keys are compromised, Diamond-tier Pioneers can vote to replace admin keys

### 6. Rate Limiting

**Problem:** Automated scripts could create many escrows or disputes to spam the network.

**Solution:**
- **Escrow creation**: Maximum 10 escrows per Pioneer per 24 hours
- **Dispute filing**: Maximum 3 disputes per Pioneer per 7 days
- **Evidence submission**: Maximum 5 evidence items per party per dispute
- **Juror acceptance**: Maximum 3 juror assignments per Pioneer per 7 days

### 7. Economic Attack Resistance

**Problem:** An attacker could profit by exploiting fee structures or reward mechanisms.

**Mitigations:**

| Attack | Mitigation |
|--------|------------|
| **Free escrow farming** | Escrow creation fee (0.1 Pi) exceeds loyalty point value |
| **Dispute fee arbitrage** | Filing fee is non-refundable if dismissed |
| **Juror self-selection** | VRF prevents targeted juror selection |
| **Reputation wash trading** | Repeat transactions with same party are weighted less |
| **Loyalty point farming** | Points capped per action; tier limited by reputation |
| **Referral fraud** | Referred Pioneer must complete real escrow first |

## Audit Considerations

### Pre-Deployment Audit Checklist

- [ ] All arithmetic uses checked operations (Soroban default)
- [ ] No uninitialized storage access
- [ ] All state transitions validated against allowed transitions
- [ ] All external calls follow Checks-Effects-Interactions
- [ ] Access control enforced on all admin functions
- [ ] Timelock enforced on all upgrade operations
- [ ] Event emission on all state-changing operations
- [ ] Fee calculations verified for edge cases (zero amount, maximum amount)
- [ ] Dispute ruling execution handles all ruling variants correctly
- [ ] Reputation score calculation matches specification exactly

### Recommended Audit Firms

| Firm | Specialization | Cost Estimate |
|------|---------------|---------------|
| CertiK | Smart contract, formal verification | $50K–$100K |
| Trail of Bits | Security, cryptography | $40K–$80K |
| OpenZeppelin | Smart contract audit | $30K–$60K |
| Sigma Prime | Stellar/Soroban specialization | $25K–$50K |

### Ongoing Security Monitoring

- **Real-time monitoring**: Off-chain indexer tracks all PiDCTP events for anomaly detection
- **Periodic review**: Quarterly security review of contract state and access patterns
- **Bug bounty**: 5,000 Pi bounty program for responsible disclosure of critical vulnerabilities
- **Incident response**: 24-hour response window for critical security events

## Emergency Procedures

### Circuit Breaker

The protocol includes a circuit breaker that can be activated by 2-of-5 admin keys:

```rust
fn emergency_pause(env: Env, admin: Address, reason: Symbol) -> void;
fn emergency_unpause(env: Env, admin: Address) -> void;
```

**When paused:**
- No new escrows can be created
- No new disputes can be filed
- Existing escrows remain frozen (no auto-release)
- Existing disputes continue to resolution (to prevent indefinite freezing)

**Unpause requires:**
- 3-of-5 admin approval
- 24-hour delay after unpause proposal
- Public disclosure of pause reason

### Fund Recovery

In the event of a critical vulnerability:
1. Circuit breaker activated immediately
2. Vulnerability assessed and patched
3. Contract upgraded with fix (48h timelock waived for emergency — requires 5-of-5)
4. Affected funds restored from treasury reserve
5. Post-mortem published within 72 hours
