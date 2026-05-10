# PiRC3 — Section 9: Error Codes

## Overview

This section defines all error codes used across PiDCTP modules. Error codes follow a structured format: **MODULE-ERROR_ID**, where MODULE is a 3-digit prefix and ERROR_ID is a unique identifier within that module.

## Error Code Format

```
MMMEEE
││ │││
││ └──┴── Error ID (001–999)
│└─────── Module prefix (100–600)
└──────── PiRC3 prefix
```

| Module | Prefix |
|--------|--------|
| Coordinator | 100 |
| Escrow | 200 |
| Reputation | 300 |
| Dispute | 400 |
| Merchant Verification | 500 |
| Loyalty | 600 |

## Coordinator Errors (100xxx)

| Code | Name | Description |
|------|------|-------------|
| 100001 | `UNAUTHORIZED_CALLER` | Caller is not authorized to perform this action |
| 100002 | `MODULE_NOT_FOUND` | Referenced module contract address not set |
| 100003 | `INVALID_STATE_TRANSITION` | Requested state transition is not allowed |
| 100004 | `PAUSED` | Protocol is currently paused |
| 100005 | `INSUFFICIENT_FEE` | Required fee not deposited |
| 100006 | `DUPLICATE_TRANSACTION` | Transaction with same parameters already exists |
| 100007 | `ADMIN_REQUIRED` | Admin-level permission required |
| 100008 | `TIMELOCK_ACTIVE` | Upgrade timelock has not expired |

## Escrow Errors (200xxx)

| Code | Name | Description |
|------|------|-------------|
| 200001 | `ESCROW_NOT_FOUND` | Escrow ID does not exist |
| 200002 | `ESCROW_NOT_CREATED_STATE` | Escrow is not in Created state |
| 200003 | `ESCROW_NOT_FUNDED_STATE` | Escrow is not in Funded state |
| 200004 | `ESCROW_NOT_DELIVERED_STATE` | Escrow is not in Delivered state |
| 200005 | `ESCROW_ALREADY_FUNDED` | Escrow has already been funded |
| 200006 | `BUYER_ONLY` | Only the buyer can perform this action |
| 200007 | `SELLER_ONLY` | Only the seller can perform this action |
| 200008 | `BUYER_SELLER_SAME` | Buyer and seller cannot be the same address |
| 200009 | `INVALID_AMOUNT` | Amount must be greater than zero |
| 200010 | `INSUFFICIENT_BALANCE` | Buyer does not have sufficient Pi tokens |
| 200011 | `INSUFFICIENT_ALLOWANCE` | Buyer has not approved sufficient token allowance |
| 200012 | `DELIVERY_DEADLINE_PASSED` | Delivery deadline has already passed |
| 200013 | `CONFIRMATION_DEADLINE_NOT_PASSED` | Auto-release timeout has not yet expired |
| 200014 | `ESCROW_DISPUTED` | Escrow is under dispute, action not allowed |
| 200015 | `CANCELLATION_NOT_ALLOWED` | Escrow cannot be cancelled in current state |
| 200016 | `MUTUAL_CANCEL_REQUIRED` | Both parties must consent for funded escrow cancellation |
| 200017 | `INVALID_DEADLINE` | Deadline must be in the future |
| 200018 | `INVALID_TIMEOUT` | Auto-release timeout below minimum (1 day) |
| 200019 | `ESCROW_ALREADY_LINKED` | Escrow already linked to a subscription |

## Reputation Errors (300xxx)

| Code | Name | Description |
|------|------|-------------|
| 300001 | `PROFILE_NOT_FOUND` | Reputation profile does not exist |
| 300002 | `SCORE_BELOW_THRESHOLD` | Pioneer's score is below the required threshold |
| 300003 | `TIER_INSUFFICIENT` | Pioneer's tier is below the required level |
| 300004 | `INVALID_SCORE_UPDATE` | Score update does not originate from coordinator |
| 300005 | `SCORE_RATE_LIMIT_EXCEEDED` | Maximum score gain per 24-hour period exceeded |
| 300006 | `INVALID_TIER_TRANSITION` | Tier transition not allowed |
| 300007 | `MERCHANT_NOT_VERIFIED` | Pioneer is not a verified merchant |
| 300008 | `DECAY_FLOOR_REACHED` | Score cannot decay below minimum floor |
| 300009 | `DUPLICATE_REPUTATION_UPDATE` | Same update already applied (nonce check) |

## Dispute Errors (400xxx)

