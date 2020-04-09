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

pub type ChainId = u8;
pub type DepositNonce = u32;
pub type ResourceId = [u8; 32];

/// Helper function to concatenate a chain ID and some bytes to produce a resource ID.
/// The common format is (31 bytes unique ID + 1 byte chain ID).
pub fn derive_resource_id(chain: u8, id: &[u8]) -> ResourceId {
    let mut r_id: ResourceId = [0; 32];
    r_id[31] = chain; // last byte is chain id
    let range = if id.len() > 31 { 31 } else { id.len() }; // Use at most 31 bytes
    for i in 0..range {
        r_id[30 - i] = id[range - 1 - i]; // Ensure left padding for eth compatibility
    }
    return r_id;
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

        /// Transfer is available for relaying (dest_id, nonce, resource_id, to, metadata)
        Transfer(ChainId, DepositNonce, ResourceId, Vec<u8>, Vec<u8>),

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
        pub ChainIdentifier get(fn chain_id): ChainId;

        /// All whitelisted chains and their respective transaction counts
        Chains get(fn chains): map hasher(blake2_256) ChainId => Option<DepositNonce>;

        /// Number of votes required for a proposal to execute
        RelayerThreshold get(fn relayer_threshold): u32;

        /// Tracks current relayer set
        pub Relayers get(fn relayers): map hasher(blake2_256) T::AccountId => bool;

        /// Number of relayers in set
        pub RelayerCount get(fn relayer_count): u32;

        /// All known proposals.
        /// The key is the hash of the call and the deposit ID, to ensure it's unique.
        pub Votes get(fn votes):
            double_map hasher(blake2_256) ChainId, hasher(blake2_256) (DepositNonce, T::Proposal)
            => Option<ProposalVotes<T::AccountId>>;

        /// Utilized by the bridge software to map resource IDs to actual methods
        pub Resources get(fn resources):
            map hasher(blake2_256) ResourceId => Option<Vec<u8>>
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

        pub fn set_resource(origin, id: ResourceId, method: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;
            <Resources>::insert(id, method);
            Ok(())
        }

        pub fn remove_resource(origin, id: ResourceId) -> DispatchResult {
            ensure_root(origin)?;
            <Resources>::remove(id);
            Ok(())
        }

        /// Enables a chain ID as a destination for a bridge transfer
        pub fn whitelist_chain(origin, id: ChainId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            // Cannot whitelist this chain
            ensure!(id != Self::chain_id(), Error::<T>::InvalidChainId);

            Chains::insert(&id, 0);
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
        pub fn acknowledge_proposal(origin, nonce: DepositNonce, src_id: ChainId, call: Box<<T as Trait>::Proposal>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            ensure!(Self::chain_whitelisted(src_id), Error::<T>::ChainNotWhitelisted);
            Self::vote_for(who, nonce, src_id, call)
        }

        /// Votes against the proposal IFF it exists.
        /// (Note: Proposal cancellation not yet fully implemented)
        pub fn reject(origin, nonce: DepositNonce, src_id: ChainId, call: Box<<T as Trait>::Proposal>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            Self::vote_against(who, nonce, src_id, call)
        }

        /// Completes a transfer fromo the chain by emitting an event to be acted on by the
        /// bridge and updating the tx count for the respective chain.
        pub fn transfer(origin, dest_id: ChainId, resource_id: ResourceId, to: Vec<u8>, metadata: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::is_initialized(), Error::<T>::NotInitialized);

            // Ensure chain is whitelisted
            if let Some(mut nonce) = Chains::get(&dest_id) {
                // Increment counter, emit event and store
                nonce += 1;
                Self::deposit_event(RawEvent::Transfer(dest_id, nonce, resource_id, to, metadata));
                Chains::insert(&dest_id, nonce);
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

    fn chain_whitelisted(id: ChainId) -> bool {
        return Self::chains(id) != None;
    }

    /// Commits a vote in favour of the proposal and executes it if the vote threshold is met.
    fn vote_for(
        who: T::AccountId,
        nonce: DepositNonce,
        src_id: ChainId,
        prop: Box<T::Proposal>,
    ) -> DispatchResult {
        // Use default in the case it doesn't already exist
        let mut votes = <Votes<T>>::get(src_id, (nonce, prop.clone())).unwrap_or_default();

        if !votes.votes_for.contains(&who) {
            // Vote and store
            votes.votes_for.push(who.clone());
            <Votes<T>>::insert(src_id, (nonce, prop.clone()), votes.clone());

            Self::deposit_event(RawEvent::VoteFor(nonce, who.clone()));

            // Check if finalization is possible
            if votes.votes_for.len() == <RelayerThreshold>::get() as usize {
                Self::finalize_execution(nonce, prop)?
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
    fn vote_against(
        who: T::AccountId,
        nonce: DepositNonce,
        src_id: ChainId,
        prop: Box<T::Proposal>,
    ) -> DispatchResult {
        // Use default in the case it doesn't already exist
        let mut votes = <Votes<T>>::get(src_id, (nonce, prop.clone())).unwrap_or_default();

        if !votes.votes_against.contains(&who) {
            // Vote and store
            votes.votes_against.push(who.clone());
            <Votes<T>>::insert(src_id, (nonce, prop.clone()), votes.clone());

            Self::deposit_event(RawEvent::VoteAgainst(nonce, who.clone()));

            // Check if cancellation is possible
            if votes.votes_against.len()
                > (<RelayerCount>::get() - <RelayerThreshold>::get()) as usize
            {
                Self::cancel_execution(nonce)?
            }

            Ok(())
        } else {
            Err(Error::<T>::RelayerAlreadyVoted)?
        }
    }

    /// Execute the proposal and signals the result as an event
    fn finalize_execution(nonce: DepositNonce, call: Box<T::Proposal>) -> DispatchResult {
        Self::deposit_event(RawEvent::ProposalApproved(nonce));
        match call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into()) {
            Ok(_) => Self::deposit_event(RawEvent::ProposalSucceeded(nonce)),
            Err(_) => Self::deposit_event(RawEvent::ProposalFailed(nonce)),
        }
        Ok(())
    }

    /// Cancels a proposal (not yet implemented)
    fn cancel_execution(nonce: DepositNonce) -> DispatchResult {
        // TODO: Incomplete
        Self::deposit_event(RawEvent::ProposalRejected(nonce));
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
