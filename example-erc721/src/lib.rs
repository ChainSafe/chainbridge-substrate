// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_core::U256;
use sp_runtime::RuntimeDebug;

mod tests;

type TokenId = U256;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Erc721Token {
    id: TokenId,
    metadata: Vec<u8>,
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
    {
        Minted(AccountId, U256),
        Transferred(AccountId, AccountId, TokenId),
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
        Tokens get(tokens): map hasher(blake2_256) TokenId => Option<Erc721Token>;
        /// Maps tokenId to owner
        TokenOwner get(owner_of): map hasher(blake2_256) TokenId => Option<T::AccountId>;

        TokenCount get(token_count): U256 = U256::zero();
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        fn mint(origin, owner: T::AccountId, id: TokenId, metadata: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;

            Self::mint_token(owner, id, metadata)?;

            Ok(())
        }

        fn transfer(origin, to: T::AccountId, id: TokenId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::transfer_from(sender, to, id)?;

            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn mint_token(owner: T::AccountId, id: TokenId, metadata: Vec<u8>) -> DispatchResult {
        ensure!(Tokens::get(id) == None, Error::<T>::TokenAlreadyExists);

        let new_token = Erc721Token { id, metadata };

        <Tokens>::insert(&id, new_token);
        <TokenOwner<T>>::insert(&id, owner.clone());
        let new_total = <TokenCount>::get().saturating_add(U256::one());
        <TokenCount>::put(new_total);

        Self::deposit_event(RawEvent::Minted(owner, id));

        Ok(())
    }

    fn transfer_from(from: T::AccountId, to: T::AccountId, id: TokenId) -> DispatchResult {
        // Check from is owner and token exists
        let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
        ensure!(owner == from, Error::<T>::NotOwner);
        // Update owner
        <TokenOwner<T>>::insert(&id, to.clone());

        Self::deposit_event(RawEvent::Transferred(from, to, id));

        Ok(())
    }
}
