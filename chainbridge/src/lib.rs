// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::Currency, Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::traits::{AccountIdConversion, Dispatchable, EnsureOrigin};
use sp_runtime::{ModuleId, RuntimeDebug};
use sp_std::prelude::*;

use codec::{Decode, Encode, EncodeLike};

mod mock;
mod tests;

const MODULE_ID: ModuleId = ModuleId(*b"cb/bridg");

type ChainId = u32;

/// Tracks the transfer in/out of each respective chain
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
struct TxCount {
    recv: u32,
    sent: u32,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ProposalVotes<AccountId> {
    pub votes_for: Vec<AccountId>,
    pub votes_against: Vec<AccountId>,
}

impl<AccountId> Default for ProposalVotes<AccountId> {
    fn default() -> Self {
        Self {
            votes_for: vec![],
            votes_against: vec![],
        }
    }
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// The currency mechanism.
    type Currency: Currency<Self::AccountId>;
    /// Proposed dispatchable call
    type Proposal: Parameter + Dispatchable<Origin = Self::Origin> + EncodeLike;
}

decl_event! {
    pub enum Event<T> where <T as frame_system::Trait>::AccountId {
        /// Vote threshold has changed (new_threshold)
        RelayerThresholdChanged(u32),
        /// Chain now available for transfers (chain_id)
        ChainWhitelisted(ChainId),
        /// Relayer added to set
        RelayerAdded(AccountId),
        /// Relayer removed from set
        RelayerRemoved(AccountId),

        /// Asset transfer is available for relaying (dest_id, prop_id, to, token_id, metadata)
        AssetTransfer(ChainId, u32, Vec<u8>, Vec<u8>, Vec<u8>),

        /// Vote submitted in favour of proposal
        VoteFor(u32, AccountId),
        /// Vot submitted against proposal
        VoteAgainst(u32, AccountId),
        /// Voting successful for a proposal
        ProposalApproved(u32),
        /// Voting rejected a proposal
        ProposalRejected(u32),
        /// Execution of call succeeded
        ProposalSucceeded(u32),
        /// Execution of call failed
        ProposalFailed(u32),
    }
}

// TODO: Pass params to errors
decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Root must call `initialize` to set params
        NotInitialized,
        /// Initialization has already been done
        AlreadyInitialized,
        /// Relayer threshold not set
        ThresholdNotSet,
        /// Provided chain Id is not valid
        InvalidChainId,
        /// Relayer threshold cannot be 0
        InvalidThreshold,
        /// Interactions with this chain is not permitted
        ChainNotWhitelisted,
        /// Relayer already in set
        RelayerAlreadyExists,
        /// Provided accountId is not a relayer
        RelayerInvalid,
        /// Protected operation, much be performed by relayer
        MustBeRelayer,
        /// Relayer has already submitted some vote for this proposal
        RelayerAlreadyVoted,
        /// A proposal with these parameters has already been submitted
        ProposalAlreadyExists,
        /// No proposal with the ID was found
        ProposalDoesNotExist,
        /// Proposal has either failed or succeeded
        ProposalAlreadyComplete,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bridge {
        /// Whether the initialize function has been called (ie. chain ID and threshold set)
        Initialized get(fn is_initialized): bool;

        /// The ChainId for this chain.
        pub ChainIdentifier get(fn chain_id): u32;

        /// All whitelisted chains and their respective transaction counts
        Chains: map hasher(blake2_256) ChainId => Option<TxCount>;

        /// Number of votes required for a proposal to execute
        RelayerThreshold get(fn relayer_threshold): u32;

        /// Tracks current relayer set
        pub Relayers get(fn relayers): map hasher(blake2_256) T::AccountId => bool;

        /// Number of relayers in set
        pub RelayerCount get(fn relayer_count): u32;

        /// All known proposals.
        /// The key is the hash of the call and the deposit ID, to ensure it's unique.
        pub Votes get(fn votes):
            map hasher(blake2_256) (u32, T::Proposal)
            => Option<ProposalVotes<T::AccountId>>;

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Sets chain ID and threshold, and enables transfers
        pub fn initialize(origin, threshold: u32, chain_id: ChainId) {
            ensure_root(origin)?;
            ensure!(!Self::is_initialized(), Error::<T>::AlreadyInitialized);
            ensure!(threshold > 0, Error::<T>::InvalidThreshold);

            <ChainIdentifier>::put(chain_id);
            RelayerThreshold::put(threshold);
            <Initialized>::put(true);
        }

        /// Sets the vote threshold for proposals
        pub fn set_threshold(origin, threshold: u32) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(threshold > 0, Error::<T>::InvalidThreshold);
            RelayerThreshold::put(threshold);
            Self::deposit_event(RawEvent::RelayerThresholdChanged(threshold));
            Ok(())
        }

        /// Enables a chain ID as a destination for a bridge transfer
        pub fn whitelist_chain(origin, id: ChainId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            // Cannot whitelist this chain
            ensure!(id != Self::chain_id(), Error::<T>::InvalidChainId);

            Chains::insert(&id, TxCount { recv: 0, sent: 0 });
            Self::deposit_event(RawEvent::ChainWhitelisted(id));
            Ok(())
        }

        /// Adds a new relayer to the set. Errors if relayer already exists.
        pub fn add_relayer(origin, v: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(!Self::is_relayer(&v), Error::<T>::RelayerAlreadyExists);
            <Relayers<T>>::insert(&v, true);
            <RelayerCount>::mutate(|i| *i += 1);

            Self::deposit_event(RawEvent::RelayerAdded(v));
            Ok(())
        }

