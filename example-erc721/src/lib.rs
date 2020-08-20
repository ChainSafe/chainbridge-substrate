// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::Get,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_core::U256;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

mod mock;
mod tests;

type TokenId = U256;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Erc721Token {
    pub id: TokenId,
    pub metadata: Vec<u8>,
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Some identifier for this token type, possibly the originating ethereum address.
    /// This is not explicitly used for anything, but may reflect the bridge's notion of resource ID.
    type Identifier: Get<[u8; 32]>;
}

decl_event! {
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
    {
        /// New token created
        Minted(AccountId, TokenId),
        /// Token transfer between two parties
        Transferred(AccountId, AccountId, TokenId),
        /// Token removed from the system
        Burned(TokenId),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// ID not recognized
        TokenIdDoesNotExist,
        /// Already exists with an owner
        TokenAlreadyExists,
        /// Origin is not owner
        NotOwner,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as TokenStorage {
        /// Maps tokenId to Erc721 object
        Tokens get(fn tokens): map hasher(opaque_blake2_256) TokenId => Option<Erc721Token>;
        /// Maps tokenId to owner
        TokenOwner get(fn owner_of): map hasher(opaque_blake2_256) TokenId => Option<T::AccountId>;
        /// Total number of tokens in existence
        TokenCount get(fn token_count): U256 = U256::zero();
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        /// Creates a new token with the given token ID and metadata, and gives ownership to owner
        #[weight = 195_000_000]
        pub fn mint(origin, owner: T::AccountId, id: TokenId, metadata: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;

            Self::mint_token(owner, id, metadata)?;

            Ok(())
        }

        /// Changes ownership of a token sender owns
        #[weight = 195_000_000]
        pub fn transfer(origin, to: T::AccountId, id: TokenId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::transfer_from(sender, to, id)?;

            Ok(())
        }

        /// Remove token from the system
        #[weight = 195_000_000]
        pub fn burn(origin, id: TokenId) -> DispatchResult {
            ensure_root(origin)?;

            let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;

            Self::burn_token(owner, id)?;

            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// Creates a new token in the system.
    pub fn mint_token(owner: T::AccountId, id: TokenId, metadata: Vec<u8>) -> DispatchResult {
        ensure!(!Tokens::contains_key(id), Error::<T>::TokenAlreadyExists);

        let new_token = Erc721Token { id, metadata };

        <Tokens>::insert(&id, new_token);
        <TokenOwner<T>>::insert(&id, owner.clone());
        let new_total = <TokenCount>::get().saturating_add(U256::one());
        <TokenCount>::put(new_total);

        Self::deposit_event(RawEvent::Minted(owner, id));

        Ok(())
    }

    /// Modifies ownership of a token
    pub fn transfer_from(from: T::AccountId, to: T::AccountId, id: TokenId) -> DispatchResult {
        // Check from is owner and token exists
        let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
        ensure!(owner == from, Error::<T>::NotOwner);
        // Update owner
        <TokenOwner<T>>::insert(&id, to.clone());

        Self::deposit_event(RawEvent::Transferred(from, to, id));

        Ok(())
    }

    /// Deletes a token from the system.
    pub fn burn_token(from: T::AccountId, id: TokenId) -> DispatchResult {
        let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
        ensure!(owner == from, Error::<T>::NotOwner);

        <Tokens>::remove(&id);
        <TokenOwner<T>>::remove(&id);
        let new_total = <TokenCount>::get().saturating_sub(U256::one());
        <TokenCount>::put(new_total);

        Self::deposit_event(RawEvent::Burned(id));

        Ok(())
    }
}
