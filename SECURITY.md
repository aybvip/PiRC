# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| v1.0.x  | ✅ |
| < 1.0   | ❌ (pre-release) |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

### Responsible Disclosure

1. **Email**: Send details to the project maintainers via GitHub's private vulnerability reporting feature
2. **GitHub Security Advisory**: Use the "Report a vulnerability" button on the [Security tab](https://github.com/aybvip/PiRC/security)
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact (fund loss, reputation manipulation, etc.)
   - Suggested fix (if available)

### Response Timeline

| Stage | Timeline |
|-------|----------|
| Acknowledgment | Within 24 hours |
| Initial assessment | Within 72 hours |
| Fix development | Within 7 days (critical) / 30 days (non-critical) |
| Disclosure | After fix is deployed and verified |

### Bug Bounty Program

| Severity | Reward | Criteria |
|----------|--------|---------|
| **Critical** | 5,000 Pi | Fund theft, contract takeover, arbitrary fund release |
| **High** | 2,000 Pi | Escrow bypass, reputation manipulation, dispute collusion exploit |
| **Medium** | 500 Pi | Unauthorized state transitions, fee calculation errors |
| **Low** | 100 Pi | Denial of service, minor logic errors |

### Scope

**In Scope:**
- All smart contracts in `contracts/`
- Coordinator routing logic
- Cross-contract interactions
- Fee calculation logic
- Juror selection algorithm
- Reputation score calculation

**Out of Scope:**
- Third-party dependencies (Soroban SDK, Stellar network)
- Front-end applications
- Social engineering attacks
- Network-level attacks (DDoS on Stellar validators)

## Security Architecture

### Smart Contract Security

- **Checks-Effects-Interactions**: All contracts follow this pattern to prevent reentrancy
- **Access Control**: Role-based permissions enforced at every entry point
- **Timelock**: 48-hour delay on contract upgrades for community review
- **Multi-sig**: 3-of-5 admin keys required for critical operations
- **Emergency Pause**: 2-of-5 can pause the protocol immediately

### Escrow Fund Protection

- Funds held in dedicated Soroban account (not admin-controlled)
- Only four release paths: `confirm_receipt`, `auto_release`, `execute_ruling`, `cancel_escrow`
- No admin override for escrow state
- All fund movements emit on-chain events

### Dispute Resolution Security

- **Commit-Reveal Voting**: Prevents vote copying and juror influence
- **VRF Juror Selection**: Unpredictable, verifiable random selection
- **Hidden Jurors**: Identities not revealed until after ruling
- **Appeal Mechanism**: First-round collusion can be overturned
- **Juror Bond**: 10 Pi stake slashable for non-participation or proven collusion

### Known Security Considerations

1. **Front-running**: Soroban's deterministic transaction ordering mitigates this
2. **Flash loan attacks**: Not applicable — PiDCTP does not use price oracles or AMM logic
3. **Governance attacks**: Multi-sig + timelock + social recovery prevents admin key compromise
4. **Reputation gaming**: Rate limits, Sybil resistance, and counterparty weighting mitigate this

## Security Audit

### Pre-Deployment Requirements

- [ ] External security audit by recognized firm
- [ ] All audit findings addressed or documented
- [ ] Testnet deployment with real-world testing (2+ weeks)
- [ ] Community review period completed

### Ongoing Security

- Real-time event monitoring for anomaly detection
- Quarterly security review of contract state
- Continuous integration security scanning (CodeQL)
