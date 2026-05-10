#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Vec, token};

#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowState { Created, Funded, Delivered, Completed, Disputed, Resolved, Expired, Cancelled, MilestoneActive }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneState { Pending, Submitted, Confirmed, Disputed, Released, Expired }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupEscrowState { Collecting, FullyFunded, Delivered, Completed, Disputed, Resolved, Cancelled, Expired }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone { pub milestone_id: u32, pub description_hash: BytesN<32>, pub amount: i128, pub state: MilestoneState, pub deadline: u64, pub submitted_at: Option<u64>, pub confirmed_at: Option<u64> }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupParticipant { pub buyer: Address, pub amount: i128, pub funded: bool, pub funded_at: Option<u64>, pub refund_percentage: u32 }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupEscrow { pub escrow_id: u64, pub organizer: Address, pub seller: Address, pub token: Address, pub total_amount: i128, pub funded_amount: i128, pub state: GroupEscrowState, pub participants: Vec<GroupParticipant>, pub created_at: u64, pub funding_deadline: u64, pub delivery_deadline: u64, pub auto_release_timeout: u64, pub order_metadata: BytesN<32> }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowAccount { pub escrow_id: u64, pub buyer: Address, pub seller: Address, pub amount: i128, pub token: Address, pub state: EscrowState, pub created_at: u64, pub delivery_deadline: u64, pub confirmation_deadline: u64, pub auto_release_timeout: u64, pub subscription_id: Option<u64>, pub order_metadata: BytesN<32>, pub is_milestone: bool, pub milestones: Vec<Milestone>, pub current_milestone: u32, pub released_amount: i128 }

const ECTR: Symbol = Symbol::new(&[], "ectr"); const GCTR: Symbol = Symbol::new(&[], "gctr");
const FEE: Symbol = Symbol::new(&[], "fee"); const COORD: Symbol = Symbol::new(&[], "coord");
fn ekey(env: &Env, id: u64) -> Symbol { Symbol::new(&[], &format!("e_{}", id).as_str()) }
fn gkey(env: &Env, id: u64) -> Symbol { Symbol::new(&[], &format!("g_{}", id).as_str()) }

