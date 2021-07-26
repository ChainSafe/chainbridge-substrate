// Copyright 2021 Centrifuge Foundation (centrifuge.io).
// This file is part of Centrifuge chain project.

// Centrifuge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).

// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

//! # Example ERC721 pallet

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// ----------------------------------------------------------------------------
// Module imports and re-exports
// ----------------------------------------------------------------------------

// Mock runtime and unit test cases
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// Pallet types and traits
pub mod types;
pub mod traits;

// Pallet extrinsics weight information
mod weights;

// Substrate primitives
use frame_support::{
    dispatch::{
        DispatchResult,
        DispatchResultWithPostInfo,
    },
    ensure,
    traits::Get,
};

use frame_system::{
    ensure_root,
    ensure_signed
};

use sp_core::U256;

use sp_std::prelude::*;

use crate::{
    traits::WeightInfo,
    types::{
        TokenId,
        Erc721Token,
    }
};

// Re-export pallet components in crate namespace (for runtime construction)
pub use pallet::*;


// ----------------------------------------------------------------------------
// Pallet module
// ----------------------------------------------------------------------------

// Chain bridge pallet module
//
// The name of the pallet is provided by `construct_runtime` and is used as
// the unique identifier for the pallet's storage. It is not defined in the 
// pallet itself.
#[frame_support::pallet]
pub mod pallet {

    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

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
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Some identifier for this token type, possibly the originating ethereum address.
        /// This is not explicitly used for anything, but may reflect the bridge's notion of resource ID.
        type Identifier: Get<[u8; 32]>;
    
        /// Weight information for extrinsics in this pallet
        type WeightInfo: WeightInfo;
    }


    // ------------------------------------------------------------------------
    // Pallet events
    // ------------------------------------------------------------------------

    // The macro generates event metadata and derive Clone, Debug, Eq, PartialEq and Codec
    #[pallet::event]
    // The macro generates a function on Pallet to deposit an event
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New token created
        Minted(<T as frame_system::Config>::AccountId, TokenId),
        /// Token transfer between two parties
        Transferred(<T as frame_system::Config>::AccountId, <T as frame_system::Config>::AccountId, TokenId),
        /// Token removed from the system
        Burned(TokenId),
    }

    // ------------------------------------------------------------------------
    // Pallet storage items
    // ------------------------------------------------------------------------
    
    /// Maps tokenId to Erc721 object
    #[pallet::storage]
    #[pallet::getter(fn get_tokens)]
    pub(super) type Tokens<T: Config> = StorageMap<
        _,
        Blake2_256,
        TokenId,
        Erc721Token,
        OptionQuery
    >;
    
    /// Maps tokenId to owner
    #[pallet::storage]
    #[pallet::getter(fn get_owner_of)]
    pub(super) type TokenOwner<T: Config> = StorageMap<
        _,
        Blake2_256,
        TokenId,
        T::AccountId,
        OptionQuery
    >;

    // Default (or initial) value for [`TokenCount`] storage item
	#[pallet::type_value]
	pub fn OnTokenCountEmpty<T: Config>() -> U256 {
		U256::zero()
	}

    /// Total number of tokens in existence
    #[pallet::storage]
    #[pallet::getter(fn get_token_count)]
    pub(super) type TokenCount<T: Config> = StorageValue<
        _,
        U256,
        ValueQuery,
        OnTokenCountEmpty<T>
    >;

	
    // ------------------------------------------------------------------------
	// Pallet genesis configuration
	// ------------------------------------------------------------------------

	// The genesis configuration type.
	#[pallet::genesis_config]
	pub struct GenesisConfig {}

	// The default value for the genesis config type.
	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {}
		}
	}

	// The build of genesis for the pallet.
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {}
	}


    // ------------------------------------------------------------------------
    // Pallet lifecycle hooks
    // ------------------------------------------------------------------------
    
    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}


    // ------------------------------------------------------------------------
    // Pallet errors
    // ------------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// ID not recognized
        TokenIdDoesNotExist,
        /// Already exists with an owner
        TokenAlreadyExists,
        /// Origin is not owner
        NotOwner,
    }


	// ------------------------------------------------------------------------
	// Pallet dispatchable functions
	// ------------------------------------------------------------------------

	// Declare Call struct and implement dispatchable (or callable) functions.
	//
	// Dispatchable functions are transactions modifying the state of the chain. They
	// are also called extrinsics are constitute the pallet's public interface.
	// Note that each parameter used in functions must implement `Clone`, `Debug`,
	// `Eq`, `PartialEq` and `Codec` traits.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

        /// Creates a new token with the given token ID and metadata, and gives ownership to owner
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        pub fn mint(
            origin: OriginFor<T>,
            owner: T::AccountId, 
            id: TokenId, metadata: Vec<u8>) -> DispatchResultWithPostInfo 
        {
            ensure_root(origin)?;

            Self::mint_token(owner, id, metadata)?;

            Ok(().into())
        }

        /// Changes ownership of a token sender owns
        #[pallet::weight(<T as Config>::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId, 
            id: TokenId) -> DispatchResultWithPostInfo 
        {
            let sender = ensure_signed(origin)?;

            Self::transfer_from(sender, to, id)?;

            Ok(().into())
        }

        /// Remove token from the system
        #[pallet::weight(<T as Config>::WeightInfo::burn())]
        pub fn burn(
            origin: OriginFor<T>,
            id: TokenId) -> DispatchResultWithPostInfo 
        {
            ensure_root(origin)?;

            let owner = Self::get_owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;

            Self::burn_token(owner, id)?;

            Ok(().into())
        }
    }
} // end of 'pallet' module


// ----------------------------------------------------------------------------
// Pallet implementation block
// ----------------------------------------------------------------------------

// Example ERC721 pallet implementation block.
//
// This main implementation block contains two categories of functions, namely:
// - Public functions: These are functions that are `pub` and generally fall into
//   inspector functions that do not write to storage and operation functions that do.
// - Private functions: These are private helpers or utilities that cannot be called
//   from other pallets.
impl<T: Config> Pallet<T> {

    /// Creates a new token in the system.
    pub fn mint_token(owner: T::AccountId, id: TokenId, metadata: Vec<u8>) -> DispatchResult {
        ensure!(!<Tokens<T>>::contains_key(id), Error::<T>::TokenAlreadyExists);

        let new_token = Erc721Token { id, metadata };

        <Tokens<T>>::insert(&id, new_token);
        <TokenOwner<T>>::insert(&id, owner.clone());
        let new_total = Self::get_token_count().saturating_add(U256::one());
        <TokenCount<T>>::put(new_total);

        Self::deposit_event(Event::Minted(owner, id));

        Ok(())
    }

    /// Modifies ownership of a token
    pub fn transfer_from(from: T::AccountId, to: T::AccountId, id: TokenId) -> DispatchResult {
        // Check from is owner and token exists
        let owner = Self::get_owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
        ensure!(owner == from, Error::<T>::NotOwner);
        // Update owner
        <TokenOwner<T>>::insert(&id, to.clone());

        Self::deposit_event(Event::Transferred(from, to, id));

        Ok(())
    }

    /// Deletes a token from the system.
    pub fn burn_token(from: T::AccountId, id: TokenId) -> DispatchResult {
        let owner = Self::get_owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
        ensure!(owner == from, Error::<T>::NotOwner);

        <Tokens<T>>::remove(&id);
        <TokenOwner<T>>::remove(&id);
        let new_total = Self::get_token_count().saturating_sub(U256::one());
        <TokenCount<T>>::put(new_total);

        Self::deposit_event(Event::Burned(id));

        Ok(())
    }
}