// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use chainbridge as bridge;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult};
use frame_system::{self as system, ensure_signed, RawOrigin};
use sp_runtime::traits::EnsureOrigin;
use sp_std::prelude::*;

mod mock;
mod tests;

pub trait Trait: system::Trait + bridge::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type BridgeOrigin: EnsureOrigin<Self::Origin>;
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Trait>::Hash,
    {
        Remark(Hash),
    }
}

// decl_error! {
//     pub enum Error for Module<T: Trait> {
//
//     }
// }

// decl_storage! {
//     trait Store for Module<T: Trait> as Bridge {}
// }

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Default method for emitting events
        fn deposit_event() = default;

        pub fn transfer_hash(origin, hash: T::Hash, recipient: Vec<u8>) -> DispatchResult {
            ensure_signed(origin)?;
            //dest_id: Vec<u8>, to: Vec<u8>, token_id: Vec<u8>, metadata: Vec<u8>
            let dest_id = vec![1];
            let token_id = vec![1];
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <bridge::Module<T>>::receive_asset(RawOrigin::Root.into(), dest_id, recipient, token_id, metadata)
        }

        /// This can be called by the bridge to demonstrate an arbitrary call from a proposal.
        pub fn remark(origin, hash: T::Hash) -> DispatchResult {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(RawEvent::Remark(hash));
            Ok(())
        }
    }
}
