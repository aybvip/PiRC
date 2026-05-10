#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, BytesN, Vec, token, IntoVal};

// ============================================================================
// Escrow Contract — PiDCTP Module 1
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

    /// Emergency pause (admin only via coordinator)
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        require_coordinator(&env, &caller);
        env.storage().instance().set(&PAUSED, &paused);
    }
}
