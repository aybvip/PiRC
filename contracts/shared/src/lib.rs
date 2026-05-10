#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Vec, Map, token};

// ============================================================================
// Shared Types
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowState {
    Created,
    Funded,
    Delivered,
    Completed,
    Disputed,
    Resolved,
    Expired,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeCategory {
    NonDelivery,
    NotAsDescribed,
    DamagedDefective,
    DeliveryDispute,
    ServiceNotProvided,
    UnauthorizedCharge,
    Other,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeRuling {
    FullRefund,
    PartialRefund,
    SellerFavored,
    Split,
    Dismissed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputePhase {
    Filed,
    Evidence,
    Voting,
    Ruling,
    Appealed,
    Final,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReputationTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationLevel {
    None,
    Basic,
    Standard,
    Premium,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoyaltyTier {
    Starter,
    Regular,
    Trusted,
    Elite,
    Legendary,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardType {
    FeeWaiver,
    JurorPriority,
    MerchantSpotlight,
    ReputationBoost,
    GovernanceVote,
}

// ============================================================================
// Data Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowAccount {
    pub escrow_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub amount: i128,
    pub token: Address,
    pub state: EscrowState,
    pub created_at: u64,
    pub delivery_deadline: u64,
    pub confirmation_deadline: u64,
    pub auto_release_timeout: u64,
    pub subscription_id: Option<u64>,
    pub order_metadata: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JurorVote {
    pub juror: Address,
    pub vote: DisputeRuling,
    pub confidence: u8,
    pub voted_at: u64,
    pub justification_hash: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCase {
    pub dispute_id: u64,
    pub escrow_id: u64,
    pub filer: Address,
    pub respondent: Address,
    pub category: DisputeCategory,
    pub phase: DisputePhase,
    pub jurors: Vec<Address>,
    pub votes: Vec<JurorVote>,
    pub filer_evidence: Vec<BytesN<32>>,
    pub respondent_evidence: Vec<BytesN<32>>,
    pub filed_at: u64,
    pub evidence_deadline: u64,
    pub voting_deadline: u64,
    pub ruling: Option<DisputeRuling>,
    pub is_appealed: bool,
    pub appeal_fee: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleAddresses {
    pub escrow: Address,
    pub reputation: Address,
    pub dispute: Address,
    pub merchant_verification: Address,
    pub loyalty: Address,
}

// ============================================================================
// Storage Keys
// ============================================================================

fn escrow_key(env: &Env, id: u64) -> Symbol {
    Symbol::new(env, &format!("escrow_{}", id))
}

fn reputation_key(env: &Env, addr: &Address) -> Symbol {
    Symbol::new(env, "rep")
}

fn dispute_key(env: &Env, id: u64) -> Symbol {
    Symbol::new(env, &format!("dispute_{}", id))
}

fn merchant_key(env: &Env, addr: &Address) -> Symbol {
    Symbol::new(env, "merchant")
}

fn loyalty_key(env: &Env, addr: &Address) -> Symbol {
    Symbol::new(env, "loyalty")
}
