// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use chainbridge as bridge;
use example_erc721 as erc721;
use frame_support::traits::{Currency, EnsureOrigin, ExistenceRequirement::AllowDeath, Get};
use frame_support::{decl_error, decl_event, decl_module, dispatch::DispatchResult, ensure};
use frame_system::{self as system, ensure_signed};
use pallet_contracts::Pallet as Contracts;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::crypto::{UncheckedFrom, Wraps};
use sp_core::U256;
use sp_std::prelude::*;

mod mock;
mod tests;

mod constants {
    use hex_literal::hex;

    /// The code hash of the contract that will be instantiated. Get it from metadata.json of the contract.
    pub const CONTRACT_CODE_HASH: [u8; 32] =
        hex!("ffd5772ad72d1305cf60c0be50bcd7ac172f3f53c08b7df65cc99ab85c8c44aa");
    /// The selector of the message to call
    pub const SELECTOR: [u8; 4] = hex!("ae04b6d1");
}

type ResourceId = bridge::ResourceId;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait Config:
    system::Config + bridge::Config + erc721::Config + pallet_contracts::Config
{
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
    type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

    /// The currency mechanism.
    type Currency: Currency<Self::AccountId>;

    /// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
    type HashId: Get<ResourceId>;
    type NativeTokenId: Get<ResourceId>;
    type Erc721Id: Get<ResourceId>;
}

decl_event! {
    pub enum Event<T> where <T as frame_system::Config>::Hash
    {
        Remark(Hash),
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        InvalidTransfer,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        const HashId: ResourceId = T::HashId::get();
        const NativeTokenId: ResourceId = T::NativeTokenId::get();
        const Erc721Id: ResourceId = T::Erc721Id::get();

        fn deposit_event() = default;

        //
        // Initiation calls. These start a bridge transfer.
        //

        /// Transfers an arbitrary hash to a (whitelisted) destination chain.
        #[weight = 195_000_000]
        pub fn transfer_hash(origin, hash: T::Hash, dest_id: bridge::ChainId) -> DispatchResult {
            ensure_signed(origin)?;

            let resource_id = T::HashId::get();
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <bridge::Module<T>>::transfer_generic(dest_id, resource_id, metadata)
        }

        /// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
        #[weight = 195_000_000]
        pub fn transfer_native(origin, amount: BalanceOf<T>, recipient: Vec<u8>, dest_id: bridge::ChainId) -> DispatchResult {
            let source = ensure_signed(origin)?;
            ensure!(<bridge::Module<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
            let bridge_id = <bridge::Module<T>>::account_id();
            // T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;

            let resource_id = T::NativeTokenId::get();
            <bridge::Module<T>>::transfer_fungible(dest_id, resource_id, recipient, U256::from(amount.saturated_into::<u128>()))
        }

        /// Transfer a non-fungible token (erc721) to a (whitelisted) destination chain.
        #[weight = 195_000_000]
        pub fn transfer_erc721(origin, recipient: Vec<u8>, token_id: U256, dest_id: bridge::ChainId) -> DispatchResult {
            let source = ensure_signed(origin)?;
            ensure!(<bridge::Module<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
            match <erc721::Module<T>>::tokens(&token_id) {
                Some(token) => {
                    <erc721::Module<T>>::burn_token(source, token_id)?;
                    let resource_id = T::Erc721Id::get();
                    let tid: &mut [u8] = &mut[0; 32];
                    token_id.to_big_endian(tid);
                    <bridge::Module<T>>::transfer_nonfungible(dest_id, resource_id, tid.to_vec(), recipient, token.metadata)
                }
                None => Err(Error::<T>::InvalidTransfer)?
            }
        }

        //
        // Executable calls. These can be triggered by a bridge transfer initiated on another chain
        //

        /// Executes a simple currency transfer using the bridge account as the source
        #[weight = 195_000_000]
        pub fn transfer(origin, to: T::AccountId, amount: BalanceOf<T>, r_id: ResourceId) -> DispatchResult {
            use core::array::IntoIter;
            use constants::*;
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            <T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;

            /*
            // Retrieve sender of the transaction.
            let who = ensure_signed(origin)?;

            // // convert the code hash to `Hash` type
            // // TODO: This should be moved to the genesis config I think?
            let mut code_hash = T::Hash::default();
            code_hash.as_mut().copy_from_slice(&CONTRACT_CODE_HASH);

            // // generate the address for the contract
            let contract_address = <Contracts<T>>::contract_address(&who, &code_hash, &[]);
            // // debug::info!("contract_address: {:x?}", contract_address);

            let result = <Contracts<T>>::bare_call(
                who,
                contract_address,
                0_u32.into(),
                10_000_000_000,
                IntoIter::new(SELECTOR)
                    .chain(AsRef::<[u8]>::as_ref(&source).to_vec())
                    .chain(AsRef::<[u8]>::as_ref(&to).to_vec())
                    // .chain(amount.clone())
                    .collect(),
            );
            */
            Ok(())
        }

        /// This can be called by the bridge to demonstrate an arbitrary call from a proposal.
        #[weight = 195_000_000]
        pub fn remark(origin, hash: T::Hash, r_id: ResourceId) -> DispatchResult {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(RawEvent::Remark(hash));
            Ok(())
        }

        /// Allows the bridge to issue new erc721 tokens
        #[weight = 195_000_000]
        pub fn mint_erc721(origin, recipient: T::AccountId, id: U256, metadata: Vec<u8>, r_id: ResourceId) -> DispatchResult {
            T::BridgeOrigin::ensure_origin(origin)?;
            <erc721::Module<T>>::mint_token(recipient, id, metadata)?;
            Ok(())
        }
    }
}
