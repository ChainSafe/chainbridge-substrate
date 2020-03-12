// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, ExistenceRequirement::AllowDeath},
    Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::traits::{AccountIdConversion, Dispatchable};
use sp_runtime::{ModuleId, RuntimeDebug};
use sp_std::prelude::*;

use codec::{Decode, Encode};

mod mock;
mod tests;

const MODULE_ID: ModuleId = ModuleId(*b"cb/bridg");

/// Tracks the transfer in/out of each respective chain
#[derive(Encode, Decode, Clone, Default)]
struct TxCount {
    recv: u32,
    sent: u32,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ProposalVotes<AccountId, Hash> {
    votes_for: Vec<AccountId>,
    votes_against: Vec<AccountId>,
    // TODO: If hash matches the key in the map, we can simplify logic below to not need deposit_id when inserting/updating
    hash: Hash,
}

impl<AccountId, Hash> ProposalVotes<AccountId, Hash> {
    fn new(hash: Hash) -> Self {
        Self {
            votes_for: vec![],
            votes_against: vec![],
            hash,
        }
    }
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// The currency mechanism.
    type Currency: Currency<Self::AccountId>;
    /// Proposed dispatchable call
    type Proposal: Parameter + Dispatchable<Origin = Self::Origin>;
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId,
        <T as frame_system::Trait>::Hash
    {
        // dest_id, deposit_id, to, token_id, metadata
        AssetTransfer(Vec<u8>, u32, Vec<u8>, Vec<u8>, Vec<u8>),
        /// Valdiator added to set
        ValidatorAdded(AccountId),
        /// Validator removed from set
        ValidatorRemoved(AccountId),

        /// Vote submitted in favour of proposal
        VoteFor(Hash, AccountId),
        /// Vot submitted against proposal
        VoteAgainst(Hash, AccountId),

        /// Voting successful for a proposal
        ProposalSucceeded(Hash),
        /// Voting rejected a proposal
        ProposalFailed(Hash),
    }
}

// TODO: Pass params to errors
decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Interactions with this chain is not permitted
        ChainNotWhitelisted,
        /// Validator already in set
        ValidatorAlreadyExists,
        /// Provided accountId is not a validator
        ValidatorInvalid,
        /// Validator has already submitted some vote for this proposal
        ValidatorAlreadyVoted,
        /// A proposal with these parameters has already been submitted
        ProposalAlreadyExists,
        /// No proposal with the ID was found
        ProposalDoesNotExist,
        /// Proposal has either failed or succeeded
        ProposalAlreadyComplete,

        DebugInnerCallFailed,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bridge {
        EmitterAddress: Vec<u8>;

        Chains: map hasher(blake2_256) Vec<u8> => Option<TxCount>;

        EndowedAccount get(fn endowed) config(): T::AccountId;

        ValidatorThreshold get(fn validator_threshold) config(): u32;

        pub Validators get(fn validators): map hasher(blake2_256) T::AccountId => bool;

        /// All known proposals.
        /// The key is the hash of the call and the deposit ID, to ensure it's unique.
        pub Votes get(fn votes):
            map hasher(blake2_256) T::Hash
            => Option<ProposalVotes<T::AccountId, T::Hash>>;

        pub Proposals get(fn proposals):
            map hasher(blake2_256) T::Hash
            => Option<<T as Trait>::Proposal>;
    }
    add_extra_genesis {
        config(validators): Vec<T::AccountId>;
        build(|config| {
            Module::<T>::initialize_validators(&config.validators);
            // Create Bridge account
            // let _ = T::Currency::make_free_balance_be(
            // 	&<Module<T>>::account_id(),
            // 	T::Currency::minimum_balance(),
            // );
        });
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Default method for emitting events
        fn deposit_event() = default;

        /// Sets the address used to identify this chain
        pub fn set_address(origin, addr: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            EmitterAddress::put(addr);
            Ok(())
        }

        /// Enables a chain ID as a destination for a bridge transfer
        pub fn whitelist_chain(origin, id: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            Chains::insert(&id, TxCount { recv: 0, sent: 0 });
            Ok(())
        }

        /// Adds a new validator to the set. Errors if validator already exists.
        pub fn add_validator(origin, v: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            ensure!(!Self::is_validator(&v), Error::<T>::ValidatorAlreadyExists);
            <Validators<T>>::insert(&v, true);
            Self::deposit_event(RawEvent::ValidatorAdded(v));
            Ok(())
        }

        /// Removes an existing validator from the set. Errors if validator doesn't exist.
        pub fn remove_validator(origin, v: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            ensure!(Self::is_validator(&v), Error::<T>::ValidatorInvalid);
            <Validators<T>>::remove(&v);
            Self::deposit_event(RawEvent::ValidatorRemoved(v));
            Ok(())
        }

        pub fn create_proposal(origin, hash: T::Hash, call: Box<<T as Trait>::Proposal>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            // Make sure proposal doesn't already exist
            ensure!(!<Votes<T>>::contains_key(hash), Error::<T>::ProposalAlreadyExists);

            let proposal = ProposalVotes::new(hash);
            <Votes<T>>::insert(hash, proposal.clone());
            <Proposals<T>>::insert(hash, call);

            // Creating a proposal also votes for it
            Self::vote_for(who, proposal)
        }

        pub fn vote(origin, hash: T::Hash, in_favour: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            // Check if proposal exists
            if let Some(votes) = <Votes<T>>::get(hash) {
                // Vote if they haven't already
                if in_favour {
                    Self::vote_for(who, votes)?
                } else {
                    Self::vote_against(who, votes)?
                }
            } else {
                Err(Error::<T>::ProposalDoesNotExist)?
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
            let who = ensure_signed(origin)?;
            ensure!(who == Self::account_id(), Error::<T>::DebugInnerCallFailed);
            let source: T::AccountId = <EndowedAccount<T>>::get();
            T::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
            Ok(())
        }
    }
}

/// Main module declaration.
/// Here we should include non-state changing public funcs
impl<T: Trait> Module<T> {
    /// Checks if who is a validator
    pub fn is_validator(who: &T::AccountId) -> bool {
        Self::validators(who)
    }

