// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Example ERC721 pallet
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(warnings)]

pub use pallet::*;
pub use traits::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types {
    use codec::{Decode, Encode};
    use scale_info::TypeInfo;
    use sp_core::U256;
    use sp_runtime::RuntimeDebug;
    use sp_std::vec::Vec;

    pub type TokenId = U256;

    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
    pub struct Erc721Token {
        pub id: TokenId,
        pub metadata: Vec<u8>,
    }
}

mod traits {

    //! Traits used by the example ERC721 pallet.
    use frame_support::weights::Weight;

    // ----------------------------------------------------------------------------
    // Traits declaration
    // ----------------------------------------------------------------------------

    /// Weight information for example ERC721 pallet extrinsics
    ///
    /// Weights are calculated using runtime benchmarking features
    /// See [`benchmarking`] module for more information
    pub trait WeightInfo {
        fn mint() -> Weight;

        fn transfer() -> Weight;

        fn burn() -> Weight;
    }
}

#[frame_support::pallet]
pub mod pallet {
    use crate::traits::WeightInfo;
    use crate::types::{Erc721Token, TokenId};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::U256;
    use sp_std::vec::Vec;

    // Bridge pallet type declaration.
    //
    // This structure is a placeholder for traits and functions implementation
    // for the pallet.
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // ------------------------------------------------------------------------
    // Pallet configuration
    // ------------------------------------------------------------------------
    /// Example ERC721 pallet's configuration trait.
    ///
    /// Associated types and constants are declared in this trait. If the pallet
    /// depends on other super-traits, the latter must be added to this trait,
    /// such as, in this case, [`frame_system::Config`] super-trait, for instance.
    /// Note that [`frame_system::Config`] must always be included.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Associated type for Event enum
        type Event: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::Event>;

        /// Some identifier for this token type, possibly the originating ethereum address.
        /// This is not explicitly used for anything, but may reflect the bridge's notion of
        /// resource ID.
        type Identifier: Get<[u8; 32]>;

        /// Weight information for extrinsics in this pallet
        type WeightInfo: WeightInfo;
    }

    /// Maps tokenId to Erc721 object
    #[pallet::storage]
    #[pallet::getter(fn tokens)]
    pub type Tokens<T: Config> =
        StorageMap<_, Blake2_256, TokenId, Erc721Token, OptionQuery>;

    /// Maps tokenId to owner
    #[pallet::storage]
    #[pallet::getter(fn owner_of)]
    pub type TokenOwner<T: Config> =
        StorageMap<_, Blake2_256, TokenId, T::AccountId, OptionQuery>;

    /// Total number of tokens in existence
    #[pallet::storage]
    #[pallet::getter(fn token_count)]
    pub type TokenCount<T: Config> = StorageValue<_, U256, ValueQuery>;

    // ------------------------------------------------------------------------
    // Pallet events
    // ------------------------------------------------------------------------

    // The macro generates event metadata and derive Clone, Debug, Eq, PartialEq and Codec
    #[pallet::event]
    // The macro generates a function on Pallet to deposit an event
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New token created
        Minted(T::AccountId, TokenId),
        /// Token transfer between two parties
        Transferred(T::AccountId, T::AccountId, TokenId),
        /// Token removed from the system
        Burned(TokenId),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// ID not recognized
        TokenIdDoesNotExist,
        /// Already exists with an owner
        TokenAlreadyExists,
        /// Origin is not owner
        NotOwner,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates a new token with the given token ID and metadata, and gives ownership to owner
        //#[pallet::weight(<T as Config>::WeightInfo::mint())]
        #[pallet::weight(10_000)]
        pub fn mint(
            origin: OriginFor<T>,
            owner: T::AccountId,
            id: TokenId,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            Self::mint_token(owner, id, metadata)?;

            Ok(())
        }

        /// Changes ownership of a token sender owns
        #[pallet::weight(10_000)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            id: TokenId,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::transfer_from(sender, to, id)?;

            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn burn(origin: OriginFor<T>, id: TokenId) -> DispatchResult {
            ensure_root(origin)?;

            let owner =
                Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;

            Self::burn_token(owner, id)?;
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Creates a new token in the system.
        pub fn mint_token(
            owner: T::AccountId,
            id: TokenId,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            ensure!(
                !Tokens::<T>::contains_key(id),
                Error::<T>::TokenAlreadyExists
            );

            let new_token = Erc721Token { id, metadata };

            <Tokens<T>>::insert(&id, new_token);
            <TokenOwner<T>>::insert(&id, owner.clone());
            let new_total = <TokenCount<T>>::get().saturating_add(U256::one());
            <TokenCount<T>>::put(new_total);

            Self::deposit_event(Event::Minted(owner, id));
            Ok(())
        }

        /// Modified ownership of a token
        pub fn transfer_from(
            from: T::AccountId,
            to: T::AccountId,
            id: TokenId,
        ) -> DispatchResult {
            // check from is owner and token exists
            let owner =
                Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
            ensure!(owner == from, Error::<T>::NotOwner);
            // Update owner
            <TokenOwner<T>>::insert(&id, to.clone());

            Self::deposit_event(Event::Transferred(from, to, id));

            Ok(())
        }

        pub fn burn_token(from: T::AccountId, id: TokenId) -> DispatchResult {
            let owner =
                Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
            ensure!(owner == from, Error::<T>::NotOwner);

            <Tokens<T>>::remove(&id);
            <TokenOwner<T>>::remove(&id);
            let new_total = <TokenCount<T>>::get().saturating_sub(U256::one());
            <TokenCount<T>>::put(new_total);

            Self::deposit_event(Event::Burned(id));

            Ok(())
        }
    }
}
