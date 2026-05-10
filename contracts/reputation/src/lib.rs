#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Map, Vec};

// ============================================================================
// Reputation Contract — PiDCTP Module 2
// + Soulbound Badges, Attestations, Sybil Resistance, ZK Roadmap
// ============================================================================

#[contract]
pub struct ReputationContract;

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ReputationTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
}

// ============================================================================
// Soulbound Reputation Badges (SBTs)
// Non-transferable, non-tradable on-chain credentials representing
// what a Pioneer has DONE, not what they HOLD.
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum SoulboundBadge {
    FirstTrade,          // Completed first escrow
    TrustedBuyer,        // 10+ completed purchases
    TrustedSeller,       // 10+ completed sales
    VerifiedMerchant,    // Passed KYB verification
    JurorVeteran,        // Served on 5+ dispute panels
    CommunityGuardian,   // 20+ dispute rulings with consensus
    EarlyAdopter,        // Active in first 90 days of protocol
    PlatinumTrader,      // 100+ completed escrows
    DiamondElite,        // Reached Diamond tier (900+)
    LoyaltyChampion,     // Reached Legendary loyalty tier
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct BadgeOwnership {
    pub pioneer: Address,
    pub badge: SoulboundBadge,
    pub awarded_at: u64,
    pub award_reason: BytesN<32>,
    pub revoked: bool,
}

// ============================================================================
// Reputation Attestations
// Third-party vouching: verified entities can attest to a Pioneer's trustworthiness
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum AttestationType {
    IdentityVouch,      // "I know this person is real"
    CommerceVouch,      // "I've done business with them successfully"
    SkillVouch,         // "They have this skill/competency"
    CommunityVouch,     // "They're an active community member"
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Attestation {
    pub attestation_id: u64,
    pub attester: Address,        // Who is vouching
    pub attested: Address,        // Who is being vouched for
    pub attestation_type: AttestationType,
    pub attester_reputation: u32, // Attester's score at time of attestation
    pub weight: u32,              // Derived from attester's tier (1-5)
    pub created_at: u64,
    pub expires_at: u64,          // Attestations expire after 180 days
    pub active: bool,
}

// ============================================================================
// Sybil Resistance — Social Graph Analysis
// Tracks transaction patterns to detect fake/sockpuppet accounts
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct SybilProfile {
    pub pioneer: Address,
    pub unique_counterparties: u32,  // Distinct addresses transacted with
    pub total_transactions: u32,
    pub reciprocal_ratio: u32,       // % of txs that are 2-way (0-10000 bps)
    pub avg_tx_interval: u64,        // Average seconds between transactions
    pub cluster_flag: bool,          // Flagged as potential Sybil cluster
    pub last_analysis: u64,
    pub sybil_score: u32,            // 0 = human, 10000 = definite Sybil
}

// ============================================================================
// ZK Reputation Verification Roadmap
// Future: Prove tier without revealing exact score
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ZKVerificationProof {
    pub pioneer: Address,
    pub claimed_tier: ReputationTier,
    pub proof_hash: BytesN<32>,      // ZK-SNARK proof hash (future)
    pub verified_at: u64,
    pub verifier_contract: Address,  // Future ZK verifier contract
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ReputationProfile {
    pub pioneer: Address,
    pub score: u32,
    pub tier: ReputationTier,
    pub total_escrows: u32,
    pub completed_escrows: u32,
    pub expired_escrows: u32,
    pub disputes_as_buyer: u32,
    pub disputes_as_seller: u32,
    pub rulings_in_favor: u32,
    pub rulings_against: u32,
    pub is_verified_merchant: bool,
    pub created_at: u64,
    pub last_active: u64,
    pub history_root: BytesN<32>,
    pub score_nonce: u32,
    // --- v1.1: Advanced Reputation Features ---
    pub badge_count: u32,              // Number of soulbound badges earned
    pub attestation_score: u32,        // Weighted score from third-party attestations
    pub sybil_score: u32,             // 0=human, 10000=definite Sybil
    pub unique_counterparties: u32,   // Distinct addresses transacted with
}

const COORDINATOR: Symbol = Symbol::new("coordinator");
const PAUSED: Symbol = Symbol::new("paused");
const PROFILES: Symbol = Symbol::new("profiles");
const DECAY_LAST: Symbol = Symbol::new("decay_last");

// Score boundaries for tiers
const BRONZE_MAX: u32 = 199;
const SILVER_MAX: u32 = 449;
const GOLD_MAX: u32 = 699;
const PLATINUM_MAX: u32 = 899;
// DIAMOND: 900-1000

fn profile_key(pioneer: &Address) -> (Symbol, Address) {
    (PROFILES, pioneer.clone())
}

fn score_to_tier(score: u32) -> ReputationTier {
    if score <= BRONZE_MAX {
        ReputationTier::Bronze
    } else if score <= SILVER_MAX {
        ReputationTier::Silver
    } else if score <= GOLD_MAX {
        ReputationTier::Gold
    } else if score <= PLATINUM_MAX {
        ReputationTier::Platinum
    } else {
        ReputationTier::Diamond
    }
}

fn clamp_score(score: u32) -> u32 {
    if score > 1000 { 1000 } else if score < 50 { 50 } else { score }
}

fn require_coordinator(env: &Env, caller: &Address) {
    let coordinator: Address = env.storage().instance().get(&COORDINATOR).unwrap();
    assert!(&caller == &coordinator, "Only coordinator");
}

fn require_not_paused(env: &Env) {
    let paused: bool = env.storage().instance().get(&PAUSED).unwrap_or(false);
    assert!(!paused, "Protocol paused");
}

#[contractimpl]
impl ReputationContract {
    /// Initialize with coordinator address
    pub fn initialize(env: Env, coordinator: Address) {
        let storage = env.storage().instance();
        assert!(!storage.has(&COORDINATOR), "Already initialized");
        storage.set(&COORDINATOR, &coordinator);
        storage.set(&PAUSED, &false);
    }

    /// Create a new reputation profile (called when Pioneer first interacts)
    pub fn create_profile(env: Env, pioneer: Address) -> ReputationProfile {
        require_not_paused(&env);
        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        assert!(!storage.has(&key), "Profile exists");

        let profile = ReputationProfile {
            pioneer: pioneer.clone(),
            score: 200, // Start at Silver floor
            tier: ReputationTier::Silver,
            total_escrows: 0,
            completed_escrows: 0,
            expired_escrows: 0,
            disputes_as_buyer: 0,
            disputes_as_seller: 0,
            rulings_in_favor: 0,
            rulings_against: 0,
            is_verified_merchant: false,
            created_at: env.ledger().timestamp(),
            last_active: env.ledger().timestamp(),
            history_root: BytesN::from_array(&env, &[0u8; 32]),
            score_nonce: 0,
            // --- v1.1: Advanced Reputation Features ---
            badge_count: 0,
            attestation_score: 0,
            sybil_score: 0,
            unique_counterparties: 0,
        };

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("reputation_created"), pioneer.clone()),
            (200u32,),
        );

        profile
    }

    /// Record escrow completion (called by coordinator)
    pub fn record_escrow_completion(
        env: Env,
        caller: Address,
        pioneer: Address,
        as_seller: bool,
    ) -> u32 {
        require_coordinator(&env, &caller);
        require_not_paused(&env);

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&key).unwrap_or_else(|| {
            ReputationContract::create_profile(env.clone(), pioneer.clone())
        });

        profile.total_escrows += 1;
        profile.completed_escrows += 1;
        profile.last_active = env.ledger().timestamp();

        // Score boost: +5 for seller, +3 for buyer
        let boost: u32 = if as_seller { 5 } else { 3 };
        let new_score = clamp_score(profile.score + boost);
        let old_score = profile.score;
        profile.score = new_score;
        profile.tier = score_to_tier(new_score);
        profile.score_nonce += 1;

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("reputation_updated"), pioneer.clone()),
            (old_score, new_score),
        );

        new_score
    }

    /// Record escrow expiry (seller failed to deliver)
    pub fn record_escrow_expiry(
        env: Env,
        caller: Address,
        seller: Address,
    ) -> u32 {
        require_coordinator(&env, &caller);

        let key = profile_key(&seller);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&key).unwrap();

        profile.total_escrows += 1;
        profile.expired_escrows += 1;

        // Penalty: -15 for expired escrow
        let new_score = clamp_score(profile.score.saturating_sub(15));
        let old_score = profile.score;
        profile.score = new_score;
        profile.tier = score_to_tier(new_score);
        profile.score_nonce += 1;

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("reputation_updated"), seller.clone()),
            (old_score, new_score),
        );

        new_score
    }

    /// Record dispute ruling
    pub fn record_dispute_ruling(
        env: Env,
        caller: Address,
        pioneer: Address,
        ruling_in_favor: bool,
        as_seller: bool,
    ) -> u32 {
        require_coordinator(&env, &caller);

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&key).unwrap();

        if as_seller {
            profile.disputes_as_seller += 1;
        } else {
            profile.disputes_as_buyer += 1;
        }

        if ruling_in_favor {
            profile.rulings_in_favor += 1;
            let new_score = clamp_score(profile.score + 10);
            profile.score = new_score;
        } else {
            profile.rulings_against += 1;
            let new_score = clamp_score(profile.score.saturating_sub(20));
            profile.score = new_score;
        }

        profile.tier = score_to_tier(profile.score);
        profile.score_nonce += 1;

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("reputation_updated"), pioneer.clone()),
            (profile.score,),
        );

        profile.score
    }

    /// Set merchant verification status (called by coordinator)
    pub fn set_merchant_status(
        env: Env,
        caller: Address,
        pioneer: Address,
        is_verified: bool,
    ) {
        require_coordinator(&env, &caller);

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&key).unwrap();

        let was_verified = profile.is_verified_merchant;
        profile.is_verified_merchant = is_verified;

        // Score impact: +50 for verification, -50 for revocation
        if is_verified && !was_verified {
            profile.score = clamp_score(profile.score + 50);
        } else if !is_verified && was_verified {
            profile.score = clamp_score(profile.score.saturating_sub(50));
        }

        profile.tier = score_to_tier(profile.score);
        profile.score_nonce += 1;

        storage.set(&key, &profile);
    }

    /// Apply inactivity decay (can be called by anyone)
    pub fn apply_decay(env: Env, pioneer: Address) -> u32 {
        require_not_paused(&env);

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&key).unwrap();

        let now = env.ledger().timestamp();
        let inactive_seconds = now.saturating_sub(profile.last_active);

        // No decay for active within 30 days
        if inactive_seconds < 30 * 86400 {
            return profile.score;
        }

        // 1% decay per week of inactivity after 30 days
        let inactive_weeks = (inactive_seconds - (30 * 86400)) / (7 * 86400);
        let decay_amount = (profile.score as u64 * inactive_weeks as u64) / 100;
        let new_score = clamp_score(profile.score.saturating_sub(decay_amount as u32));

        profile.score = new_score;
        profile.tier = score_to_tier(new_score);
        profile.score_nonce += 1;

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("reputation_decayed"), pioneer.clone()),
            (new_score,),
        );

        new_score
    }

    /// Get full reputation profile
    pub fn get_profile(env: Env, pioneer: Address) -> ReputationProfile {
        let key = profile_key(&pioneer);
        env.storage().persistent().get(&key).unwrap()
    }

    /// Get score only (gas-efficient)
    pub fn get_score(env: Env, pioneer: Address) -> u32 {
        let key = profile_key(&pioneer);
        let profile: ReputationProfile = env.storage().persistent().get(&key).unwrap();
        profile.score
    }

    /// Get tier only
    pub fn get_tier(env: Env, pioneer: Address) -> ReputationTier {
        let key = profile_key(&pioneer);
        let profile: ReputationProfile = env.storage().persistent().get(&key).unwrap();
        profile.tier
    }

    /// Verify reputation meets threshold
    pub fn verify_threshold(env: Env, pioneer: Address, minimum_score: u32) -> bool {
        let key = profile_key(&pioneer);
        match env.storage().persistent().get::<_, ReputationProfile>(&key) {
            Some(profile) => profile.score >= minimum_score,
            None => false,
        }
    }

    // ========================================================================
    // v1.1: Soulbound Badge Functions
    // ========================================================================

    /// Award a soulbound badge to a Pioneer (called by coordinator)
    /// Badges are NON-TRANSFERABLE — they represent what you've DONE, not what you HOLD
    pub fn award_badge(
        env: Env,
        caller: Address,
        pioneer: Address,
        badge: SoulboundBadge,
        reason: BytesN<32>,
    ) {
        require_coordinator(&env, &caller);

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&key).unwrap();

        // Store badge ownership (soulbound — cannot be transferred)
        let badge_key = (Symbol::new("badge"), pioneer.clone(), badge.clone());
        let ownership = BadgeOwnership {
            pioneer: pioneer.clone(),
            badge: badge.clone(),
            awarded_at: env.ledger().timestamp(),
            award_reason: reason,
            revoked: false,
        };
        env.storage().persistent().set(&badge_key, &ownership);

        profile.badge_count += 1;
        // Badge bonus: +2 score per badge (capped at +20 total from badges)
        let badge_bonus = if profile.badge_count <= 10 { 2 } else { 0 };
        profile.score = clamp_score(profile.score + badge_bonus);
        profile.tier = score_to_tier(profile.score);
        profile.score_nonce += 1;

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("badge_awarded"), pioneer.clone()),
            (badge, badge_bonus),
        );
    }

    /// Revoke a soulbound badge (only for proven fraud/misconduct)
    pub fn revoke_badge(
        env: Env,
        caller: Address,
        pioneer: Address,
        badge: SoulboundBadge,
    ) {
        require_coordinator(&env, &caller);

        let badge_key = (Symbol::new("badge"), pioneer.clone(), badge.clone());
        let mut ownership: BadgeOwnership = env.storage().persistent().get(&badge_key).unwrap();
        assert!(!ownership.revoked, "Already revoked");
        ownership.revoked = true;
        env.storage().persistent().set(&badge_key, &ownership);

        // Penalty for badge revocation: -10 score
        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&key).unwrap();
        profile.badge_count = profile.badge_count.saturating_sub(1);
        profile.score = clamp_score(profile.score.saturating_sub(10));
        profile.tier = score_to_tier(profile.score);
        profile.score_nonce += 1;
        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("badge_revoked"), pioneer.clone()),
            (badge,),
        );
    }

    /// Check if a Pioneer holds a specific soulbound badge
    pub fn has_badge(env: Env, pioneer: Address, badge: SoulboundBadge) -> bool {
        let badge_key = (Symbol::new("badge"), pioneer, badge);
        match env.storage().persistent().get::<_, BadgeOwnership>(&badge_key) {
            Some(ownership) => !ownership.revoked,
            None => false,
        }
    }

    // ========================================================================
    // v1.1: Reputation Attestation Functions
    // ========================================================================

    /// Create an attestation — a verified Pioneer vouches for another
    /// Weight is derived from attester's tier (Bronze=1, Silver=2, Gold=3, Platinum=4, Diamond=5)
    pub fn create_attestation(
        env: Env,
        attester: Address,
        attested: Address,
        attestation_type: AttestationType,
    ) -> u64 {
        require_not_paused(&env);
        attester.require_auth();
        assert!(attester != attested, "Cannot self-attest");

        // Get attester's profile to determine weight
        let attester_key = profile_key(&attester);
        let attester_profile: ReputationProfile = env.storage().persistent()
            .get(&attester_key).unwrap_or_else(|| {
            panic!("Attester has no profile");
        });

        // Minimum Silver tier required to attest
        assert!(attester_profile.score >= 200, "Min Silver to attest");

        // Weight from tier: Bronze=1, Silver=2, Gold=3, Platinum=4, Diamond=5
        let weight: u32 = match attester_profile.tier {
            ReputationTier::Bronze => 1,
            ReputationTier::Silver => 2,
            ReputationTier::Gold => 3,
            ReputationTier::Platinum => 4,
            ReputationTier::Diamond => 5,
        };

        let next_att_id: u64 = env.storage().instance()
            .get(&Symbol::new("next_att_id")).unwrap_or(1);
        env.storage().instance().set(&Symbol::new("next_att_id"), &(next_att_id + 1));

        let attestation = Attestation {
            attestation_id: next_att_id,
            attester: attester.clone(),
            attested: attested.clone(),
            attestation_type,
            attester_reputation: attester_profile.score,
            weight,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 180 * 86400, // 180 days
            active: true,
        };

        // Store attestation
        let att_key = (Symbol::new("attestation"), next_att_id);
        env.storage().persistent().set(&att_key, &attestation);

        // Update attested profile's attestation_score
        let attested_key = profile_key(&attested);
        let storage = env.storage().persistent();
        let mut attested_profile: ReputationProfile = storage.get(&attested_key)
            .unwrap_or_else(|| ReputationContract::create_profile(env.clone(), attested.clone()));

        // Attestation score: weighted sum, capped at 100
        let new_att_score = (attested_profile.attestation_score + weight).min(100);
        let att_bonus = if new_att_score >= 20 && attested_profile.attestation_score < 20 {
            5u32 // +5 reputation bonus when reaching 20 attestation score
        } else {
            0u32
        };
        attested_profile.attestation_score = new_att_score;
        attested_profile.score = clamp_score(attested_profile.score + att_bonus);
        attested_profile.tier = score_to_tier(attested_profile.score);
        storage.set(&attested_key, &attested_profile);

        env.events().publish(
            (Symbol::new("attestation_created"), attested.clone()),
            (attester, weight, attestation_type),
        );

        next_att_id
    }

    /// Revoke an attestation (by attester or coordinator)
    pub fn revoke_attestation(env: Env, caller: Address, attestation_id: u64) {
        let att_key = (Symbol::new("attestation"), attestation_id);
        let mut attestation: Attestation = env.storage().persistent().get(&att_key).unwrap();
        assert!(attestation.active, "Already revoked");

        // Only attester or coordinator can revoke
        let coordinator: Address = env.storage().instance().get(&COORDINATOR).unwrap();
        assert!(caller == attestation.attester || caller == coordinator, "Not authorized");

        attestation.active = false;
        env.storage().persistent().set(&att_key, &attestation);

        // Reduce attested profile's attestation_score
        let attested_key = profile_key(&attestation.attested);
        let storage = env.storage().persistent();
        let mut profile: ReputationProfile = storage.get(&attested_key).unwrap();
        profile.attestation_score = profile.attestation_score.saturating_sub(attestation.weight);
        storage.set(&attested_key, &profile);

        env.events().publish(
            (Symbol::new("attestation_revoked"), attestation.attested.clone()),
            (attestation_id,),
        );
    }

    // ========================================================================
    // v1.1: Sybil Resistance — Social Graph Analysis
    // ========================================================================

    /// Update a Pioneer's Sybil profile based on transaction patterns
    /// Called by coordinator after each escrow completion
    pub fn update_sybil_profile(
        env: Env,
        caller: Address,
        pioneer: Address,
        new_counterparty: Address,
    ) {
        require_coordinator(&env, &caller);

        let sybil_key = (Symbol::new("sybil"), pioneer.clone());
        let storage = env.storage().persistent();

        let mut sybil = env.storage().persistent()
            .get::<_, SybilProfile>(&sybil_key)
            .unwrap_or(SybilProfile {
                pioneer: pioneer.clone(),
                unique_counterparties: 0,
                total_transactions: 0,
                reciprocal_ratio: 10000, // Start at 100% (neutral)
                avg_tx_interval: 0,
                cluster_flag: false,
                last_analysis: env.ledger().timestamp(),
                sybil_score: 0,
            });

        sybil.total_transactions += 1;

        // Track unique counterparties
        let cp_key = (Symbol::new("counterparty"), pioneer.clone(), new_counterparty.clone());
        if !env.storage().persistent().has(&cp_key) {
            env.storage().persistent().set(&cp_key, &true);
            sybil.unique_counterparties += 1;
        }

        // Sybil scoring heuristic:
        // - Accounts with few counterparties + many transactions = suspicious
        // - Ratio < 30% unique counterparties = high Sybil score
        if sybil.total_transactions > 5 {
            let unique_ratio = (sybil.unique_counterparties as u64 * 10000)
                / sybil.total_transactions as u64;
            if unique_ratio < 3000 {
                // Less than 30% unique = suspicious
                sybil.sybil_score = ((10000 - unique_ratio) as u32).min(10000);
                sybil.cluster_flag = true;
            } else {
                sybil.sybil_score = 0;
                sybil.cluster_flag = false;
            }
        }

        sybil.last_analysis = env.ledger().timestamp();

        // Update profile's sybil fields
        let profile_key_val = profile_key(&pioneer);
        let mut profile: ReputationProfile = storage.get(&profile_key_val).unwrap();
        profile.sybil_score = sybil.sybil_score;
        profile.unique_counterparties = sybil.unique_counterparties;

        // Sybil penalty: reduce effective score for high Sybil scores
        if sybil.sybil_score > 5000 {
            profile.score = clamp_score(profile.score.saturating_sub(5));
            profile.tier = score_to_tier(profile.score);
        }

        storage.set(&profile_key_val, &profile);
        env.storage().persistent().set(&sybil_key, &sybil);

        env.events().publish(
            (Symbol::new("sybil_updated"), pioneer.clone()),
            (sybil.sybil_score, sybil.unique_counterparties),
        );
    }

    /// Get Sybil profile for a Pioneer
    pub fn get_sybil_profile(env: Env, pioneer: Address) -> SybilProfile {
        let sybil_key = (Symbol::new("sybil"), pioneer);
        env.storage().persistent().get(&sybil_key).unwrap_or(SybilProfile {
            pioneer: Address::from_string(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            unique_counterparties: 0,
            total_transactions: 0,
            reciprocal_ratio: 0,
            avg_tx_interval: 0,
            cluster_flag: false,
            last_analysis: 0,
            sybil_score: 0,
        })
    }

    // ========================================================================
    // v1.1: ZK Reputation Verification (Roadmap)
    // ========================================================================

    /// Verify tier claim without revealing exact score (placeholder for ZK-SNARK integration)
    /// Currently does a simple on-chain verification; future version will use ZK proofs
    pub fn verify_tier_claim(
        env: Env,
        pioneer: Address,
        claimed_tier: ReputationTier,
    ) -> bool {
        let key = profile_key(&pioneer);
        let profile: ReputationProfile = env.storage().persistent().get(&key).unwrap();

        // Current: simple on-chain check
        // Future: ZK-SNARK proof that score >= tier_min without revealing score
        let actual_tier = score_to_tier(profile.score);
        let verified = actual_tier == claimed_tier;

        env.events().publish(
            (Symbol::new("tier_verified"), pioneer),
            (claimed_tier, verified),
        );

        verified
    }

    /// Get effective reputation score (adjusted for Sybil risk)
    pub fn get_effective_score(env: Env, pioneer: Address) -> u32 {
        let key = profile_key(&pioneer);
        let profile: ReputationProfile = env.storage().persistent().get(&key).unwrap();

        // Sybil penalty: reduce effective score proportionally
        if profile.sybil_score > 0 {
            let reduction = (profile.score as u64 * profile.sybil_score as u64) / 20000;
            profile.score.saturating_sub(reduction as u32)
        } else {
            profile.score
        }
    }

    /// Emergency pause
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
    }
}
