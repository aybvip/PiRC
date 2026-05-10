#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Vec, Map};

// ============================================================================
// Dispute Resolution Contract — PiDCTP Module 3
// ============================================================================

#[contract]
pub struct DisputeContract;

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

    /// Emergency pause
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
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
