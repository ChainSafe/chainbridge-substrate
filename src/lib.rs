#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, traits::Currency,
    traits::ExistenceRequirement::AllowDeath, ensure, codec::{Decode, Encode},
};
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_runtime::RuntimeDebug;
use sp_core::U256;


mod mock;
mod tests;

// TODO: Should be configurable
const MINIMUM_VOTES: usize = 1;

/// Tracks the transfer in/out of each respective chain
#[derive(Encode, Decode, Clone, Default)]
struct TxCount {
    recv: u32,
    sent: u32,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct TransferProposal<AccountId> {
    votes_for: Vec<AccountId>,
    votes_against: Vec<AccountId>,
    deposit_id: U256,
    origin_chain: U256,
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// The currency mechanism.
    type Currency: Currency<Self::AccountId>;
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId,
        <T as frame_system::Trait>::Hash
    {
        // dest_id, deposit_id, to, token_id, metadata
        AssetTransfer(Vec<u8>, u32, Vec<u8>, Vec<u8>, Vec<u8>),
        ValidatorAdded(AccountId),
        ValidatorRemoved(AccountId),
        /// Validator has created new proposal
        ProposalCreated(Hash, AccountId),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Interactions with this chain is not permitted
        ChainNotWhitelisted,
        ValidatorAlreadyExists,
        /// Provided accountId is not a validator
        ValidatorInvalid,
        ValidatorAlreadyVoted,

        ProposalAlreadyExists,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bridge {
        EmitterAddress: Vec<u8>;

        Chains: map hasher(blake2_256) Vec<u8> => Option<TxCount>;

        EndowedAccount get(fn endowed) config(): T::AccountId;

        pub Validators get(fn validators): map hasher(blake2_256) T::AccountId => bool;

        Proposals get(fn proposals): map hasher(blake2_256) T::Hash => Option<TransferProposal<T::AccountId>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Default method for emitting events
        fn deposit_event() = default;

        /// Sets the address used to identify this chain
        pub fn set_address(origin, addr: Vec<u8>) -> DispatchResult {
            // TODO: Limit access
            ensure_root(origin)?;
            EmitterAddress::put(addr);
            Ok(())
        }

        /// Enables a chain ID as a destination for a bridge transfer
        pub fn whitelist_chain(origin, id: Vec<u8>) -> DispatchResult {
            // TODO: Limit access
            ensure_root(origin)?;
            Chains::insert(&id, TxCount { recv: 0, sent: 0 });
            Ok(())
        }

        /// Adds a new validator to the set. Errors if validator already exists.
        pub fn add_validator(origin, v: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(!Self::is_validator(&v), Error::<T>::ValidatorAlreadyExists);
            <Validators<T>>::insert(&v, true);
            Self::deposit_event(RawEvent::ValidatorAdded(v));
            Ok(())
        }

        /// Removes an existing validator from the set. Errors if validator doesn't exist.
        pub fn remove_validator(origin, v: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::is_validator(&v), Error::<T>::ValidatorInvalid);

            <Validators<T>>::remove(&v);
            Self::deposit_event(RawEvent::ValidatorRemoved(v));
            Ok(())
        }

        pub fn create_proposal(origin, proposal_id: T::Hash, deposit_id: U256, origin_chain: U256, metadata: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);
            // Make sure proposal doesn't already exist
            ensure!(!<Proposals<T>>::contains_key(proposal_id), Error::<T>::ProposalAlreadyExists);

            let mut proposal = TransferProposal {
                // TODO: Get this working
                // votes_for: vec![who],
                // votes_against: vec![],
                votes_for: Vec::new(),
                votes_against: Vec::new(),
                deposit_id,
                origin_chain,
            };
            proposal.votes_for.push(who.clone());

            <Proposals<T>>::insert(proposal_id, proposal);
            Self::deposit_event(RawEvent::ProposalCreated(proposal_id, who));
            Ok(())
        }

        pub fn vote(origin, vote_id: T::Hash, deposit_id: U256, origin_chain: U256, metadata: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            // Check if proposal exists
            if let Some(votes) = <Proposals<T>>::get(vote_id) {
                // If validator hasn't voted, update
                // if !votes.contains(who) {
                //     // votes.insert(who);
                //     // <Proposals<T>>::put(vote_id, votes);
                //     // Execute transfer if threshold is met
                //     // TODO: Uncomment this
                //     // if votes.len() > MINIMUM_VOTES {
                //     //     Self::execute_transfer(vote_id);
                //     // }
                // } else {
                //     Err(Error::<T>::ValidatorAlreadyVoted)?
                // }
            } else {
                // First proposal submission, must create one
                // let votes = Vec::new();
                // votes.insert(who);
                // <Proposals<T>>::put(vote_id, votes);
            }

            Ok(())
        }

        /// Completes an asset transfer to the chain by emitting an event to be acted on by the
        /// bridge and updating the tx count for the respective chan.
        pub fn receive_asset(origin, dest_id: Vec<u8>, to: Vec<u8>, token_id: Vec<u8>, metadata: Vec<u8>) -> DispatchResult {
            // TODO: Limit access
            ensure_root(origin)?;
            // Ensure chain is whitelisted
            if let Some(mut counter) = Chains::get(&dest_id) {
                // Increment counter and store
                counter.recv += 1;
                Chains::insert(&dest_id, counter.clone());
                Self::deposit_event(RawEvent::AssetTransfer(dest_id, counter.recv, to, token_id, metadata));
                Ok(())
            } else {
                Err(Error::<T>::ChainNotWhitelisted)?
            }
        }

        // TODO: Should use correct amount type
        pub fn transfer(origin, to: T::AccountId, amount: u32) -> DispatchResult {
            ensure_root(origin)?;
            let source: T::AccountId = <EndowedAccount<T>>::get();
            T::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
            Ok(())
        }
    }

}

/// Main module declaration.
/// Here we should include non-state changing public funcs
impl<T: Trait> Module<T> {
	pub fn is_validator(who: &T::AccountId) -> bool {
		Self::validators(who)
	}
}