| Code | Name | Description |
|------|------|-------------|
| 400001 | `DISPUTE_NOT_FOUND` | Dispute ID does not exist |
| 400002 | `DISPUTE_ALREADY_EXISTS` | A dispute is already open for this escrow |
| 400003 | `NOT_DISPUTE_PARTY` | Caller is not a party to this dispute |
| 400004 | `NOT_EVIDENCE_PHASE` | Dispute is not in evidence submission phase |
| 400005 | `NOT_VOTING_PHASE` | Dispute is not in voting phase |
| 400006 | `EVIDENCE_LIMIT_EXCEEDED` | Maximum evidence submissions reached (5 per party) |
| 400007 | `EVIDENCE_DEADLINE_PASSED` | Evidence submission deadline has passed |
| 400008 | `VOTING_DEADLINE_PASSED` | Voting deadline has passed |
| 400009 | `NOT_SELECTED_JUROR` | Caller is not a selected juror for this dispute |
| 400010 | `JUROR_ALREADY_VOTED` | Juror has already cast a vote |
| 400011 | `INVALID_CONFIDENCE` | Confidence must be between 1 and 5 |
| 400012 | `INSUFFICIENT_JURORS` | Not enough jurors have been selected |
| 400013 | `APPEAL_NOT_ALLOWED` | Dispute cannot be appealed (already appealed or window expired) |
| 400014 | `APPEAL_FEE_INSUFFICIENT` | Appeal fee (2.0 Pi) not deposited |
| 400015 | `DISPUTE_FEE_INSUFFICIENT` | Dispute filing fee (1.0 Pi) not deposited |
| 400016 | `JUROR_NOT_ELIGIBLE` | Pioneer does not meet juror eligibility criteria |
| 400017 | `JUROR_CONFLICT_OF_INTEREST` | Juror is a party to the dispute |
| 400018 | `JUROR_RECENTLY_SERVED` | Juror served within the last 7 days |
| 400019 | `JUROR_BOND_INSUFFICIENT` | Juror has not staked the required 10 Pi bond |
| 400020 | `RULING_ALREADY_EXECUTED` | Dispute ruling has already been executed |
| 400021 | `INVALID_RULING` | Ruling parameters are invalid |

## Merchant Verification Errors (500xxx)

| Code | Name | Description |
|------|------|-------------|
| 500001 | `MERCHANT_NOT_FOUND` | Merchant profile does not exist |
| 500002 | `ALREADY_VERIFIED` | Merchant is already verified at this level or higher |
| 500003 | `VERIFICATION_PENDING` | Merchant already has a pending application |
| 500004 | `NOT_VERIFICATION_AGENT` | Caller is not an authorized verification agent |
| 500005 | `NOT_UNDER_REVIEW` | Application is not in UnderReview status |
| 500006 | `VERIFICATION_FEE_INSUFFICIENT` | Verification fee not deposited |
| 500007 | `REPUTATION_BELOW_MINIMUM` | Reputation score below required minimum for level |
| 500008 | `ACCOUNT_TOO_NEW` | Account age below required minimum |
| 500009 | `INSUFFICIENT_ESCROWS` | Not enough completed escrows for requested level |
| 500010 | `NOT_APPROVED_MERCHANT` | Merchant is not in Approved status |
| 500011 | `RATING_ALREADY_SUBMITTED` | Buyer has already rated this escrow |
| 500012 | `INVALID_RATING` | Rating must be between 50 and 500 |
| 500013 | `ESCROW_NOT_COMPLETED` | Cannot rate an escrow that is not completed |
| 500014 | `NOT_BUYER_OF_ESCROW` | Caller is not the buyer of the specified escrow |
| 500015 | `RENEWAL_NOT_DUE` | Verification is not within renewal window |
| 500016 | `RENEWAL_EXPIRED` | Verification expired more than 90 days ago |
| 500017 | `AGENT_STAKE_INSUFFICIENT` | Agent has not staked the required 50 Pi bond |
| 500018 | `AGENT_NOT_ELIGIBLE` | Agent does not meet eligibility criteria |

## Loyalty Errors (600xxx)

| Code | Name | Description |
|------|------|-------------|
| 600001 | `LOYALTY_PROFILE_NOT_FOUND` | Loyalty profile does not exist |
| 600002 | `INSUFFICIENT_POINTS` | Not enough redeemable points for requested reward |
| 600003 | `INVALID_REWARD_TYPE` | Specified reward type does not exist |
| 600004 | `REDEMPTION_LIMIT_EXCEEDED` | Maximum redemptions for this reward type reached |
| 600005 | `TIER_EXCEEDS_REPUTATION` | Loyalty tier cannot exceed reputation tier |
| 600006 | `REFERRAL_LIMIT_EXCEEDED` | Maximum referral bonuses (50) already earned |
| 600007 | `REFERRED_PIONEER_INELIGIBLE` | Referred Pioneer has not completed required escrow |
| 600008 | `REPUTATION_BOOST_QUOTA_EXCEEDED` | Quarterly reputation boost already redeemed |
| 600009 | `POINTS_NEGATIVE` | Points cannot go below zero |
| 600010 | `INVALID_TIER_FOR_REWARD` | Current tier does not qualify for this reward |
