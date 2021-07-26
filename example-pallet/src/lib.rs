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

//! # Example pallet

// Ensure we're `no_std` when compiling for WebAssembly.
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
mod traits;

// Pallet extrinsics weight information
mod weights;

use pallet_example_erc721 as erc721;

use frame_support::{
    dispatch::DispatchResultWithPostInfo,
    ensure,
    traits::{
        Currency, 
        EnsureOrigin, 
        ExistenceRequirement::AllowDeath, 
        Get,
    }
};

use frame_system::{
    ensure_signed
};

use sp_arithmetic::traits::SaturatedConversion;
use sp_core::U256;
use sp_std::prelude::*;

use chainbridge::types::{
    ResourceId, 
    ChainId
};

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

use crate::traits::WeightInfo;

// Re-export pallet components in crate namespace (for runtime construction)
pub use pallet::*;


// ----------------------------------------------------------------------------
// Pallet module
// ----------------------------------------------------------------------------

// Example pallet module
//
// The name of the pallet is provided by `construct_runtime` and is used as
// the unique identifier for the pallet's storage. It is not defined in the 
// pallet itself.
#[frame_support::pallet]
pub mod pallet {

    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    // Example pallet type declaration.
    //
    // This structure is a placeholder for traits and functions implementation
    // for the pallet.
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // ------------------------------------------------------------------------
    // Pallet configuration
    // ------------------------------------------------------------------------

    /// Example pallet's configuration trait.
    ///
    /// Associated types and constants are declared in this trait. If the pallet
    /// depends on other super-traits, the latter must be added to this trait, 
    /// such as, in this case, [`chainbridge::Config`] super-trait, for instance. 
    /// Note that [`frame_system::Config`] must always be included.
    #[pallet::config]
    pub trait Config: frame_system::Config + chainbridge::Config + erc721::Config {

        /// Associated type for Event enum
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
        type BridgeOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin, Success = <Self as frame_system::Config>::AccountId>;

        /// The currency mechanism.
        type Currency: Currency<<Self as frame_system::Config>::AccountId>;

        /// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
        type HashId: Get<ResourceId>;

        type NativeTokenId: Get<ResourceId>;
        
        type Erc721Id: Get<ResourceId>;

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
        Remark(<T as frame_system::Config>::Hash),
    }


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
        InvalidTransfer,
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

        //
        // Initiation calls. These start a bridge transfer.
        //

        /// Transfers an arbitrary hash to a (whitelisted) destination chain.
        #[pallet::weight(<T as pallet::Config>::WeightInfo::transfer_hash())]
        pub fn transfer_hash(
            origin: OriginFor<T>,
            hash: <T as frame_system::Config>::Hash, 
            dest_id: ChainId
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let resource_id = T::HashId::get();
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <chainbridge::Pallet<T>>::transfer_generic(dest_id, resource_id, metadata)?;
            Ok(().into())
        }

        /// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
        #[pallet::weight(<T as pallet::Config>::WeightInfo::transfer_native())]
        pub fn transfer_native(
            origin: OriginFor<T>,
            amount: BalanceOf<T>, 
            recipient: Vec<u8>, 
            dest_id: ChainId) -> DispatchResultWithPostInfo
        {
            let source = ensure_signed(origin)?;
            ensure!(<chainbridge::Pallet<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
            let bridge_id = <chainbridge::Pallet<T>>::account_id();
            T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;

            let resource_id = T::NativeTokenId::get();
            <chainbridge::Pallet<T>>::transfer_fungible(dest_id, resource_id, recipient, U256::from(amount.saturated_into::<u128>()))?;
            
            Ok(().into())
        }

        /// Transfer a non-fungible token (erc721) to a (whitelisted) destination chain.
        #[pallet::weight(<T as pallet::Config>::WeightInfo::transfer_erc721())]
        pub fn transfer_erc721(
            origin: OriginFor<T>,
            recipient: Vec<u8>,
            token_id: U256,
            dest_id: ChainId) -> DispatchResultWithPostInfo
        {
            let source = ensure_signed(origin)?;
            ensure!(<chainbridge::Pallet<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
            match <erc721::Pallet<T>>::get_tokens(&token_id) {
                Some(token) => {
                    <erc721::Pallet<T>>::burn_token(source, token_id)?;
                    let resource_id = T::Erc721Id::get();
                    let tid: &mut [u8] = &mut[0; 32];
                    token_id.to_big_endian(tid);
                    <chainbridge::Pallet<T>>::transfer_nonfungible(dest_id, resource_id, tid.to_vec(), recipient, token.metadata)?;
                    Ok(().into())
                }
                None => Err(Error::<T>::InvalidTransfer)?
            }
        }

        //
        // Executable calls. These can be triggered by a bridge transfer initiated on another chain
        //

        /// Executes a simple currency transfer using the bridge account as the source
        #[pallet::weight(<T as pallet::Config>::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            to: <T as frame_system::Config>::AccountId,
            amount: BalanceOf<T>,
            _r_id: ResourceId) -> DispatchResultWithPostInfo
        {
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            <T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
            Ok(().into())
        }

        /// This can be called by the bridge to demonstrate an arbitrary call from a proposal.
        #[pallet::weight(<T as pallet::Config>::WeightInfo::remark())]
        pub fn remark(
            origin: OriginFor<T>,
            hash: <T as frame_system::Config>::Hash,
            _r_id: ResourceId) -> DispatchResultWithPostInfo
        {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(Event::Remark(hash));
            Ok(().into())
        }

        /// Allows the bridge to issue new erc721 tokens
        #[pallet::weight(<T as pallet::Config>::WeightInfo::mint_erc721())]
        pub fn mint_erc721(
            origin: OriginFor<T>,
            recipient: <T as frame_system::Config>::AccountId,
            id: U256,
            metadata: Vec<u8>,
            _r_id: ResourceId) -> DispatchResultWithPostInfo
        {
            T::BridgeOrigin::ensure_origin(origin)?;
            <erc721::Pallet<T>>::mint_token(recipient, id, metadata)?;
            Ok(().into())
        }
    }
} // end of 'pallet' module
