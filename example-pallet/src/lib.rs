// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use chainbridge as bridge;
use frame_support::traits::{Currency, ExistenceRequirement::AllowDeath};
use frame_support::{decl_event, decl_module, dispatch::DispatchResult};
use frame_system::{self as system, ensure_signed, RawOrigin};
use sp_runtime::traits::EnsureOrigin;
use sp_std::prelude::*;

mod mock;
mod tests;

pub trait Trait: system::Trait + bridge::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Trait>::Hash,
    {
        Remark(Hash),
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Transfers an arbitrary hash to some recipient on a (whitelisted) destination chain.
        pub fn transfer_hash(origin, hash: T::Hash, recipient: Vec<u8>, dest_id: u32) -> DispatchResult {
            ensure_signed(origin)?;

            let token_id = vec![1];
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <bridge::Module<T>>::receive_asset(RawOrigin::Root.into(), dest_id, recipient, token_id, metadata)
        }

        /// Executes a simple currency transfer using the bridge account as the source
        pub fn transfer(origin, to: T::AccountId, amount: u32) -> DispatchResult {
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            T::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
            Ok(())
        }

        /// This can be called by the bridge to demonstrate an arbitrary call from a proposal.
        pub fn remark(origin, hash: T::Hash) -> DispatchResult {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(RawEvent::Remark(hash));
            Ok(())
        }
    }
}