#[contract] pub struct EscrowContract;
#[contractimpl]
impl EscrowContract {
    pub fn initialize(env: Env, coordinator: Address, fee_bps: u32) {
        coordinator.require_auth(); if fee_bps > 1000 { panic!("ESC015"); }
        env.storage().persistent().set(&COORD, &coordinator); env.storage().persistent().set(&FEE, &fee_bps);
        env.storage().persistent().set(&ECTR, &0u64); env.storage().persistent().set(&GCTR, &0u64);
    }
    pub fn create_escrow(env: Env, buyer: Address, seller: Address, amount: i128, token: Address, delivery_deadline: u64, auto_release_timeout: u64, order_metadata: BytesN<32>) -> u64 {
        buyer.require_auth(); if amount <= 0 { panic!("ESC007"); } if buyer == seller { panic!("ESC009"); }
        let id: u64 = env.storage().persistent().get(&ECTR).unwrap_or(0) + 1;
        let e = EscrowAccount { escrow_id: id, buyer, seller, amount, token, state: EscrowState::Created, created_at: env.ledger().timestamp(), delivery_deadline, confirmation_deadline: delivery_deadline + auto_release_timeout, auto_release_timeout, subscription_id: None, order_metadata, is_milestone: false, milestones: Vec::new(&env), current_milestone: 0, released_amount: 0 };
        env.storage().persistent().set(&ekey(&env, id), &e); env.storage().persistent().set(&ECTR, &id); id
    }
    pub fn fund_escrow(env: Env, buyer: Address, escrow_id: u64) {
        buyer.require_auth(); let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        if e.buyer != buyer { panic!("ESC001"); } if e.state != EscrowState::Created { panic!("ESC003"); }
        let f: u32 = env.storage().persistent().get(&FEE).unwrap_or(100);
        let fee = e.amount * f as i128 / 10000;
        token::Client::new(&env, &e.token).transfer(&buyer, &env.current_contract_address(), &(e.amount + fee));
        e.state = EscrowState::Funded; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn confirm_delivery(env: Env, seller: Address, escrow_id: u64) {
        seller.require_auth(); let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        if e.seller != seller { panic!("ESC002"); } if e.state != EscrowState::Funded { panic!("ESC004"); }
        e.state = EscrowState::Delivered; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn confirm_receipt(env: Env, buyer: Address, escrow_id: u64) {
        buyer.require_auth(); let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        if e.buyer != buyer { panic!("ESC001"); } if e.state != EscrowState::Delivered { panic!("ESC005"); }
        let f: u32 = env.storage().persistent().get(&FEE).unwrap_or(100); let fee = e.amount * f as i128 / 10000;
        token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.seller, &(e.amount - fee));
        e.state = EscrowState::Completed; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn auto_release(env: Env, _caller: Address, escrow_id: u64) {
        let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        if e.state != EscrowState::Delivered { panic!("ESC005"); } if env.ledger().timestamp() < e.confirmation_deadline { panic!("ESC013"); }
        let f: u32 = env.storage().persistent().get(&FEE).unwrap_or(100); let fee = e.amount * f as i128 / 10000;
        token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.seller, &(e.amount - fee));
        e.state = EscrowState::Completed; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn cancel_escrow(env: Env, caller: Address, escrow_id: u64) {
        caller.require_auth(); let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        if e.state != EscrowState::Created && e.state != EscrowState::Funded { panic!("ESC012"); }
        if e.state == EscrowState::Funded { token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.buyer, &e.amount); }
        e.state = EscrowState::Cancelled; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn expire_escrow(env: Env, _caller: Address, escrow_id: u64) {
        let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        if e.state != EscrowState::Funded { panic!("ESC004"); }
        token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.buyer, &e.amount);
        e.state = EscrowState::Expired; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn freeze_for_dispute(env: Env, _caller: Address, escrow_id: u64) {
        let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        e.state = EscrowState::Disputed; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn execute_ruling(env: Env, _caller: Address, escrow_id: u64, buyer_pct: u32) {
        if buyer_pct > 100 { panic!("ESC011"); }
        let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        if e.state != EscrowState::Disputed { panic!("ESC006"); }
        let ba = e.amount * buyer_pct as i128 / 100; let sa = e.amount - ba;
        if ba > 0 { token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.buyer, &ba); }
        if sa > 0 { token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.seller, &sa); }
        e.state = EscrowState::Resolved; env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn create_milestone_escrow(env: Env, buyer: Address, seller: Address, total_amount: i128, token: Address, ms_amounts: Vec<i128>, ms_deadlines: Vec<u64>, ms_descs: Vec<BytesN<32>>, auto_release_timeout: u64, order_metadata: BytesN<32>) -> u64 {
        buyer.require_auth(); if ms_amounts.len() < 2 { panic!("Need 2+ milestones"); }
        let id: u64 = env.storage().persistent().get(&ECTR).unwrap_or(0) + 1;
        let mut ms = Vec::new(&env);
        for i in 0..ms_amounts.len() { ms.push_back(Milestone { milestone_id: i as u32, description_hash: ms_descs.get(i).unwrap(), amount: ms_amounts.get(i).unwrap(), state: MilestoneState::Pending, deadline: ms_deadlines.get(i).unwrap(), submitted_at: None, confirmed_at: None }); }
        let e = EscrowAccount { escrow_id: id, buyer, seller, amount: total_amount, token, state: EscrowState::MilestoneActive, created_at: env.ledger().timestamp(), delivery_deadline: ms_deadlines.get(ms_deadlines.len()-1).unwrap(), confirmation_deadline: env.ledger().timestamp() + auto_release_timeout, auto_release_timeout, subscription_id: None, order_metadata, is_milestone: true, milestones: ms, current_milestone: 0, released_amount: 0 };
        env.storage().persistent().set(&ekey(&env, id), &e); env.storage().persistent().set(&ECTR, &id); id
    }
    pub fn submit_milestone(env: Env, seller: Address, escrow_id: u64, ms_id: u32) {
        seller.require_auth(); let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        let mut m = e.milestones.get(ms_id as usize).unwrap(); m.state = MilestoneState::Submitted; m.submitted_at = Some(env.ledger().timestamp());
        e.milestones.set(ms_id as usize, m); env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn confirm_milestone(env: Env, buyer: Address, escrow_id: u64, ms_id: u32) {
        buyer.require_auth(); let mut e: EscrowAccount = env.storage().persistent().get(&ekey(&env, escrow_id)).unwrap();
        let mut m = e.milestones.get(ms_id as usize).unwrap();
        let f: u32 = env.storage().persistent().get(&FEE).unwrap_or(100); let fee = m.amount * f as i128 / 10000;
        token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.seller, &(m.amount - fee));
        m.state = MilestoneState::Released; m.confirmed_at = Some(env.ledger().timestamp()); e.milestones.set(ms_id as usize, m);
        e.released_amount += m.amount; e.current_milestone = ms_id + 1;
        if e.released_amount >= e.amount { e.state = EscrowState::Completed; }
        env.storage().persistent().set(&ekey(&env, escrow_id), &e);
    }
    pub fn create_group_escrow(env: Env, organizer: Address, seller: Address, token: Address, total_amount: i128, participants: Vec<GroupParticipant>, funding_deadline: u64, delivery_deadline: u64, auto_release_timeout: u64, order_metadata: BytesN<32>) -> u64 {
        organizer.require_auth(); let id: u64 = env.storage().persistent().get(&GCTR).unwrap_or(0) + 1;
        let g = GroupEscrow { escrow_id: id, organizer, seller, token, total_amount, funded_amount: 0, state: GroupEscrowState::Collecting, participants, created_at: env.ledger().timestamp(), funding_deadline, delivery_deadline, auto_release_timeout, order_metadata };
        env.storage().persistent().set(&gkey(&env, id), &g); env.storage().persistent().set(&GCTR, &id); id
    }
    pub fn fund_group_escrow(env: Env, buyer: Address, escrow_id: u64) {
        buyer.require_auth(); let mut g: GroupEscrow = env.storage().persistent().get(&gkey(&env, escrow_id)).unwrap();
        for i in 0..g.participants.len() { let mut p = g.participants.get(i).unwrap();
            if p.buyer == buyer && !p.funded { token::Client::new(&env, &g.token).transfer(&buyer, &env.current_contract_address(), &p.amount);
                p.funded = true; p.funded_at = Some(env.ledger().timestamp()); g.funded_amount += p.amount; g.participants.set(i, p); break; } }
        if g.funded_amount >= g.total_amount { g.state = GroupEscrowState::FullyFunded; }
        env.storage().persistent().set(&gkey(&env, escrow_id), &g);
    }
    pub fn get_escrow(env: Env, id: u64) -> EscrowAccount { env.storage().persistent().get(&ekey(&env, id)).unwrap() }
    pub fn get_group_escrow(env: Env, id: u64) -> GroupEscrow { env.storage().persistent().get(&gkey(&env, id)).unwrap() }
}
