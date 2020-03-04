#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, ExistenceRequirement::AllowDeath},
    weights::GetDispatchInfo,
    Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::traits::{Dispatchable, EnsureOrigin};
use sp_runtime::{ModuleId, RuntimeDebug};
use sp_std::prelude::*;

use codec::{Decode, Encode};
use sp_core::U256;

mod mock;
mod tests;

// TODO: Could use as "endowed account":
// 	pub fn account_id() -> T::AccountId {
// 		MODULE_ID.into_account()
// 	}
const MODULE_ID: ModuleId = ModuleId(*b"py/bridg");

/// Tracks the transfer in/out of each respective chain
#[derive(Encode, Decode, Clone, Default)]
struct TxCount {
    recv: u32,
    sent: u32,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct TransferProposal<AccountId, Call> {
    votes_for: Vec<AccountId>,
    votes_against: Vec<AccountId>,
    call: Box<Call>,
}

impl<AccountId, Call> TransferProposal<AccountId, Call> {
    fn new(call: Box<Call>) -> Self {
        Self {
            votes_for: Vec::new(),
            votes_against: Vec::new(),
            call,
        }
    }
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// The currency mechanism.
    type Currency: Currency<Self::AccountId>;
    /// The origin used to manage who can modify the bridge configuration
    // type ValidatorOrigin: EnsureOrigin<Self::Origin>; // + From<frame_system::RawOrigin<Self>>;
    // type TransferCall: Parameter + Dispatchable<Origin=Self::ValidatorOrigin> + GetDispatchInfo;
    type Proposal: Parameter + Dispatchable<Origin = Self::Origin>;
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

        VoteFor(Hash, AccountId),
        VoteAgainst(Hash, AccountId),

        ProposalSuceeded(Hash),
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

        pub Proposals get(fn proposals):
            map hasher(blake2_256) T::Hash =>
            Option<TransferProposal<T::AccountId, <T as Trait>::Proposal>>;
    }
    add_extra_genesis {
        config(validators): Vec<T::AccountId>;
        build(|config| Module::<T>::initialize_validators(&config.validators));
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Default method for emitting events
        fn deposit_event() = default;

        /// Sets the address used to identify this chain
        pub fn set_address(origin, addr: Vec<u8>) -> DispatchResult {
            // TODO: Limit access
            Self::ensure_validator(origin)?;
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
        // TODO: Is the hash needed?
        pub fn create_proposal(origin, prop_id: T::Hash, call: Box<<T as Trait>::Proposal>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);
            // Make sure proposal doesn't already exist
            ensure!(!<Proposals<T>>::contains_key(prop_id), Error::<T>::ProposalAlreadyExists);

            let proposal = TransferProposal::new(call);

            <Proposals<T>>::insert(prop_id, proposal);

            // Creating a proposal also votes for it
            Self::vote_for(who, prop_id)
        }

        pub fn vote(origin, prop_id: T::Hash, in_favour: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_validator(&who), Error::<T>::ValidatorInvalid);

            // Check if proposal exists
            if let Some(votes) = <Proposals<T>>::get(prop_id) {
                // Vote if they haven't already
                if in_favour {
                    Self::vote_for(who, prop_id)?
                } else {
                    Self::vote_against(who, prop_id)?
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
            ensure_root(origin)?;
            let source: T::AccountId = <EndowedAccount<T>>::get();
            T::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
            Ok(())
        }

        pub fn mock_transfer(origin, n: u32) -> DispatchResult {
            let mock_data = Vec::new();
            Self::deposit_event(RawEvent::AssetTransfer(
                mock_data.clone(),
                n,
                mock_data.clone(),
                mock_data.clone(),
                mock_data.clone(),
            ));
            Ok(())
        }
    }

}

/// Main module declaration.
/// Here we should include non-state changing public funcs
impl<T: Trait> Module<T> {
    // TODO: Rename
    fn ensure_validator(origin: T::Origin) -> DispatchResult {
        // T::ValidatorOrigin::try_origin(origin)
        //     .map(|_| ())
        //     .or_else(ensure_root)?;
        ensure_root(origin)?;
        Ok(())
    }

    pub fn is_validator(who: &T::AccountId) -> bool {
        Self::validators(who)
    }

    fn initialize_validators(validators: &[T::AccountId]) {
        if !validators.is_empty() {
            for v in validators {
                <Validators<T>>::insert(v, true);
            }
        }
    }

    /// Note: Existence of proposal must be checked before calling
    fn vote_for(who: T::AccountId, prop_id: T::Hash) -> DispatchResult {
        let mut prop = <Proposals<T>>::get(&prop_id).unwrap();
        if !prop.votes_for.contains(&who) {
            prop.votes_for.push(who.clone());
            <Proposals<T>>::insert(&prop_id, prop.clone());
            Self::deposit_event(RawEvent::VoteFor(prop_id.clone(), who.clone()));

            if prop.votes_for.len() == <ValidatorThreshold>::get() as usize {
                Self::finalize_transfer(prop_id)?
            } else if prop.votes_for.len() > <ValidatorThreshold>::get() as usize {
                Err(Error::<T>::ProposalAlreadyComplete)?
            }
            Ok(())
        } else {
            Err(Error::<T>::ValidatorAlreadyVoted)?
        }
    }

    /// Note: Existence of proposal must be checked before calling
    fn vote_against(who: T::AccountId, prop_id: T::Hash) -> DispatchResult {
        let mut prop = <Proposals<T>>::get(&prop_id).unwrap();
        if !prop.votes_against.contains(&who) {
            prop.votes_against.push(who.clone());
            <Proposals<T>>::insert(&prop_id, prop.clone());
            Self::deposit_event(RawEvent::VoteAgainst(prop_id.clone(), who.clone()));

            if prop.votes_against.len() > <ValidatorThreshold>::get() as usize {
                Self::cancel_transfer(prop_id)?
            }
            Ok(())
        } else {
            Err(Error::<T>::ValidatorAlreadyVoted)?
        }
    }

    fn finalize_transfer(prop_id: T::Hash) -> DispatchResult {
        Self::deposit_event(RawEvent::ProposalSuceeded(prop_id));
        let prop = <Proposals<T>>::get(prop_id).unwrap();
        let result = prop.call.dispatch(frame_system::RawOrigin::Root.into());
        match result {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::<T>::DebugInnerCallFailed.into()),
        }
    }

    fn cancel_transfer(prop_id: T::Hash) -> DispatchResult {
        // TODO
        Self::deposit_event(RawEvent::ProposalFailed(prop_id));
        Ok(())
    }
}
