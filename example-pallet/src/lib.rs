// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use chainbridge as bridge;
use example_erc721 as erc721;
use frame_support::ensure;
use frame_support::traits::{Currency, EnsureOrigin, Get};
use frame_system::ensure_signed;
use pallet_contracts::Pallet as Contracts;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::crypto::UncheckedFrom;
use sp_core::U256;
use sp_std::prelude::*;

mod mock;
mod tests;

mod constants {
    use hex_literal::hex;
    /// The code hash of the contract that will be instantiated. Get it from metadata.json of the contract.
    pub const CONTRACT_CODE_HASH: [u8; 32] =
        hex!("d174dc6e68f6f23eedb52608e204239ed01a317f990bec0c05f48c721d34823d");
    /// The selector of the message to call
    pub const MINT_SELECTOR: [u8; 4] = hex!("CAFEBABE");
}

type ResourceId = bridge::ResourceId;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config:
        frame_system::Config + bridge::Config + erc721::Config + pallet_contracts::Config
    where
        Self::AccountId: UncheckedFrom<Self::Hash> + AsRef<[u8]>,
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
        type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

        /// The currency mechanism.
        type Currency: Currency<Self::AccountId>;

        /// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
        type HashId: Get<ResourceId>;
        type NativeTokenId: Get<ResourceId>;
        type Erc721Id: Get<ResourceId>;
        type Deployer: Get<Self::AccountId>;
        type ContractAddress: Get<Self::AccountId>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        T::AccountId: UncheckedFrom<T::Hash>,
        T::AccountId: AsRef<[u8]>,
    {
    }

    #[pallet::event]
    #[pallet::metadata(<T as frame_system::Config>::Hash = "Hash")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config>
    where
        T::AccountId: UncheckedFrom<T::Hash>,
        T::AccountId: AsRef<[u8]>,
    {
        Remark(<T as frame_system::Config>::Hash),
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidTransfer,
    }
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        T::AccountId: UncheckedFrom<T::Hash>,
        T::AccountId: AsRef<[u8]>,
    {
        //
        // Initiation calls. These start a bridge transfer.
        //

        /// Transfers an arbitrary hash to a (whitelisted) destination chain.
        #[pallet::weight(195_000_000)]
        pub fn transfer_hash(
            origin: OriginFor<T>,
            hash: T::Hash,
            dest_id: bridge::ChainId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let resource_id = T::HashId::get();
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <bridge::Module<T>>::transfer_generic(dest_id, resource_id, metadata);
            Ok(().into())
        }

        /// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
        #[pallet::weight(195_000_000)]
        pub fn transfer_native(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: bridge::ChainId,
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;
            ensure!(
                <bridge::Module<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );
            let bridge_id = <bridge::Module<T>>::account_id();
            // T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;

            let resource_id = T::NativeTokenId::get();
            <bridge::Module<T>>::transfer_fungible(
                dest_id,
                resource_id,
                recipient,
                U256::from(amount.saturated_into::<u128>()),
            );
            Ok(().into())
        }

        /// Transfer a non-fungible token (erc721) to a (whitelisted) destination chain.
        #[pallet::weight(195_000_000)]
        pub fn transfer_erc721(
            origin: OriginFor<T>,
            recipient: Vec<u8>,
            token_id: U256,
            dest_id: bridge::ChainId,
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;
            ensure!(
                <bridge::Module<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );
            match <erc721::Module<T>>::tokens(&token_id) {
                Some(token) => {
                    <erc721::Module<T>>::burn_token(source, token_id)?;
                    let resource_id = T::Erc721Id::get();
                    let tid: &mut [u8] = &mut [0; 32];
                    token_id.to_big_endian(tid);
                    <bridge::Module<T>>::transfer_nonfungible(
                        dest_id,
                        resource_id,
                        tid.to_vec(),
                        recipient,
                        token.metadata,
                    );
                    Ok(().into())
                }
                None => Err(Error::<T>::InvalidTransfer)?,
            }
        }

        //
        // Executable calls. These can be triggered by a bridge transfer initiated on another chain
        //

        /// Executes a simple currency transfer using the bridge account as the source
        #[pallet::weight(195_000_000)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: BalanceOf<T>,
            r_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            use constants::*;
            use core::array::IntoIter;
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            // <T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;

            // Retrieve sender of the transaction.
            // let who = ensure_signed(origin)?;

            // // convert the code hash to `Hash` type
            // // TODO: This should be moved to the genesis config I think?
            let mut code_hash = T::Hash::default();
            code_hash.as_mut().copy_from_slice(&CONTRACT_CODE_HASH);

            // // generate the address for the contract
            // let contract_address =
            //     <Contracts<T>>::contract_address(&T::Deployer::get(), &code_hash, &[]);

            let contract_address = T::ContractAddress::get();
            debug::info!("contract_address: {:x?}", contract_address);

            let result = <Contracts<T>>::bare_call(
                source,
                contract_address,
                0_u32.into(),
                1000_000,
                IntoIter::new(MINT_SELECTOR)
                    .chain(AsRef::<[u8]>::as_ref(&to).to_vec())
                    .chain(amount.encode())
                    .collect(),
            );

            if let Err(e) = result.exec_result {
                debug::error!("erc20 contract error: {:?}", e);
            } else {
                debug::info!("call succeeded {:?}", result);
            }

            Ok(().into())
        }

        /// This can be called by the bridge to demonstrate an arbitrary call from a proposal.
        #[pallet::weight(195_000_000)]
        pub fn remark(
            origin: OriginFor<T>,
            hash: T::Hash,
            r_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(Event::Remark(hash));
            Ok(().into())
        }

        /// Allows the bridge to issue new erc721 tokens
        #[pallet::weight(195_000_000)]
        pub fn mint_erc721(
            origin: OriginFor<T>,
            recipient: T::AccountId,
            id: U256,
            metadata: Vec<u8>,
            r_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            T::BridgeOrigin::ensure_origin(origin)?;
            <erc721::Module<T>>::mint_token(recipient, id, metadata)?;
            Ok(().into())
        }
    }
}
