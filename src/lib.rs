#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, traits::Currency,
    traits::ExistenceRequirement::AllowDeath,
};
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_std::vec::Vec;
use codec::{Decode, Encode};

mod mock;
mod tests;

#[derive(Encode, Decode, Clone)]
struct TxCount {
    recv: u32,
    sent: u32,
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// The currency mechanism.
    type Currency: Currency<Self::AccountId>;
}

decl_event!(
    pub enum Event<T> where <T as frame_system::Trait>::Hash {
        // dest_id, deposit_id, to, token_id, metadata
        AssetTransfer(Vec<u8>, u32, Vec<u8>, Vec<u8>, Vec<u8>),
        UselessEvent(Hash),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Interactions with this chain is not permitted
        ChainNotWhitelisted
    }
}

decl_storage!(
    trait Store for Module<T: Trait> as Bridge {
        EmitterAddress: Vec<u8>;

        Chains: map
            hasher(blake2_256) Vec<u8>
            => Option<TxCount>;

        EndowedAccount get(fn endowed) config(): T::AccountId;
    }
);

decl_module!(
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

        /// Commits an asset transfer to the chain as an event to be acted on by the bridge.
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
);