        /// Removes an existing relayer from the set. Errors if relayer doesn't exist.
        pub fn remove_relayer(origin, v: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(Self::is_relayer(&v), Error::<T>::RelayerInvalid);
            <Relayers<T>>::remove(&v);
            <RelayerCount>::mutate(|i| *i -= 1);
            Self::deposit_event(RawEvent::RelayerRemoved(v));
            Ok(())
        }

        /// Commits a vote in favour of the proposal. This may be called to initially create and
        /// vote for the proposal, or to simply vote.
        pub fn acknowledge_proposal(origin, prop_id: u32, call: Box<<T as Trait>::Proposal>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            Self::vote_for(who, prop_id, call)
        }

        /// Votes against the proposal IFF it exists.
        /// (Note: Proposal cancellation not yet fully implemented)
        pub fn reject(origin, prop_id: u32, call: Box<<T as Trait>::Proposal>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            Self::vote_against(who, prop_id, call)
        }

        /// Completes an asset transfer to the chain by emitting an event to be acted on by the
        /// bridge and updating the tx count for the respective chan.
        pub fn receive_asset(origin, dest_id: ChainId, to: Vec<u8>, token_id: Vec<u8>, metadata: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            // Ensure chain is whitelisted
            if let Some(mut counter) = Chains::get(&dest_id) {
                Self::deposit_event(RawEvent::AssetTransfer(dest_id, counter.recv, to, token_id, metadata));
                // Increment counter and store
                counter.recv += 1;
                Chains::insert(&dest_id, counter.clone());
                Ok(())
            } else {
                Err(Error::<T>::ChainNotWhitelisted)?
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// Checks if who is a relayer
    pub fn is_relayer(who: &T::AccountId) -> bool {
        Self::relayers(who)
    }

    /// Provides an AccountId for the pallet.
    /// This is used both as an origin check and deposit/withdrawal account.
    pub fn account_id() -> T::AccountId {
        MODULE_ID.into_account()
    }

    /// Commits a vote in favour of the proposal and executes it if the vote threshold is met.
    fn vote_for(who: T::AccountId, prop_id: u32, prop: Box<T::Proposal>) -> DispatchResult {
        // Use default in the case it doesn't already exist
        let mut votes = <Votes<T>>::get((prop_id, prop.clone())).unwrap_or_default();

        if !votes.votes_for.contains(&who) {
            // Vote and store
            votes.votes_for.push(who.clone());
            <Votes<T>>::insert((prop_id, prop.clone()), votes.clone());

            Self::deposit_event(RawEvent::VoteFor(prop_id, who.clone()));

            // Check if finalization is possible
            if votes.votes_for.len() == <RelayerThreshold>::get() as usize {
                Self::finalize_execution(prop_id, prop)?
            } else if votes.votes_for.len() > <RelayerThreshold>::get() as usize {
                Err(Error::<T>::ProposalAlreadyComplete)?
            }
            Ok(())
        } else {
            Err(Error::<T>::RelayerAlreadyVoted)?
        }
    }

    /// Commits a vote against the proposal and cancels it if more than (relayers.len() - threshold)
    /// votes against exist.
    fn vote_against(who: T::AccountId, prop_id: u32, prop: Box<T::Proposal>) -> DispatchResult {
        // Use default in the case it doesn't already exist
        let mut votes = <Votes<T>>::get((prop_id, prop.clone())).unwrap_or_default();

        if !votes.votes_against.contains(&who) {
            // Vote and store
            votes.votes_against.push(who.clone());
            <Votes<T>>::insert((prop_id, prop.clone()), votes.clone());

            Self::deposit_event(RawEvent::VoteAgainst(prop_id, who.clone()));

            // Check if cancellation is possible
            if votes.votes_against.len()
                > (<RelayerCount>::get() - <RelayerThreshold>::get()) as usize
            {
                Self::cancel_execution(prop_id)?
            }

            Ok(())
        } else {
            Err(Error::<T>::RelayerAlreadyVoted)?
        }
    }

    /// Execute the proposal and signals the result as an event
    fn finalize_execution(prop_id: u32, call: Box<T::Proposal>) -> DispatchResult {
        Self::deposit_event(RawEvent::ProposalApproved(prop_id));
        match call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into()) {
            Ok(_) => Self::deposit_event(RawEvent::ProposalSucceeded(prop_id)),
            Err(_) => Self::deposit_event(RawEvent::ProposalFailed(prop_id)),
        }
        Ok(())
    }

    /// Cancels a proposal (not yet implemented)
    fn cancel_execution(prop_id: u32) -> DispatchResult {
        // TODO: Incomplete
        Self::deposit_event(RawEvent::ProposalRejected(prop_id));
        Ok(())
    }
}

/// Simple ensure origin for the bridge account
pub struct EnsureBridge<T>(sp_std::marker::PhantomData<T>);
impl<T: Trait> EnsureOrigin<T::Origin> for EnsureBridge<T> {
    type Success = T::AccountId;
    fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
        let bridge_id = MODULE_ID.into_account();
        o.into().and_then(|o| match o {
            system::RawOrigin::Signed(who) if who == bridge_id => Ok(bridge_id),
            r => Err(T::Origin::from(r)),
        })
    }
}
