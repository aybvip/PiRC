#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Vec};

#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeCategory { NonDelivery, NotAsDescribed, DamagedDefective, DeliveryDispute, ServiceNotProvided, UnauthorizedCharge, Other }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeRuling { FullRefund, PartialRefund, SellerFavored, Split, Dismissed }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputePhase { Filed, Evidence, Voting, Ruling, Appealed, Final }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReputationTier { Bronze, Silver, Gold, Platinum, Diamond }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum JurorSpecialty { General, Commerce, DigitalGoods, Services, Subscription }

#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub struct JurorVote { pub juror: Address, pub vote: DisputeRuling, pub confidence: u8, pub voted_at: u64, pub justification_hash: BytesN<32> }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub struct JurorVettingProfile { pub juror: Address, pub reputation_score: u32, pub cases_served: u32, pub cases_consensus: u32, pub consensus_rate: u32, pub specialty: JurorSpecialty, pub active: bool, pub stake: i128, pub last_served: u64, pub penalty_points: u32 }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCase { pub dispute_id: u64, pub escrow_id: u64, pub filer: Address, pub respondent: Address, pub category: DisputeCategory, pub phase: DisputePhase, pub jurors: Vec<Address>, pub votes: Vec<JurorVote>, pub filer_evidence: Vec<BytesN<32>>, pub respondent_evidence: Vec<BytesN<32>>, pub filed_at: u64, pub evidence_deadline: u64, pub voting_deadline: u64, pub ruling: Option<DisputeRuling>, pub is_appealed: bool, pub appeal_fee: i128 }

const DISPUTE_CTR: Symbol = Symbol::new(&[], "dis_ctr");
fn dispute_key(env: &Env, id: u64) -> Symbol { Symbol::new(&[], &format!("dis_{}", id).as_str()) }
fn juror_key(env: &Env, addr: &Address) -> Symbol { Symbol::new(&[], "juror") }

fn tier_weight(tier: &ReputationTier) -> u32 { match tier { ReputationTier::Bronze => 1, ReputationTier::Silver => 2, ReputationTier::Gold => 3, ReputationTier::Platinum => 4, ReputationTier::Diamond => 5 } }

#[contract] pub struct DisputeContract;
#[contractimpl]
impl DisputeContract {
    pub fn open_dispute(env: Env, _caller: Address, escrow_id: u64, filer: Address, respondent: Address, category: DisputeCategory, initial_evidence: BytesN<32>, jurors: Vec<Address>) -> u64 {
        let id: u64 = env.storage().persistent().get(&DISPUTE_CTR).unwrap_or(0) + 1;
        let now = env.ledger().timestamp();
        let mut fe = Vec::new(&env); fe.push_back(initial_evidence);
        let dc = DisputeCase { dispute_id: id, escrow_id, filer, respondent, category, phase: DisputePhase::Evidence, jurors, votes: Vec::new(&env), filer_evidence: fe, respondent_evidence: Vec::new(&env), filed_at: now, evidence_deadline: now + 259200, voting_deadline: now + 432000, ruling: None, is_appealed: false, appeal_fee: 0 };
        env.storage().persistent().set(&dispute_key(&env, id), &dc); env.storage().persistent().set(&DISPUTE_CTR, &id); id
    }
    pub fn submit_evidence(env: Env, party: Address, dispute_id: u64, evidence_hash: BytesN<32>) {
        let mut dc: DisputeCase = env.storage().persistent().get(&dispute_key(&env, dispute_id)).unwrap();
        if dc.phase != DisputePhase::Evidence { panic!("DIS001"); }
        if env.ledger().timestamp() > dc.evidence_deadline { panic!("DIS002"); }
        if party == dc.filer { dc.filer_evidence.push_back(evidence_hash); }
        else if party == dc.respondent { dc.respondent_evidence.push_back(evidence_hash); }
        else { panic!("DIS003"); }
        env.storage().persistent().set(&dispute_key(&env, dispute_id), &dc);
    }
    pub fn start_voting(env: Env, _caller: Address, dispute_id: u64) {
        let mut dc: DisputeCase = env.storage().persistent().get(&dispute_key(&env, dispute_id)).unwrap();
        dc.phase = DisputePhase::Voting; env.storage().persistent().set(&dispute_key(&env, dispute_id), &dc);
    }
    pub fn commit_vote(env: Env, juror: Address, dispute_id: u64, _commitment: BytesN<32>) {
        juror.require_auth();
        let dc: DisputeCase = env.storage().persistent().get(&dispute_key(&env, dispute_id)).unwrap();
        if dc.phase != DisputePhase::Voting { panic!("DIS005"); }
        if !dc.jurors.contains(&juror) { panic!("DIS006"); }
    }
    pub fn reveal_vote(env: Env, juror: Address, dispute_id: u64, vote: DisputeRuling, confidence: u8, justification_hash: BytesN<32>) {
        juror.require_auth();
        let mut dc: DisputeCase = env.storage().persistent().get(&dispute_key(&env, dispute_id)).unwrap();
        let jv = JurorVote { juror: juror.clone(), vote, confidence, voted_at: env.ledger().timestamp(), justification_hash };
        dc.votes.push_back(jv); env.storage().persistent().set(&dispute_key(&env, dispute_id), &dc);
    }
    pub fn execute_ruling(env: Env, _caller: Address, dispute_id: u64) -> (DisputeRuling, u32) {
        let mut dc: DisputeCase = env.storage().persistent().get(&dispute_key(&env, dispute_id)).unwrap();
        if dc.votes.is_empty() { panic!("DIS011"); }
        let mut counts: [u32; 5] = [0; 5];
        for i in 0..dc.votes.len() { let v = dc.votes.get(i).unwrap(); match v.vote { DisputeRuling::FullRefund => counts[0] += 1, DisputeRuling::PartialRefund => counts[1] += 1, DisputeRuling::SellerFavored => counts[2] += 1, DisputeRuling::Split => counts[3] += 1, DisputeRuling::Dismissed => counts[4] += 1 } }
        let max_idx = counts.iter().enumerate().max_by_key(|&(_, c)| c).map(|(i, _)| i).unwrap_or(0);
        let ruling = match max_idx { 0 => DisputeRuling::FullRefund, 1 => DisputeRuling::PartialRefund, 2 => DisputeRuling::SellerFavored, 3 => DisputeRuling::Split, _ => DisputeRuling::Dismissed };
        dc.ruling = Some(ruling.clone()); dc.phase = DisputePhase::Final;
        env.storage().persistent().set(&dispute_key(&env, dispute_id), &dc); (ruling, dc.votes.len() as u32)
    }

