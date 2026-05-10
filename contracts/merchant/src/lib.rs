#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Vec, Map};

// ============================================================================
// Merchant Verification Contract — PiDCTP Module 4
// ============================================================================

#[contract]
pub struct MerchantContract;

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VerificationLevel {
    None,
    Basic,
    Standard,
    Premium,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VerificationStatus {
    NotApplied,
    Pending,
    UnderReview,
    InfoRequested,
    Approved,
    Suspended,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum MerchantCategory {
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

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct MerchantProfile {
    pub merchant: Address,
    pub level: VerificationLevel,
    pub business_name_hash: BytesN<32>,
    pub category: MerchantCategory,
    pub status: VerificationStatus,
    pub jurisdiction: BytesN<2>,
    pub total_volume: i128,
    pub total_orders: u32,
    pub avg_rating: u32,
    pub verified_at: Option<u64>,
    pub expires_at: Option<u64>,
    pub location_count: u32,
    pub metadata_uri: BytesN<32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct MerchantRating {
    pub buyer: Address,
    pub escrow_id: u64,
    pub rating: u32,       // 50-500 (0.5-5.0 stars)
    pub review_hash: BytesN<32>,
    pub rated_at: u64,
}

const COORDINATOR: Symbol = Symbol::new("coordinator");
const PAUSED: Symbol = Symbol::new("paused");
const AGENTS: Symbol = Symbol::new("agents");
const VERIFICATION_FEES: Symbol = Symbol::new("vfees");
const RENEWAL_PERIOD: Symbol = Symbol::new("renewal_period");

fn merchant_key(merchant: &Address) -> Symbol {
    Symbol::new("merchant")
}

fn rating_key(merchant: &Address, escrow_id: u64) -> Symbol {
    Symbol::new(&format!("rate_{}_{}", merchant.to_string(), escrow_id))
}

fn require_coordinator(env: &Env, caller: &Address) {
    let coordinator: Address = env.storage().instance().get(&COORDINATOR).unwrap();
    assert!(&caller == &coordinator, "Only coordinator");
}

fn require_not_paused(env: &Env) {
    let paused: bool = env.storage().instance().get(&PAUSED).unwrap_or(false);
    assert!(!paused, "Protocol paused");
}

fn require_agent(env: &Env, caller: &Address) {
    let agents: Map<Address, bool> = env.storage().instance().get(&AGENTS).unwrap_or(Map::new(env));
    assert!(agents.contains_key(caller.clone()), "Not an agent");
}

fn verification_fee(level: &VerificationLevel) -> i128 {
    match level {
        VerificationLevel::Basic => 2_000_000_000i128,    // 2 Pi
        VerificationLevel::Standard => 5_000_000_000i128, // 5 Pi
        VerificationLevel::Premium => 10_000_000_000i128, // 10 Pi
        VerificationLevel::None => 0,
    }
}

#[contractimpl]
impl MerchantContract {
    /// Initialize merchant verification contract
    pub fn initialize(env: Env, coordinator: Address) {
        let storage = env.storage().instance();
        assert!(!storage.has(&COORDINATOR), "Already initialized");
        storage.set(&COORDINATOR, &coordinator);
        storage.set(&PAUSED, &false);
        storage.set(&AGENTS, &Map::<Address, bool>::new(&env));
        storage.set(&RENEWAL_PERIOD, &31536000u64); // 1 year
    }

    /// Add a verification agent
    pub fn add_agent(env: Env, caller: Address, agent: Address) {
        require_coordinator(&env, &caller);
        let mut agents: Map<Address, bool> = env.storage().instance().get(&AGENTS).unwrap();
        agents.set(agent.clone(), true);
        env.storage().instance().set(&AGENTS, &agents);

        env.events().publish(
            (Symbol::new("agent_added"), agent),
            (),
        );
    }

    /// Remove a verification agent
    pub fn remove_agent(env: Env, caller: Address, agent: Address) {
        require_coordinator(&env, &caller);
        let mut agents: Map<Address, bool> = env.storage().instance().get(&AGENTS).unwrap();
        agents.remove(agent.clone());
        env.storage().instance().set(&AGENTS, &agents);
    }

    /// Apply for merchant verification
    pub fn apply_verification(
        env: Env,
        merchant: Address,
        level: VerificationLevel,
        business_name_hash: BytesN<32>,
        category: MerchantCategory,
        jurisdiction: BytesN<2>,
        metadata_uri: BytesN<32>,
    ) {
        require_not_paused(&env);
        merchant.require_auth();

        let key = merchant_key(&merchant);
        let storage = env.storage().persistent();

        // Check not already verified at this level or higher
        if storage.has(&key) {
            let existing: MerchantProfile = storage.get(&key).unwrap();
            assert!(
                existing.status == VerificationStatus::Revoked
                    || existing.status == VerificationStatus::Expired
                    || existing.status == VerificationStatus::NotApplied,
                "Already verified or pending"
            );
        }

        let profile = MerchantProfile {
            merchant: merchant.clone(),
            level,
            business_name_hash,
            category,
            status: VerificationStatus::Pending,
            jurisdiction,
            total_volume: 0,
            total_orders: 0,
            avg_rating: 0,
            verified_at: None,
            expires_at: None,
            location_count: 0,
            metadata_uri,
        };

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("merchant_applied"), merchant.clone()),
            (level,),
        );
    }

    /// Move application to UnderReview (agent picks up the application)
    pub fn start_review(env: Env, agent: Address, merchant: Address) {
        require_agent(&env, &agent);

        let key = merchant_key(&merchant);
        let storage = env.storage().persistent();
        let mut profile: MerchantProfile = storage.get(&key).unwrap();
        assert!(profile.status == VerificationStatus::Pending, "Not pending");

        profile.status = VerificationStatus::UnderReview;
        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("review_started"), merchant),
            (agent,),
        );
    }

    /// Request additional information from merchant
    pub fn request_info(env: Env, agent: Address, merchant: Address) {
        require_agent(&env, &agent);

        let key = merchant_key(&merchant);
        let storage = env.storage().persistent();
        let mut profile: MerchantProfile = storage.get(&key).unwrap();
        assert!(profile.status == VerificationStatus::UnderReview, "Not under review");

        profile.status = VerificationStatus::InfoRequested;
        storage.set(&key, &profile);
    }

    /// Approve merchant verification
    pub fn approve_verification(
        env: Env,
        agent: Address,
        merchant: Address,
        level: VerificationLevel,
    ) {
        require_agent(&env, &agent);

        let key = merchant_key(&merchant);
        let storage = env.storage().persistent();
        let mut profile: MerchantProfile = storage.get(&key).unwrap();
        assert!(profile.status == VerificationStatus::UnderReview, "Not under review");

        let now = env.ledger().timestamp();
        let renewal_period: u64 = env.storage().instance().get(&RENEWAL_PERIOD).unwrap_or(31536000);

        profile.level = level;
        profile.status = VerificationStatus::Approved;
        profile.verified_at = Some(now);
        profile.expires_at = Some(now + renewal_period);

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("merchant_approved"), merchant),
            (level, now),
        );
    }

    /// Suspend a merchant
    pub fn suspend_merchant(
        env: Env,
        agent: Address,
        merchant: Address,
        _reason_hash: BytesN<32>,
    ) {
        require_agent(&env, &agent);

        let key = merchant_key(&merchant);
        let storage = env.storage().persistent();
        let mut profile: MerchantProfile = storage.get(&key).unwrap();
        assert!(profile.status == VerificationStatus::Approved, "Not approved");

        profile.status = VerificationStatus::Suspended;
        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("merchant_suspended"), merchant),
            (),
        );
    }

    /// Revoke merchant verification permanently
    pub fn revoke_verification(
        env: Env,
        agent: Address,
        merchant: Address,
        _reason_hash: BytesN<32>,
    ) {
        require_agent(&env, &agent);

        let key = merchant_key(&merchant);
        let storage = env.storage().persistent();
        let mut profile: MerchantProfile = storage.get(&key).unwrap();
        assert!(
            profile.status == VerificationStatus::Approved
                || profile.status == VerificationStatus::Suspended,
            "Cannot revoke"
        );

        profile.status = VerificationStatus::Revoked;
        profile.level = VerificationLevel::None;
        profile.verified_at = None;
        profile.expires_at = None;

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("merchant_revoked"), merchant),
            (),
        );
    }

    /// Renew merchant verification
    pub fn renew_verification(
        env: Env,
        merchant: Address,
        metadata_uri: BytesN<32>,
    ) {
        require_not_paused(&env);
        merchant.require_auth();

        let key = merchant_key(&merchant);
        let storage = env.storage().persistent();
        let mut profile: MerchantProfile = storage.get(&key).unwrap();

        let now = env.ledger().timestamp();
        let expires_at = profile.expires_at.unwrap_or(0);

        // Must be within 30 days of expiration or expired within 90 days
        assert!(
            now >= expires_at.saturating_sub(30 * 86400)
                && now <= expires_at.saturating_add(90 * 86400),
            "Not in renewal window"
        );

        let renewal_period: u64 = env.storage().instance().get(&RENEWAL_PERIOD).unwrap_or(31536000);

        profile.status = VerificationStatus::Pending;
        profile.metadata_uri = metadata_uri;
        profile.expires_at = Some(now + renewal_period);

        storage.set(&key, &profile);

        env.events().publish(
            (Symbol::new("merchant_renewal"), merchant),
            (),
        );
    }

    /// Rate a merchant after completed escrow
    pub fn rate_merchant(
        env: Env,
        buyer: Address,
        merchant: Address,
        escrow_id: u64,
        rating: u32,
        review_hash: BytesN<32>,
    ) {
        require_not_paused(&env);
        buyer.require_auth();

        assert!(rating >= 50 && rating <= 500, "Invalid rating");

        // Check not already rated
        let rkey = rating_key(&merchant, escrow_id);
        let storage = env.storage().persistent();
        assert!(!storage.has(&rkey), "Already rated");

        let merchant_rating = MerchantRating {
            buyer,
            escrow_id,
            rating,
            review_hash: review_hash.clone(),
            rated_at: env.ledger().timestamp(),
        };

        storage.set(&rkey, &merchant_rating);

        // Update merchant's average rating (exponential moving average)
        let mkey = merchant_key(&merchant);
        if storage.has(&mkey) {
            let mut profile: MerchantProfile = storage.get(&mkey).unwrap();
            if profile.avg_rating == 0 {
                profile.avg_rating = rating;
            } else {
                // EMA with alpha=0.3
                let alpha_numerator = 3;
                let alpha_denominator = 10;
                let old = profile.avg_rating as u64;
                let new_val = rating as u64;
                let ema = (alpha_numerator * new_val + (alpha_denominator - alpha_numerator) * old)
                    / alpha_denominator;
                profile.avg_rating = ema as u32;
            }
            profile.total_orders += 1;
            storage.set(&mkey, &profile);
        }

        env.events().publish(
            (Symbol::new("merchant_rated"), merchant),
            (rating, escrow_id),
        );
    }

    /// Get merchant profile
    pub fn get_profile(env: Env, merchant: Address) -> MerchantProfile {
        let key = merchant_key(&merchant);
        env.storage().persistent().get(&key).unwrap()
    }

    /// Check if merchant is verified
    pub fn is_verified(env: Env, merchant: Address) -> bool {
        let key = merchant_key(&merchant);
        match env.storage().persistent().get::<_, MerchantProfile>(&key) {
            Some(profile) => profile.status == VerificationStatus::Approved,
            None => false,
        }
    }

    /// Emergency pause
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
    }
}