    /// Used for genesis config of validator set
    fn initialize_validators(validators: &[T::AccountId]) {
        if !validators.is_empty() {
            for v in validators {
                <Validators<T>>::insert(v, true);
            }
        }
    }

    /// Provides an AccountId for the pallet.
    /// This is used both as an origin check and deposit/withdrawal account.
    pub fn account_id() -> T::AccountId {
        MODULE_ID.into_account()
    }

    /// Note: Existence of proposal must be checked before calling
    fn vote_for(
        who: T::AccountId,
        mut votes: ProposalVotes<T::AccountId, T::Hash>,
    ) -> DispatchResult {
        if !votes.votes_for.contains(&who) {
            votes.votes_for.push(who.clone());
            <Votes<T>>::insert(votes.hash, votes.clone());
            Self::deposit_event(RawEvent::VoteFor(votes.hash, who.clone()));

            if votes.votes_for.len() == <ValidatorThreshold>::get() as usize {
                Self::finalize_transfer(votes)?
            } else if votes.votes_for.len() > <ValidatorThreshold>::get() as usize {
                Err(Error::<T>::ProposalAlreadyComplete)?
            }
            Ok(())
        } else {
            Err(Error::<T>::ValidatorAlreadyVoted)?
        }
    }

    /// Note: Existence of proposal must be checked before calling
    fn vote_against(
        who: T::AccountId,
        mut votes: ProposalVotes<T::AccountId, T::Hash>,
    ) -> DispatchResult {
        if !votes.votes_against.contains(&who) {
            votes.votes_against.push(who.clone());
            <Votes<T>>::insert(votes.hash, votes.clone());
            Self::deposit_event(RawEvent::VoteAgainst(votes.hash, who.clone()));

            if votes.votes_against.len() > <ValidatorThreshold>::get() as usize {
                Self::cancel_transfer(votes.hash)?
            }
            Ok(())
        } else {
            Err(Error::<T>::ValidatorAlreadyVoted)?
        }
    }

    fn finalize_transfer(votes: ProposalVotes<T::AccountId, T::Hash>) -> DispatchResult {
        Self::deposit_event(RawEvent::ProposalSucceeded(votes.hash));
        let prop = <Proposals<T>>::get(votes.hash).unwrap();
        prop.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into())
        // match result {
        //     Ok(res) => Ok(res),
        //     Err(_) => Err(Error::<T>::DebugInnerCallFailed.into()),
        // }
    }

    fn cancel_transfer(prop_id: T::Hash) -> DispatchResult {
        // TODO: Incomplete
        Self::deposit_event(RawEvent::ProposalFailed(prop_id));
        Ok(())
    }
}
