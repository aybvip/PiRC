#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Map, Vec};

// ============================================================================
// Loyalty & Reward Contract — PiDCTP Module 5
// ============================================================================

#[contract]
pub struct LoyaltyContract;

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum LoyaltyTier {
    Starter,    // 0 points
    Regular,    // 100 points
    Trusted,    // 500 points
    Elite,      // 2,000 points
    Legendary,  // 10,000 points
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum RewardType {
    FeeWaiver,
    JurorPriority,
    MerchantSpotlight,
    ReputationBoost,
    GovernanceVote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct LoyaltyProfile {
    pub pioneer: Address,
    pub points: u32,
    pub tier: LoyaltyTier,
    pub lifetime_points: u32,
    pub redeemable_points: u32,
    pub last_activity: u64,
    pub referral_code: BytesN<32>,
    pub referral_count: u32,
    pub activity_streak: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct TierBenefits {
    pub tier: LoyaltyTier,
    pub fee_discount_bps: u32,  // 0-7500 (0%-75%)
    pub juror_weight: u32,      // 100=1.0x, 200=2.0x
}

const COORDINATOR: Symbol = Symbol::new("coordinator");
const PAUSED: Symbol = Symbol::new("paused");
const LOYALTY_POOL: Symbol = Symbol::new("loyalty_pool");
const MAX_POOL: Symbol = Symbol::new("max_pool");
const REP_BOOST_QUOTA: Symbol = Symbol::new("rep_boost_quota");

// Tier thresholds
const REGULAR_THRESHOLD: u32 = 100;
const TRUSTED_THRESHOLD: u32 = 500;
const ELITE_THRESHOLD: u32 = 2000;
const LEGENDARY_THRESHOLD: u32 = 10000;

fn profile_key(pioneer: &Address) -> (Symbol, Address) {
    (Symbol::new("loyalty"), pioneer.clone())
}

fn points_to_tier(lifetime_points: u32) -> LoyaltyTier {
    if lifetime_points >= LEGENDARY_THRESHOLD {
        LoyaltyTier::Legendary
    } else if lifetime_points >= ELITE_THRESHOLD {
        LoyaltyTier::Elite
    } else if lifetime_points >= TRUSTED_THRESHOLD {
        LoyaltyTier::Trusted
    } else if lifetime_points >= REGULAR_THRESHOLD {
        LoyaltyTier::Regular
    } else {
        LoyaltyTier::Starter
    }
}

fn require_coordinator(env: &Env, caller: &Address) {
    let coordinator: Address = env.storage().instance().get(&COORDINATOR).unwrap();
    assert!(&caller == &coordinator, "Only coordinator");
}

fn require_not_paused(env: &Env) {
    let paused: bool = env.storage().instance().get(&PAUSED).unwrap_or(false);
    assert!(!paused, "Protocol paused");
}

fn get_tier_benefits(tier: &LoyaltyTier) -> TierBenefits {
    match tier {
        LoyaltyTier::Starter => TierBenefits {
            tier: LoyaltyTier::Starter,
            fee_discount_bps: 0,
            juror_weight: 100,
        },
        LoyaltyTier::Regular => TierBenefits {
            tier: LoyaltyTier::Regular,
            fee_discount_bps: 1000, // 10%
            juror_weight: 100,
        },
        LoyaltyTier::Trusted => TierBenefits {
            tier: LoyaltyTier::Trusted,
            fee_discount_bps: 2500, // 25%
            juror_weight: 120,
        },
        LoyaltyTier::Elite => TierBenefits {
            tier: LoyaltyTier::Elite,
            fee_discount_bps: 5000, // 50%
            juror_weight: 150,
        },
        LoyaltyTier::Legendary => TierBenefits {
            tier: LoyaltyTier::Legendary,
            fee_discount_bps: 7500, // 75%
            juror_weight: 200,
        },
    }
}

#[contractimpl]
impl LoyaltyContract {
    /// Initialize loyalty contract
    pub fn initialize(env: Env, coordinator: Address) {
        let storage = env.storage().instance();
        assert!(!storage.has(&COORDINATOR), "Already initialized");
        storage.set(&COORDINATOR, &coordinator);
        storage.set(&PAUSED, &false);
        storage.set(&LOYALTY_POOL, &0i128);
        storage.set(&MAX_POOL, &100_000_000_000_000i128); // 100K Pi in stroops
    }

    /// Create a new loyalty profile
    pub fn create_profile(env: Env, pioneer: Address) -> LoyaltyProfile {
        require_not_paused(&env);
        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        assert!(!storage.has(&key), "Profile exists");

        let profile = LoyaltyProfile {
            pioneer: pioneer.clone(),
            points: 0,
            tier: LoyaltyTier::Starter,
            lifetime_points: 0,
            redeemable_points: 0,
            last_activity: env.ledger().timestamp(),
            referral_code: BytesN::from_array(&env, &[0u8; 32]),
            referral_count: 0,
            activity_streak: 0,
        };

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("loyalty_created"), pioneer),
            (),
        );

        profile
    }

    /// Award points to a Pioneer (called by coordinator)
    pub fn award_points(
        env: Env,
        caller: Address,
        pioneer: Address,
        points: u32,
        action: Symbol,
    ) -> u32 {
        require_coordinator(&env, &caller);

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: LoyaltyProfile = storage
            .get(&key)
            .unwrap_or_else(|| LoyaltyContract::create_profile(env.clone(), pioneer.clone()));

        let old_tier = profile.tier.clone();
        profile.points += points;
        profile.redeemable_points += points;
        profile.lifetime_points += points;
        profile.last_activity = env.ledger().timestamp();
        profile.tier = points_to_tier(profile.lifetime_points);

        storage.set(&key, &profile);

        if profile.tier != old_tier {
            env.events().publish(
                (Symbol::new("loyalty_tier_changed"), pioneer.clone()),
                (old_tier, profile.tier.clone()),
            );
        }

        env.events().publish(
            (Symbol::new("loyalty_earned"), pioneer),
            (points, action),
        );

        profile.lifetime_points
    }

    /// Deduct points (penalty)
    pub fn deduct_points(
        env: Env,
        caller: Address,
        pioneer: Address,
        points: u32,
    ) -> u32 {
        require_coordinator(&env, &caller);

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: LoyaltyProfile = storage.get(&key).unwrap();

        profile.points = profile.points.saturating_sub(points);
        profile.redeemable_points = profile.redeemable_points.saturating_sub(points);
        // Note: lifetime_points never decrease
        profile.tier = points_to_tier(profile.lifetime_points);

        storage.set(&key, &profile);

        profile.points
    }

    /// Redeem points for a reward
    pub fn redeem_points(
        env: Env,
        pioneer: Address,
        reward: RewardType,
        amount: u32,
    ) {
        require_not_paused(&env);
        pioneer.require_auth();

        let cost = match reward {
            RewardType::FeeWaiver => 50,
            RewardType::JurorPriority => 100,
            RewardType::MerchantSpotlight => 200,
            RewardType::ReputationBoost => 500,
            RewardType::GovernanceVote => 1000,
        };

        let total_cost = cost * amount;

        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let mut profile: LoyaltyProfile = storage.get(&key).unwrap();

        assert!(profile.redeemable_points >= total_cost, "Insufficient points");

        // Special check: Reputation boost has quarterly quota
        if reward == RewardType::ReputationBoost {
            let quota_key = (Symbol::new("rep_boost"), pioneer.clone());
            let used: u32 = env.storage().persistent().get(&quota_key).unwrap_or(0);
            assert!(used < 1, "Quarterly quota exceeded");
            env.storage().persistent().set(&quota_key, &(used + 1));
        }

        profile.redeemable_points -= total_cost;
        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("loyalty_redeemed"), pioneer),
            (reward, total_cost),
        );
    }

    /// Apply fee discount based on loyalty tier (called by coordinator)
    pub fn apply_fee_discount(
        env: Env,
        pioneer: Address,
        base_fee: i128,
    ) -> i128 {
        let key = profile_key(&pioneer);
        let storage = env.storage().persistent();
        let profile: LoyaltyProfile = storage.get(&key).unwrap_or(LoyaltyProfile {
            pioneer: pioneer.clone(),
            points: 0,
            tier: LoyaltyTier::Starter,
            lifetime_points: 0,
            redeemable_points: 0,
            last_activity: 0,
            referral_code: BytesN::from_array(&env, &[0u8; 32]),
            referral_count: 0,
            activity_streak: 0,
        });

        let benefits = get_tier_benefits(&profile.tier);
        let discount = (base_fee * (benefits.fee_discount_bps as i128)) / 10000;
        base_fee - discount
    }

    /// Get loyalty profile
    pub fn get_profile(env: Env, pioneer: Address) -> LoyaltyProfile {
        let key = profile_key(&pioneer);
        env.storage().persistent().get(&key).unwrap()
    }

    /// Get tier benefits
    pub fn get_tier_benefits(env: Env, tier: LoyaltyTier) -> TierBenefits {
        get_tier_benefits(&tier)
    }

    /// Add funds to loyalty pool
    pub fn add_to_pool(env: Env, caller: Address, amount: i128) {
        require_coordinator(&env, &caller);
        let mut pool: i128 = env.storage().instance().get(&LOYALTY_POOL).unwrap_or(0);
        pool += amount;
        let max_pool: i128 = env.storage().instance().get(&MAX_POOL).unwrap_or(100_000_000_000_000);
        assert!(pool <= max_pool, "Pool cap exceeded");
        env.storage().instance().set(&LOYALTY_POOL, &pool);
    }

    /// Emergency pause
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
    }
}
