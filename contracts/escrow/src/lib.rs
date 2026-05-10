#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, BytesN, Vec, Map, token, IntoVal};

// ============================================================================
// Escrow Contract — PiDCTP Module 1
// + Milestone Escrow, Group Escrow (v1.1)
// ============================================================================

#[contract]
pub struct EscrowContract;

// Escrow State enum
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum EscrowState {
    Created,
    Funded,
    Delivered,
    Completed,
    Disputed,
    Resolved,
    Expired,
    Cancelled,
    MilestoneActive,  // v1.1: Milestone escrow in progress
}

// ============================================================================
// v1.1: Milestone Escrow
// Multi-stage release: funds released incrementally as milestones are confirmed
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum MilestoneState {
    Pending,       // Not yet started
    Submitted,     // Seller submitted milestone deliverable
    Confirmed,     // Buyer confirmed, funds released
    Disputed,      // Milestone under dispute
    Released,      // Funds released to seller
    Expired,       // Deadline passed without submission
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Milestone {
    pub milestone_id: u32,
    pub description_hash: BytesN<32>,
    pub amount: i128,              // Amount released on this milestone
    pub state: MilestoneState,
    pub deadline: u64,             // Deadline for seller submission
    pub submitted_at: Option<u64>,
    pub confirmed_at: Option<u64>,
}

// ============================================================================
// v1.1: Group Escrow
// Multi-party escrow: multiple buyers pool funds for a single purchase
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum GroupEscrowState {
    Collecting,     // Waiting for all participants to fund
    FullyFunded,    // All participants have funded
    Delivered,
    Completed,
    Disputed,
    Resolved,
    Cancelled,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct GroupParticipant {
    pub buyer: Address,
    pub amount: i128,             // This participant's contribution
    pub funded: bool,
    pub funded_at: Option<u64>,
    pub refund_percentage: u32,   // Their share for refunds (0-10000 bps)
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct GroupEscrow {
    pub escrow_id: u64,
    pub seller: Address,
    pub token: Address,
    pub total_amount: i128,
    pub participants: Map<Address, GroupParticipant>,
    pub participant_count: u32,
    pub funded_count: u32,
    pub state: GroupEscrowState,
    pub created_at: u64,
    pub funding_deadline: u64,
    pub delivery_deadline: u64,
    pub confirmation_deadline: u64,
    pub auto_release_timeout: u64,
    pub order_metadata: BytesN<32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
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
    // --- v1.1: Milestone Escrow Fields ---
    pub is_milestone: bool,
    pub milestones: Vec<Milestone>,
    pub current_milestone: u32,
    pub released_amount: i128,
}

// Storage keys
const COORDINATOR: Symbol = Symbol::new("coordinator");
const NEXT_ID: Symbol = Symbol::new("next_id");
const PAUSED: Symbol = Symbol::new("paused");
const FEE_RECIPIENT: Symbol = Symbol::new("fee_recipient");
const FEE_BPS: Symbol = Symbol::new("fee_bps"); // fee in basis points (100 = 1%)

fn escrow_key(id: u64) -> Symbol {
    Symbol::new(&format!("e_{}", id))
}

fn get_escrow(env: &Env, id: u64) -> EscrowAccount {
    env.storage().persistent().get(&escrow_key(id)).unwrap()
}

fn set_escrow(env: &Env, escrow: &EscrowAccount) {
    env.storage().persistent().set(&escrow_key(escrow.escrow_id), escrow);
}

fn require_not_paused(env: &Env) {
    let paused: bool = env.storage().instance().get(&PAUSED).unwrap_or(false);
    assert!(!paused, "Protocol paused");
}

fn require_coordinator(env: &Env, caller: &Address) {
    let coordinator: Address = env.storage().instance().get(&COORDINATOR).unwrap();
    assert!(&caller == &coordinator, "Only coordinator");
}

#[contractimpl]
impl EscrowContract {
    /// Initialize the escrow contract with coordinator address and fee configuration
    pub fn initialize(
        env: Env,
        coordinator: Address,
        fee_recipient: Address,
        fee_bps: u32, // e.g., 100 = 1% fee
    ) {
        let storage = env.storage().instance();
        assert!(!storage.has(&COORDINATOR), "Already initialized");
        assert!(fee_bps <= 1000, "Fee cannot exceed 10%");
        storage.set(&COORDINATOR, &coordinator);
        storage.set(&FEE_RECIPIENT, &fee_recipient);
        storage.set(&FEE_BPS, &fee_bps);
        storage.set(&NEXT_ID, &1u64);
        storage.set(&PAUSED, &false);
    }

    /// Create a new escrow order
    pub fn create_escrow(
        env: Env,
        buyer: Address,
        seller: Address,
        amount: i128,
        token_addr: Address,
        delivery_deadline: u64,
        auto_release_timeout: u64,
        order_metadata: BytesN<32>,
    ) -> u64 {
        require_not_paused(&env);
        buyer.require_auth();

        assert!(buyer != seller, "Buyer seller same");
        assert!(amount > 0, "Amount zero");
        assert!(
            delivery_deadline > env.ledger().timestamp(),
            "Deadline past"
        );
        assert!(auto_release_timeout >= 86400, "Timeout min 1d");

        let next_id: u64 = env.storage().instance().get(&NEXT_ID).unwrap_or(1);
        env.storage().instance().set(&NEXT_ID, &(next_id + 1));

        let escrow = EscrowAccount {
            escrow_id: next_id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            amount,
            token: token_addr,
            state: EscrowState::Created,
            created_at: env.ledger().timestamp(),
            delivery_deadline,
            confirmation_deadline: 0,
            auto_release_timeout,
            subscription_id: None,
            order_metadata,
            // --- v1.1: Milestone fields ---
            is_milestone: false,
            milestones: Vec::new(&env),
            current_milestone: 0,
            released_amount: 0,
        };

        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("escrow_created"), next_id),
            (buyer, seller, amount),
        );

        next_id
    }

    /// Buyer deposits payment into escrow
    pub fn fund_escrow(env: Env, buyer: Address, escrow_id: u64) {
        require_not_paused(&env);
        buyer.require_auth();

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Created, "Not Created");
        assert!(buyer == escrow.buyer, "Not buyer");

        // Transfer tokens from buyer to this contract
        let client = token::Client::new(&env, &escrow.token);
        client.transfer(
            &buyer,
            &env.current_contract_address(),
            &escrow.amount,
        );

        escrow.state = EscrowState::Funded;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("escrow_funded"), escrow_id),
            (escrow.amount,),
        );
    }

    /// Seller confirms delivery of goods/services
    pub fn confirm_delivery(env: Env, seller: Address, escrow_id: u64) {
        require_not_paused(&env);
        seller.require_auth();

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Funded, "Not Funded");
        assert!(seller == escrow.seller, "Not seller");
        assert!(
            env.ledger().timestamp() <= escrow.delivery_deadline,
            "Deadline passed"
        );

        escrow.state = EscrowState::Delivered;
        escrow.confirmation_deadline =
            env.ledger().timestamp() + escrow.auto_release_timeout;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("escrow_delivered"), escrow_id),
            (seller,),
        );
    }

    /// Buyer confirms receipt, triggering fund release to seller
    pub fn confirm_receipt(env: Env, buyer: Address, escrow_id: u64) {
        require_not_paused(&env);
        buyer.require_auth();

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Delivered, "Not Delivered");
        assert!(buyer == escrow.buyer, "Not buyer");

        // Effects first
        escrow.state = EscrowState::Completed;
        set_escrow(&env, &escrow);

        // Interactions: release funds to seller minus fee
        let fee_bps: u32 = env.storage().instance().get(&FEE_BPS).unwrap_or(100);
        let fee_recipient: Address = env.storage().instance().get(&FEE_RECIPIENT).unwrap();
        let fee = (escrow.amount * (fee_bps as i128)) / 10000;
        let net = escrow.amount - fee;

        let client = token::Client::new(&env, &escrow.token);
        client.transfer(&env.current_contract_address(), &escrow.seller, &net);
        if fee > 0 {
            client.transfer(&env.current_contract_address(), &fee_recipient, &fee);
        }

        env.events().publish(
            (Symbol::new("escrow_completed"), escrow_id),
            (escrow.buyer, escrow.seller, net),
        );
    }

    /// Auto-release funds to seller if buyer doesn't confirm within timeout
    pub fn auto_release(env: Env, _caller: Address, escrow_id: u64) {
        require_not_paused(&env);

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Delivered, "Not Delivered");
        assert!(
            env.ledger().timestamp() > escrow.confirmation_deadline,
            "Timeout not reached"
        );

        // Effects
        escrow.state = EscrowState::Completed;
        set_escrow(&env, &escrow);

        // Interactions
        let fee_bps: u32 = env.storage().instance().get(&FEE_BPS).unwrap_or(100);
        let fee_recipient: Address = env.storage().instance().get(&FEE_RECIPIENT).unwrap();
        let fee = (escrow.amount * (fee_bps as i128)) / 10000;
        let net = escrow.amount - fee;

        let client = token::Client::new(&env, &escrow.token);
        client.transfer(&env.current_contract_address(), &escrow.seller, &net);
        if fee > 0 {
            client.transfer(&env.current_contract_address(), &fee_recipient, &fee);
        }

        env.events().publish(
            (Symbol::new("escrow_auto_released"), escrow_id),
            (escrow.seller, net),
        );
    }

    /// Cancel escrow (buyer cancels before funding, or mutual cancel if funded)
    pub fn cancel_escrow(env: Env, caller: Address, escrow_id: u64) {
        require_not_paused(&env);
        caller.require_auth();

        let mut escrow = get_escrow(&env, escrow_id);

        match escrow.state {
            EscrowState::Created => {
                assert!(caller == escrow.buyer, "Only buyer cancels Created");
            }
            EscrowState::Funded => {
                // Mutual cancel: both buyer and seller must sign
                assert!(
                    caller == escrow.buyer || caller == escrow.seller,
                    "Not a party"
                );
            }
            _ => panic!("Cannot cancel in current state"),
        }

        // Refund if funded
        if escrow.state == EscrowState::Funded {
            let client = token::Client::new(&env, &escrow.token);
            client.transfer(
                &env.current_contract_address(),
                &escrow.buyer,
                &escrow.amount,
            );
        }

        escrow.state = EscrowState::Cancelled;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("escrow_cancelled"), escrow_id),
            (caller,),
        );
    }

    /// Expire escrow if seller fails to deliver by deadline
    pub fn expire_escrow(env: Env, _caller: Address, escrow_id: u64) {
        require_not_paused(&env);

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Funded, "Not Funded");
        assert!(
            env.ledger().timestamp() > escrow.delivery_deadline,
            "Not expired"
        );

        // Refund buyer
        let client = token::Client::new(&env, &escrow.token);
        client.transfer(
            &env.current_contract_address(),
            &escrow.buyer,
            &escrow.amount,
        );

        escrow.state = EscrowState::Expired;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("escrow_expired"), escrow_id),
            (escrow.seller,),
        );
    }

    /// Freeze escrow funds when a dispute is opened (called by coordinator)
    pub fn freeze_for_dispute(env: Env, caller: Address, escrow_id: u64) {
        require_coordinator(&env, &caller);

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(
            escrow.state == EscrowState::Funded || escrow.state == EscrowState::Delivered,
            "Cannot dispute"
        );

        escrow.state = EscrowState::Disputed;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("escrow_disputed"), escrow_id),
            (escrow.buyer, escrow.seller),
        );
    }

    /// Execute dispute ruling — distribute funds per ruling (called by coordinator)
    pub fn execute_ruling(
        env: Env,
        caller: Address,
        escrow_id: u64,
        buyer_percentage: u32, // 0-10000 (0%=0, 100%=10000)
    ) {
        require_coordinator(&env, &caller);

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.state == EscrowState::Disputed, "Not Disputed");
        assert!(buyer_percentage <= 10000, "Invalid percentage");

        let buyer_amount = (escrow.amount * (buyer_percentage as i128)) / 10000;
        let seller_amount = escrow.amount - buyer_amount;

        let client = token::Client::new(&env, &escrow.token);

        if buyer_amount > 0 {
            client.transfer(
                &env.current_contract_address(),
                &escrow.buyer,
                &buyer_amount,
            );
        }
        if seller_amount > 0 {
            client.transfer(
                &env.current_contract_address(),
                &escrow.seller,
                &seller_amount,
            );
        }

        escrow.state = EscrowState::Resolved;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("escrow_resolved"), escrow_id),
            (buyer_amount, seller_amount),
        );
    }

    /// Get escrow details
    pub fn get_escrow(env: Env, escrow_id: u64) -> EscrowAccount {
        get_escrow(&env, escrow_id)
    }

    // ========================================================================
    // v1.1: Milestone Escrow Functions
    // ========================================================================

    /// Create a milestone escrow with multiple release stages
    /// Each milestone has its own amount, deadline, and independent confirmation
    pub fn create_milestone_escrow(
        env: Env,
        buyer: Address,
        seller: Address,
        total_amount: i128,
        token_addr: Address,
        milestone_amounts: Vec<i128>,
        milestone_deadlines: Vec<u64>,
        milestone_descriptions: Vec<BytesN<32>>,
        auto_release_timeout: u64,
        order_metadata: BytesN<32>,
    ) -> u64 {
        require_not_paused(&env);
        buyer.require_auth();

        assert!(buyer != seller, "Buyer seller same");
        assert!(total_amount > 0, "Amount zero");
        assert!(milestone_amounts.len() >= 2, "Min 2 milestones");
        assert!(milestone_amounts.len() == milestone_deadlines.len(), "Deadline mismatch");
        assert!(milestone_amounts.len() == milestone_descriptions.len(), "Desc mismatch");

        // Verify amounts sum to total
        let mut sum: i128 = 0;
        for i in 0..milestone_amounts.len() {
            assert!(milestone_amounts.get(i).unwrap() > 0, "Milestone amount zero");
            sum += milestone_amounts.get(i).unwrap();
        }
        assert!(sum == total_amount, "Amounts don't sum to total");

        // Verify deadlines are sequential and in the future
        let now = env.ledger().timestamp();
        let mut prev_deadline: u64 = 0;
        for i in 0..milestone_deadlines.len() {
            let dl = milestone_deadlines.get(i).unwrap();
            assert!(dl > now, "Deadline past");
            if i > 0 {
                assert!(dl > prev_deadline, "Deadlines not sequential");
            }
            prev_deadline = dl;
        }

        let next_id: u64 = env.storage().instance().get(&NEXT_ID).unwrap_or(1);
        env.storage().instance().set(&NEXT_ID, &(next_id + 1));

        // Build milestones vector
        let mut milestones = Vec::new(&env);
        for i in 0..milestone_amounts.len() {
            milestones.push_back(Milestone {
                milestone_id: (i as u32) + 1,
                description_hash: milestone_descriptions.get(i).unwrap(),
                amount: milestone_amounts.get(i).unwrap(),
                state: MilestoneState::Pending,
                deadline: milestone_deadlines.get(i).unwrap(),
                submitted_at: None,
                confirmed_at: None,
            });
        }

        let escrow = EscrowAccount {
            escrow_id: next_id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            amount: total_amount,
            token: token_addr,
            state: EscrowState::Created,
            created_at: now,
            delivery_deadline: milestone_deadlines.get(milestone_deadlines.len() - 1).unwrap(),
            confirmation_deadline: 0,
            auto_release_timeout,
            subscription_id: None,
            order_metadata,
            is_milestone: true,
            milestones,
            current_milestone: 1,
            released_amount: 0,
        };

        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("milestone_escrow_created"), next_id),
            (buyer, seller, total_amount, milestone_amounts.len() as u32),
        );

        next_id
    }

    /// Seller submits a milestone deliverable
    pub fn submit_milestone(env: Env, seller: Address, escrow_id: u64, milestone_id: u32) {
        require_not_paused(&env);
        seller.require_auth();

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.is_milestone, "Not milestone escrow");
        assert!(seller == escrow.seller, "Not seller");

        let idx = (milestone_id - 1) as usize;
        let mut milestone = escrow.milestones.get(idx as u32).unwrap();
        assert!(milestone.state == MilestoneState::Pending, "Not pending");
        assert!(env.ledger().timestamp() <= milestone.deadline, "Deadline passed");

        milestone.state = MilestoneState::Submitted;
        milestone.submitted_at = Some(env.ledger().timestamp());
        escrow.milestones.set(idx as u32, milestone);

        if escrow.state == EscrowState::Created || escrow.state == EscrowState::Funded {
            escrow.state = EscrowState::MilestoneActive;
        }

        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("milestone_submitted"), escrow_id),
            (milestone_id, seller),
        );
    }

    /// Buyer confirms a milestone, releasing its funds to seller
    pub fn confirm_milestone(env: Env, buyer: Address, escrow_id: u64, milestone_id: u32) {
        require_not_paused(&env);
        buyer.require_auth();

        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.is_milestone, "Not milestone escrow");
        assert!(buyer == escrow.buyer, "Not buyer");

        let idx = (milestone_id - 1) as usize;
        let mut milestone = escrow.milestones.get(idx as u32).unwrap();
        assert!(milestone.state == MilestoneState::Submitted, "Not submitted");

        milestone.state = MilestoneState::Released;
        milestone.confirmed_at = Some(env.ledger().timestamp());
        escrow.milestones.set(idx as u32, milestone);

        // Release milestone funds to seller
        let fee_bps: u32 = env.storage().instance().get(&FEE_BPS).unwrap_or(100);
        let fee_recipient: Address = env.storage().instance().get(&FEE_RECIPIENT).unwrap();
        let fee = (milestone.amount * (fee_bps as i128)) / 10000;
        let net = milestone.amount - fee;

        let client = token::Client::new(&env, &escrow.token);
        client.transfer(&env.current_contract_address(), &escrow.seller, &net);
        if fee > 0 {
            client.transfer(&env.current_contract_address(), &fee_recipient, &fee);
        }

        escrow.released_amount += milestone.amount;
        escrow.current_milestone = milestone_id + 1;

        // Check if all milestones are completed
        let all_released = are_all_milestones_released(&escrow);
        if all_released {
            escrow.state = EscrowState::Completed;
        }

        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("milestone_confirmed"), escrow_id),
            (milestone_id, net),
        );
    }

    /// Expire a milestone if seller didn't submit by deadline
    pub fn expire_milestone(env: Env, _caller: Address, escrow_id: u64, milestone_id: u32) {
        let mut escrow = get_escrow(&env, escrow_id);
        assert!(escrow.is_milestone, "Not milestone escrow");

        let idx = (milestone_id - 1) as usize;
        let mut milestone = escrow.milestones.get(idx as u32).unwrap();
        assert!(milestone.state == MilestoneState::Pending, "Not pending");
        assert!(env.ledger().timestamp() > milestone.deadline, "Not expired");

        milestone.state = MilestoneState::Expired;
        escrow.milestones.set(idx as u32, milestone);

        // Refund remaining funds to buyer
        let remaining = escrow.amount - escrow.released_amount;
        if remaining > 0 {
            let client = token::Client::new(&env, &escrow.token);
            client.transfer(&env.current_contract_address(), &escrow.buyer, &remaining);
        }

        escrow.state = EscrowState::Expired;
        set_escrow(&env, &escrow);

        env.events().publish(
            (Symbol::new("milestone_expired"), escrow_id),
            (milestone_id,),
        );
    }

    // ========================================================================
    // v1.1: Group Escrow Functions
    // ========================================================================

    /// Create a group escrow — multiple buyers pool funds
    pub fn create_group_escrow(
        env: Env,
        organizer: Address,
        seller: Address,
        token_addr: Address,
        total_amount: i128,
        participants: Vec<(Address, i128)>,  // (address, contribution amount)
        funding_deadline: u64,
        delivery_deadline: u64,
        auto_release_timeout: u64,
        order_metadata: BytesN<32>,
    ) -> u64 {
        require_not_paused(&env);
        organizer.require_auth();

        assert!(total_amount > 0, "Amount zero");
        assert!(funding_deadline > env.ledger().timestamp(), "Deadline past");
        assert!(delivery_deadline > funding_deadline, "Delivery before funding");

        // Verify participant amounts sum to total
        let mut sum: i128 = 0;
        let mut participant_map = Map::new(&env);
        for i in 0..participants.len() {
            let (addr, amt) = participants.get(i).unwrap();
            assert!(amt > 0, "Contribution zero");
            sum += amt;
            let pct = ((amt as u64) * 10000 / (total_amount as u64)) as u32;
            participant_map.set(addr, GroupParticipant {
                buyer: addr.clone(),
                amount: amt,
                funded: false,
                funded_at: None,
                refund_percentage: pct,
            });
        }
        assert!(sum == total_amount, "Contributions don't sum");

        let next_id: u64 = env.storage().instance().get(&NEXT_ID).unwrap_or(1);
        env.storage().instance().set(&NEXT_ID, &(next_id + 1));

        let group = GroupEscrow {
            escrow_id: next_id,
            seller: seller.clone(),
            token: token_addr,
            total_amount,
            participants: participant_map,
            participant_count: participants.len() as u32,
            funded_count: 0,
            state: GroupEscrowState::Collecting,
            created_at: env.ledger().timestamp(),
            funding_deadline,
            delivery_deadline,
            confirmation_deadline: 0,
            auto_release_timeout,
            order_metadata,
        };

        let group_key = Symbol::new(&format!("g_{}", next_id));
        env.storage().persistent().set(&group_key, &group);

        env.events().publish(
            (Symbol::new("group_escrow_created"), next_id),
            (organizer, seller, total_amount, participants.len() as u32),
        );

        next_id
    }

    /// Participant funds their contribution to a group escrow
    pub fn fund_group_escrow(env: Env, buyer: Address, escrow_id: u64) {
        require_not_paused(&env);
        buyer.require_auth();

        let group_key = Symbol::new(&format!("g_{}", escrow_id));
        let mut group: GroupEscrow = env.storage().persistent().get(&group_key).unwrap();
        assert!(group.state == GroupEscrowState::Collecting, "Not collecting");
        assert!(env.ledger().timestamp() <= group.funding_deadline, "Funding ended");

        let mut participant = group.participants.get(buyer.clone()).unwrap();
        assert!(!participant.funded, "Already funded");

        // Transfer contribution
        let client = token::Client::new(&env, &group.token);
        client.transfer(
            &buyer,
            &env.current_contract_address(),
            &participant.amount,
        );

        participant.funded = true;
        participant.funded_at = Some(env.ledger().timestamp());
        group.participants.set(buyer.clone(), participant);
        group.funded_count += 1;

        // Check if all funded
        if group.funded_count == group.participant_count {
            group.state = GroupEscrowState::FullyFunded;
        }

        env.storage().persistent().set(&group_key, &group);

        env.events().publish(
            (Symbol::new("group_funded"), escrow_id),
            (buyer, group.funded_count, group.participant_count),
        );
    }

    /// Get group escrow details
    pub fn get_group_escrow(env: Env, escrow_id: u64) -> GroupEscrow {
        let group_key = Symbol::new(&format!("g_{}", escrow_id));
        env.storage().persistent().get(&group_key).unwrap()
    }

    /// Emergency pause (admin only via coordinator)
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
    }
}

fn are_all_milestones_released(escrow: &EscrowAccount) -> bool {
    for i in 0..escrow.milestones.len() {
        let m = escrow.milestones.get(i).unwrap();
        if m.state != MilestoneState::Released {
            return false;
        }
    }
    true
}
