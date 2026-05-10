#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Vec, Map};

// ============================================================================
// Dispute Resolution Contract — PiDCTP Module 3
// + Juror Vetting, Reputation-Weighted Voting (v1.1)
// ============================================================================

#[contract]
pub struct DisputeContract;

// ============================================================================
// v1.1: Juror Vetting
// Jurors must meet minimum reputation and have relevant expertise
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum JurorSpecialty {
    General,            // No specialty — can serve on any case
    Commerce,           // Specializes in commerce/trade disputes
    DigitalGoods,       // Specializes in digital product disputes
    Services,           // Specializes in service disputes
    Subscription,       // Specializes in PiRC2 subscription disputes
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct JurorVettingProfile {
    pub juror: Address,
    pub reputation_score: u32,      // Current reputation score
    pub cases_served: u32,          // Total disputes served as juror
    pub cases_consensus: u32,       // Times juror voted with majority
    pub consensus_rate: u32,        // cases_consensus/cases_served * 10000
    pub specialty: JurorSpecialty,
    pub active: bool,               // Currently available for selection
    pub stake: i128,                // Pi staked as juror bond
    pub last_served: u64,
    pub penalty_points: u32,        // Accumulated penalties for non-participation
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DisputeCategory {
    NonDelivery,
    NotAsDescribed,
    DamagedDefective,
    DeliveryDispute,
    ServiceNotProvided,
    UnauthorizedCharge,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DisputePhase {
    Filed,
    Evidence,
    Voting,
    Ruling,
    Appealed,
    Final,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DisputeRuling {
    FullRefund,
    PartialRefund,
    SellerFavored,
    Split,
    Dismissed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct JurorVote {
    pub juror: Address,
    pub vote: DisputeRuling,
    pub confidence: u8,
    pub voted_at: u64,
    pub justification_hash: BytesN<32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct CommitVote {
    pub commitment: BytesN<32>,
    pub revealed: bool,
    pub actual_vote: Option<DisputeRuling>,
    pub salt: Option<BytesN<32>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct DisputeCase {
    pub dispute_id: u64,
    pub escrow_id: u64,
    pub filer: Address,
    pub respondent: Address,
    pub category: DisputeCategory,
    pub phase: DisputePhase,
    pub jurors: Vec<Address>,
    pub commit_votes: Map<Address, CommitVote>,
    pub filer_evidence: Vec<BytesN<32>>,
    pub respondent_evidence: Vec<BytesN<32>>,
    pub filed_at: u64,
    pub evidence_deadline: u64,
    pub voting_deadline: u64,
    pub reveal_deadline: u64,
    pub ruling: Option<DisputeRuling>,
    pub is_appealed: bool,
    pub appeal_fee: i128,
    pub juror_count: u32,
}

const COORDINATOR: Symbol = Symbol::new("coordinator");
const PAUSED: Symbol = Symbol::new("paused");
const NEXT_DISPUTE_ID: Symbol = Symbol::new("next_id");
const JUROR_POOL: Symbol = Symbol::new("juror_pool");
const DISPUTE_FEE: Symbol = Symbol::new("dispute_fee");
const APPEAL_FEE: Symbol = Symbol::new("appeal_fee");
const EVIDENCE_DURATION: Symbol = Symbol::new("evidence_dur");
const VOTING_DURATION: Symbol = Symbol::new("voting_dur");
const REVEAL_DURATION: Symbol = Symbol::new("reveal_dur");
const APPEAL_WINDOW: Symbol = Symbol::new("appeal_window");

fn dispute_key(id: u64) -> Symbol {
    Symbol::new(&format!("d_{}", id))
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
impl DisputeContract {
    /// Initialize dispute contract
    pub fn initialize(env: Env, coordinator: Address) {
        let storage = env.storage().instance();
        assert!(!storage.has(&COORDINATOR), "Already initialized");
        storage.set(&COORDINATOR, &coordinator);
        storage.set(&PAUSED, &false);
        storage.set(&NEXT_DISPUTE_ID, &1u64);
        storage.set(&DISPUTE_FEE, &1_000_000_000i128); // 1 Pi in stroops (assuming 7 decimals: 10M stroops = 1 Pi)
        storage.set(&APPEAL_FEE, &2_000_000_000i128);  // 2 Pi
        storage.set(&EVIDENCE_DURATION, &259200u64);     // 72 hours
        storage.set(&VOTING_DURATION, &172800u64);       // 48 hours
        storage.set(&REVEAL_DURATION, &86400u64);        // 24 hours
        storage.set(&APPEAL_WINDOW, &86400u64);          // 24 hours
    }

    /// Open a new dispute
    pub fn open_dispute(
        env: Env,
        caller: Address,
        escrow_id: u64,
        filer: Address,
        respondent: Address,
        category: DisputeCategory,
        initial_evidence: BytesN<32>,
        jurors: Vec<Address>,
    ) -> u64 {
        require_coordinator(&env, &caller);
        require_not_paused(&env);

        let next_id: u64 = env.storage().instance().get(&NEXT_DISPUTE_ID).unwrap_or(1);
        env.storage().instance().set(&NEXT_DISPUTE_ID, &(next_id + 1));

        let now = env.ledger().timestamp();
        let evidence_dur: u64 = env.storage().instance().get(&EVIDENCE_DURATION).unwrap_or(259200);
        let voting_dur: u64 = env.storage().instance().get(&VOTING_DURATION).unwrap_or(172800);
        let reveal_dur: u64 = env.storage().instance().get(&REVEAL_DURATION).unwrap_or(86400);

        let dispute = DisputeCase {
            dispute_id: next_id,
            escrow_id,
            filer: filer.clone(),
            respondent: respondent.clone(),
            category,
            phase: DisputePhase::Evidence,
            jurors,
            commit_votes: Map::new(&env),
            filer_evidence: Vec::from_array(&env, [initial_evidence]),
            respondent_evidence: Vec::new(&env),
            filed_at: now,
            evidence_deadline: now + evidence_dur,
            voting_deadline: now + evidence_dur + voting_dur,
            reveal_deadline: now + evidence_dur + voting_dur + reveal_dur,
            ruling: None,
            is_appealed: false,
            appeal_fee: 0,
            juror_count: 3,
        };

        env.storage().persistent().set(&dispute_key(next_id), &dispute);

        env.events().publish(
            (Symbol::new("dispute_opened"), next_id),
            (filer, respondent, escrow_id),
        );

        next_id
    }

    /// Submit evidence for a dispute
    pub fn submit_evidence(
        env: Env,
        party: Address,
        dispute_id: u64,
        evidence_hash: BytesN<32>,
    ) {
        require_not_paused(&env);
        party.require_auth();

        let mut dispute: DisputeCase = env.storage().persistent().get(&dispute_key(dispute_id)).unwrap();
        assert!(dispute.phase == DisputePhase::Evidence, "Not evidence phase");
        assert!(env.ledger().timestamp() <= dispute.evidence_deadline, "Deadline passed");

        if party == dispute.filer {
            assert!(dispute.filer_evidence.len() < 5, "Evidence limit");
            dispute.filer_evidence.push_back(evidence_hash);
        } else if party == dispute.respondent {
            assert!(dispute.respondent_evidence.len() < 5, "Evidence limit");
            dispute.respondent_evidence.push_back(evidence_hash);
        } else {
            panic!("Not a party");
        }

        env.storage().persistent().set(&dispute_key(dispute_id), &dispute);

        env.events().publish(
            (Symbol::new("evidence_submitted"), dispute_id),
            (party,),
        );
    }

    /// Transition to voting phase (called automatically or by coordinator)
    pub fn start_voting(env: Env, caller: Address, dispute_id: u64) {
        require_coordinator(&env, &caller);

        let mut dispute: DisputeCase = env.storage().persistent().get(&dispute_key(dispute_id)).unwrap();
        assert!(dispute.phase == DisputePhase::Evidence, "Not evidence phase");
        assert!(env.ledger().timestamp() > dispute.evidence_deadline, "Evidence not ended");

        dispute.phase = DisputePhase::Voting;
        env.storage().persistent().set(&dispute_key(dispute_id), &dispute);

        env.events().publish(
            (Symbol::new("voting_started"), dispute_id),
            (dispute.jurors.len() as u32,),
        );
    }

    /// Juror commits a vote (hash of vote + salt)
    pub fn commit_vote(
        env: Env,
        juror: Address,
        dispute_id: u64,
        commitment: BytesN<32>,
    ) {
        require_not_paused(&env);
        juror.require_auth();

        let mut dispute: DisputeCase = env.storage().persistent().get(&dispute_key(dispute_id)).unwrap();
        assert!(dispute.phase == DisputePhase::Voting, "Not voting phase");
        assert!(env.ledger().timestamp() <= dispute.voting_deadline, "Voting ended");

        // Verify juror is selected
        let mut is_juror = false;
        for j in dispute.jurors.iter() {
            if j == juror {
                is_juror = true;
                break;
            }
        }
        assert!(is_juror, "Not a juror");

        // Check not already committed
        assert!(!dispute.commit_votes.contains_key(juror.clone()), "Already committed");

        let commit = CommitVote {
            commitment,
            revealed: false,
            actual_vote: None,
            salt: None,
        };

        dispute.commit_votes.set(juror.clone(), commit);
        env.storage().persistent().set(&dispute_key(dispute_id), &dispute);

        env.events().publish(
            (Symbol::new("vote_committed"), dispute_id),
            (juror,),
        );
    }

    /// Juror reveals their vote
    pub fn reveal_vote(
        env: Env,
        juror: Address,
        dispute_id: u64,
        vote: DisputeRuling,
        salt: BytesN<32>,
    ) {
        require_not_paused(&env);
        juror.require_auth();

        let mut dispute: DisputeCase = env.storage().persistent().get(&dispute_key(dispute_id)).unwrap();
        assert!(
            dispute.phase == DisputePhase::Voting || dispute.phase == DisputePhase::Ruling,
            "Not in reveal phase"
        );
        assert!(env.ledger().timestamp() > dispute.voting_deadline, "Voting not ended");

        let mut commit = dispute.commit_votes.get(juror.clone()).unwrap();
        assert!(!commit.revealed, "Already revealed");

        // In production, verify: hash(vote + salt) == commitment
        // For now, trust the reveal matches the commitment
        commit.revealed = true;
        commit.actual_vote = Some(vote.clone());
        commit.salt = Some(salt);

        dispute.commit_votes.set(juror.clone(), commit);

        // Check if all jurors have revealed
        let all_revealed = are_all_revealed(&dispute);
        if all_revealed {
            dispute.phase = DisputePhase::Ruling;
        }

        env.storage().persistent().set(&dispute_key(dispute_id), &dispute);

        env.events().publish(
            (Symbol::new("vote_revealed"), dispute_id),
            (juror,),
        );
    }

    /// Execute the ruling based on majority vote
    pub fn execute_ruling(
        env: Env,
        caller: Address,
        dispute_id: u64,
    ) -> (DisputeRuling, u32) {
        require_coordinator(&env, &caller);

        let mut dispute: DisputeCase = env.storage().persistent().get(&dispute_key(dispute_id)).unwrap();
        assert!(dispute.phase == DisputePhase::Ruling, "Not ruling phase");

        // Count votes
        let mut vote_counts: Map<DisputeRuling, u32> = Map::new(&env);
        let mut total_revealed: u32 = 0;

        for (_, commit) in dispute.commit_votes.iter() {
            if commit.revealed {
                if let Some(ref vote) = commit.actual_vote {
                    let count = vote_counts.get(vote.clone()).unwrap_or(0);
                    vote_counts.set(vote.clone(), count + 1);
                    total_revealed += 1;
                }
            }
        }

        assert!(total_revealed > 0, "No votes revealed");

        // Find majority ruling
        let mut winning_ruling = DisputeRuling::Dismissed;
        let mut max_votes: u32 = 0;
        for (ruling, count) in vote_counts.iter() {
            if count > max_votes {
                max_votes = count;
                winning_ruling = ruling;
            }
        }

        dispute.ruling = Some(winning_ruling.clone());
        dispute.phase = DisputePhase::Final;
        env.storage().persistent().set(&dispute_key(dispute_id), &dispute);

        env.events().publish(
            (Symbol::new("dispute_resolved"), dispute_id),
            (winning_ruling.clone(), max_votes),
        );

        (winning_ruling, max_votes)
    }

    /// Get dispute details
    pub fn get_dispute(env: Env, dispute_id: u64) -> DisputeCase {
        env.storage().persistent().get(&dispute_key(dispute_id)).unwrap()
    }

    // ========================================================================
    // v1.1: Juror Vetting Functions
    // ========================================================================

    /// Register as a potential juror (requires minimum Silver reputation + stake)
    pub fn register_juror(
        env: Env,
        juror: Address,
        specialty: JurorSpecialty,
        reputation_score: u32,
        stake: i128,
    ) {
        require_not_paused(&env);
        juror.require_auth();

        // Minimum requirements: Silver tier (200+) and 10 Pi stake
        assert!(reputation_score >= 200, "Min Silver reputation");
        assert!(stake >= 1_000_000_000, "Min 10 Pi stake"); // 10 Pi in stroops

        let vetting_key = (Symbol::new("juror_vet"), juror.clone());
        assert!(
            !env.storage().persistent().has(&vetting_key),
            "Already registered"
        );

        let profile = JurorVettingProfile {
            juror: juror.clone(),
            reputation_score,
            cases_served: 0,
            cases_consensus: 0,
            consensus_rate: 10000, // Start at 100%
            specialty,
            active: true,
            stake,
            last_served: 0,
            penalty_points: 0,
        };

        env.storage().persistent().set(&vetting_key, &profile);

        env.events().publish(
            (Symbol::new("juror_registered"), juror),
            (specialty, reputation_score, stake),
        );
    }

    /// Deactivate juror (stop being selected for new cases)
    pub fn deactivate_juror(env: Env, juror: Address) {
        require_not_paused(&env);
        juror.require_auth();

        let vetting_key = (Symbol::new("juror_vet"), juror.clone());
        let mut profile: JurorVettingProfile = env.storage().persistent().get(&vetting_key).unwrap();
        profile.active = false;
        env.storage().persistent().set(&vetting_key, &profile);

        env.events().publish(
            (Symbol::new("juror_deactivated"), juror),
            (),
        );
    }

    /// Get juror vetting profile
    pub fn get_juror_profile(env: Env, juror: Address) -> JurorVettingProfile {
        let vetting_key = (Symbol::new("juror_vet"), juror);
        env.storage().persistent().get(&vetting_key).unwrap()
    }

    /// Check if a juror is eligible for a specific dispute category
    pub fn is_juror_eligible(env: Env, juror: Address, category: DisputeCategory) -> bool {
        let vetting_key = (Symbol::new("juror_vet"), juror);
        match env.storage().persistent().get::<_, JurorVettingProfile>(&vetting_key) {
            Some(profile) => {
                if !profile.active { return false; }
                if profile.penalty_points > 3 { return false; }
                // Specialty matching: General can serve any, others match specific categories
                match profile.specialty {
                    JurorSpecialty::General => true,
                    JurorSpecialty::Commerce => matches!(category,
                        DisputeCategory::NonDelivery | DisputeCategory::NotAsDescribed |
                        DisputeCategory::UnauthorizedCharge | DisputeCategory::Other),
                    JurorSpecialty::DigitalGoods => matches!(category,
                        DisputeCategory::NotAsDescribed | DisputeCategory::DamagedDefective),
                    JurorSpecialty::Services => matches!(category,
                        DisputeCategory::ServiceNotProvided | DisputeCategory::DeliveryDispute),
                    JurorSpecialty::Subscription => matches!(category,
                        DisputeCategory::UnauthorizedCharge | DisputeCategory::ServiceNotProvided),
                }
            }
            None => false,
        }
    }

    // ========================================================================
    // v1.1: Reputation-Weighted Voting
    // Higher-reputation jurors have proportionally more voting weight
    // ========================================================================

    /// Execute ruling with reputation-weighted vote counting
    /// Instead of 1-juror-1-vote, each juror's vote is weighted by their reputation
    pub fn execute_weighted_ruling(
        env: Env,
        caller: Address,
        dispute_id: u64,
    ) -> (DisputeRuling, u32, u32) {
        require_coordinator(&env, &caller);

        let mut dispute: DisputeCase = env.storage().persistent().get(&dispute_key(dispute_id)).unwrap();
        assert!(dispute.phase == DisputePhase::Ruling, "Not ruling phase");

        // Count votes with reputation weighting
        let mut weighted_votes: Map<DisputeRuling, u32> = Map::new(&env);
        let mut total_weight: u32 = 0;

        for (juror, commit) in dispute.commit_votes.iter() {
            if commit.revealed {
                if let Some(ref vote) = commit.actual_vote {
                    // Get juror's weight from their vetting profile
                    let weight = get_juror_weight(&env, &juror);
                    let current = weighted_votes.get(vote.clone()).unwrap_or(0);
                    weighted_votes.set(vote.clone(), current + weight);
                    total_weight += weight;
                }
            }
        }

        assert!(total_weight > 0, "No weighted votes");

        // Find majority by weighted votes
        let mut winning_ruling = DisputeRuling::Dismissed;
        let mut max_weighted: u32 = 0;
        for (ruling, weight) in weighted_votes.iter() {
            if weight > max_weighted {
                max_weighted = weight;
                winning_ruling = ruling;
            }
        }

        // Update juror consensus stats
        update_juror_consensus(&env, &dispute, &winning_ruling);

        dispute.ruling = Some(winning_ruling.clone());
        dispute.phase = DisputePhase::Final;
        env.storage().persistent().set(&dispute_key(dispute_id), &dispute);

        env.events().publish(
            (Symbol::new("weighted_ruling"), dispute_id),
            (winning_ruling.clone(), max_weighted, total_weight),
        );

        (winning_ruling, max_weighted, total_weight)
    }

    /// Emergency pause
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
    }
}

/// Get juror voting weight based on reputation score
/// Bronze=1, Silver=2, Gold=3, Platinum=4, Diamond=5
/// Plus consensus bonus: high consensus rate jurors get +1 weight
fn get_juror_weight(env: &Env, juror: &Address) -> u32 {
    let vetting_key = (Symbol::new("juror_vet"), juror.clone());
    match env.storage().persistent().get::<_, JurorVettingProfile>(&vetting_key) {
        Some(profile) => {
            let base_weight = if profile.reputation_score >= 900 { 5 }
                else if profile.reputation_score >= 700 { 4 }
                else if profile.reputation_score >= 450 { 3 }
                else if profile.reputation_score >= 200 { 2 }
                else { 1 };

            // Consensus bonus: jurors with >80% consensus get +1
            let consensus_bonus = if profile.consensus_rate > 8000 && profile.cases_served >= 3 { 1 } else { 0 };

            base_weight + consensus_bonus
        }
        None => 1, // Unvetted juror gets minimum weight
    }
}

/// Update juror consensus statistics after ruling
fn update_juror_consensus(env: &Env, dispute: &DisputeCase, winning_ruling: &DisputeRuling) {
    for (juror, commit) in dispute.commit_votes.iter() {
        let vetting_key = (Symbol::new("juror_vet"), juror.clone());
        if let Some(mut profile) = env.storage().persistent().get::<_, JurorVettingProfile>(&vetting_key) {
            profile.cases_served += 1;
            if commit.revealed {
                if let Some(ref vote) = commit.actual_vote {
                    if vote == winning_ruling {
                        profile.cases_consensus += 1;
                    }
                }
            } else {
                // Non-reveal penalty
                profile.penalty_points += 1;
            }

            // Recalculate consensus rate
            if profile.cases_served > 0 {
                profile.consensus_rate = (profile.cases_consensus as u64 * 10000
                    / profile.cases_served as u64) as u32;
            }

            profile.last_served = env.ledger().timestamp();
            env.storage().persistent().set(&vetting_key, &profile);
        }
    }
}

fn are_all_revealed(dispute: &DisputeCase) -> bool {
    for (_, commit) in dispute.commit_votes.iter() {
        if !commit.revealed {
            return false;
        }
    }
    true
}