    // v1.1: Juror Vetting
    pub fn register_juror(env: Env, juror: Address, specialty: JurorSpecialty, reputation_score: u32, stake: i128) {
        juror.require_auth();
        if reputation_score < 200 { panic!("DIS012"); } if stake < 10_0000000 { panic!("DIS013"); }
        let jp = JurorVettingProfile { juror: juror.clone(), reputation_score, cases_served: 0, cases_consensus: 0, consensus_rate: 0, specialty, active: true, stake, last_served: 0, penalty_points: 0 };
        env.storage().persistent().set(&juror_key(&env, &juror), &jp);
    }
    pub fn deactivate_juror(env: Env, juror: Address) {
        juror.require_auth();
        let mut jp: JurorVettingProfile = env.storage().persistent().get(&juror_key(&env, &juror)).unwrap();
        jp.active = false; env.storage().persistent().set(&juror_key(&env, &juror), &jp);
    }
    pub fn get_juror_profile(env: Env, juror: Address) -> JurorVettingProfile { env.storage().persistent().get(&juror_key(&env, &juror)).unwrap() }
    pub fn is_juror_eligible(env: Env, juror: Address, _category: DisputeCategory) -> bool {
        match env.storage().persistent().get(&juror_key(&env, &juror)) { Some(jp: JurorVettingProfile) => jp.active && jp.penalty_points < 4, None => false }
    }

    // v1.1: Weighted Ruling
    pub fn execute_weighted_ruling(env: Env, _caller: Address, dispute_id: u64) -> (DisputeRuling, u32, u32) {
        let mut dc: DisputeCase = env.storage().persistent().get(&dispute_key(&env, dispute_id)).unwrap();
        if dc.votes.is_empty() { panic!("DIS011"); }
        let mut weights: [u32; 5] = [0; 5];
        for i in 0..dc.votes.len() {
            let v = dc.votes.get(i).unwrap();
            let jp = env.storage().persistent().get(&juror_key(&env, &v.juror)).unwrap_or(JurorVettingProfile { juror: v.juror.clone(), reputation_score: 200, cases_served: 0, cases_consensus: 0, consensus_rate: 0, specialty: JurorSpecialty::General, active: true, stake: 0, last_served: 0, penalty_points: 0 });
            let tier = if jp.reputation_score >= 900 { ReputationTier::Diamond } else if jp.reputation_score >= 700 { ReputationTier::Platinum } else if jp.reputation_score >= 450 { ReputationTier::Gold } else if jp.reputation_score >= 200 { ReputationTier::Silver } else { ReputationTier::Bronze };
            let mut w = tier_weight(&tier);
            if jp.consensus_rate > 80 && jp.cases_served >= 3 { w += 1; }
            match v.vote { DisputeRuling::FullRefund => weights[0] += w, DisputeRuling::PartialRefund => weights[1] += w, DisputeRuling::SellerFavored => weights[2] += w, DisputeRuling::Split => weights[3] += w, DisputeRuling::Dismissed => weights[4] += w }
        }
        let max_idx = weights.iter().enumerate().max_by_key(|&(_, c)| c).map(|(i, _)| i).unwrap_or(0);
        let ruling = match max_idx { 0 => DisputeRuling::FullRefund, 1 => DisputeRuling::PartialRefund, 2 => DisputeRuling::SellerFavored, 3 => DisputeRuling::Split, _ => DisputeRuling::Dismissed };
        dc.ruling = Some(ruling.clone()); dc.phase = DisputePhase::Final;
        env.storage().persistent().set(&dispute_key(&env, dispute_id), &dc);
        (ruling, dc.votes.len() as u32, weights.iter().sum())
    }
}
