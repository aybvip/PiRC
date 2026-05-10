#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Map, Vec};

// ============================================================================
// Reputation Contract — PiDCTP Module 2
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

    /// Emergency pause
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
    }
}
