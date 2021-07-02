// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use chainbridge as bridge;
use codec::Encode;
use example_erc721 as erc721;
use frame_support::ensure;
use frame_support::traits::{Currency, EnsureOrigin, Get};
use frame_system::ensure_signed;
use pallet_contracts::Pallet as Contracts;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::crypto::UncheckedFrom;
use sp_core::{H160, U256};
use sp_std::prelude::*;

mod mock;
mod tests;

mod constants {
    use hex_literal::hex;
    /// The code hash of the contract that will be instantiated. Get it from metadata.json of the contract.
    pub const CONTRACT_CODE_HASH: [u8; 32] =
        hex!("352e27b91bca40ca114f84c11f443015683cbd19b39107570f1a1992bcc152be");
    /// The selector of the message to call
    pub const MINT_SELECTOR: [u8; 4] = hex!("cfdd9aa2");
    pub const BURN_SELECTOR: [u8; 4] = hex!("27212bbb");
    pub const APPROVE_SELECTOR: [u8; 4] = hex!("681266a0");
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
    use hex_literal::hex;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn address_mapping)]
    pub type AddressMapping<T: Config> =
        StorageMap<_, Blake2_128Concat, H160, ([u8; 32], [u8; 32])>;

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
        ContractNotCalled,
        ApproveFailed,
        BurnFailed,
        MintFailed,
        AddressMappingNotFound,
        AddressMappingAlreadyExisted,
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
            token_addr: H160,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: bridge::ChainId,
        ) -> DispatchResultWithPostInfo {
            use constants::*;
            use core::array::IntoIter;
            let source = ensure_signed(origin)?;
            ensure!(
                <bridge::Module<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );

            ensure!(
                AddressMapping::<T>::contains_key(token_addr.clone()),
                Error::<T>::AddressMappingNotFound
            );

            let bridge_id = <bridge::Module<T>>::account_id();

            let contract_code_hash =
                AddressMapping::<T>::get(token_addr.clone()).unwrap_or_default();

            let mut code_hash = T::Hash::default();
            code_hash.as_mut().copy_from_slice(&contract_code_hash.0);

            let contract_address = <Contracts<T>>::contract_address(
                &T::Deployer::get(),
                &code_hash,
                &contract_code_hash.1,
            );

            debug::info!(
                "contract_address: {:x?} {:?}",
                contract_address,
                amount.encode()
            );

            let result = <Contracts<T>>::bare_call(
                source.clone(),
                contract_address.clone(),
                0_u32.into(),
                100_000_000_000,
                IntoIter::new(APPROVE_SELECTOR)
                    .chain(AsRef::<[u8]>::as_ref(&bridge_id).to_vec())
                    .chain(amount.encode())
                    .collect(),
            );

            match result.exec_result {
                Err(e) => {
                    debug::error!("erc20 contract approve not called: {:?}", e);
                    Err(Error::<T>::ContractNotCalled)?
                }
                Ok(res) => {
                    if res.data != [0] {
                        debug::error!("erc20 contract approve call failed: {:?}", res.data);
                        Err(Error::<T>::ApproveFailed)?
                    } else {
                        debug::info!("approve succeeded {:?}", res);
                    }
                }
            };

            let result = <Contracts<T>>::bare_call(
                bridge_id,
                contract_address.clone(),
                0_u32.into(),
                100_000_000_000,
                IntoIter::new(BURN_SELECTOR)
                    .chain(AsRef::<[u8]>::as_ref(&source).to_vec())
                    .chain(amount.encode())
                    .collect(),
            );

            match result.exec_result {
                Err(e) => {
                    debug::error!("erc20 contract burn not called: {:?}", e);
                    Err(Error::<T>::ContractNotCalled)?
                }
                Ok(res) => {
                    if res.data != [0] {
                        debug::error!("erc20 contract burn call failed: {:?}", res.data);
                        Err(Error::<T>::BurnFailed)?
                    } else {
                        debug::info!("burn succeeded {:?}", res);
                    }
                }
            };

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
        #[pallet::weight(195_000_000_000)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            token_addr: H160,
            amount: BalanceOf<T>,
            r_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            use constants::*;
            use core::array::IntoIter;
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            // <T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;

            // Retrieve sender of the transaction.
            // let who = ensure_signed(origin)?;

            ensure!(
                AddressMapping::<T>::contains_key(token_addr.clone()),
                Error::<T>::AddressMappingNotFound
            );

            assert_eq!(
                hex!("87ad8fcfe229e7901b71a84971b07c6de93501dffce99a0bb4ac79ff32ba3e61"),
                [
                    137, 148, 222, 19, 228, 109, 183, 245, 144, 147, 109, 89, 87, 100, 70, 80, 198,
                    77, 103, 12, 72, 213, 13, 71, 152, 135, 102, 144, 4, 225, 88, 217
                ]
            );

            let contract_code_hash =
                AddressMapping::<T>::get(token_addr.clone()).unwrap_or_default();

            let mut code_hash = T::Hash::default();
            code_hash.as_mut().copy_from_slice(&contract_code_hash.0);

            let contract_address = <Contracts<T>>::contract_address(
                &T::Deployer::get(),
                &code_hash,
                &contract_code_hash.1,
            );
            debug::info!(
                "erc20 contract address: {:x?} {:?} {:?}",
                contract_address,
                token_addr,
                contract_code_hash
            );

            let result = <Contracts<T>>::bare_call(
                source,
                contract_address,
                0_u32.into(),
                100_000_000_000,
                IntoIter::new(MINT_SELECTOR)
                    .chain(AsRef::<[u8]>::as_ref(&to).to_vec())
                    .chain(amount.encode())
                    .collect(),
            );

            match result.exec_result {
                Err(e) => {
                    debug::error!("erc20 contract mint not called: {:?}", e);
                    Err(Error::<T>::ContractNotCalled)?
                }
                Ok(res) => {
                    if res.data != [0] {
                        debug::error!("erc20 contract mint call failed: {:?}", res.data);
                        Err(Error::<T>::MintFailed)?
                    } else {
                        debug::info!("mint succeeded {:?}", res);
                    }
                }
            };

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

        /// Add new token address pair
        #[pallet::weight(195_000_000)]
        pub fn add_address_mapping(
            origin: OriginFor<T>,
            eth_token_addr: H160,
            ink_contract_hash: [u8; 32],
            ink_contract_salt: [u8; 32],
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ensure!(
                !AddressMapping::<T>::contains_key(&eth_token_addr),
                Error::<T>::AddressMappingAlreadyExisted
            );
            AddressMapping::<T>::insert(&eth_token_addr, (ink_contract_hash, ink_contract_salt));
            Ok(().into())
        }

        /// Update new token address pair
        #[pallet::weight(195_000_000)]
        pub fn update_address_mapping(
            origin: OriginFor<T>,
            eth_token_addr: H160,
            ink_contract_hash: [u8; 32],
            ink_contract_salt: [u8; 32],
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ensure!(
                AddressMapping::<T>::contains_key(&eth_token_addr),
                Error::<T>::AddressMappingNotFound
            );
            AddressMapping::<T>::insert(&eth_token_addr, (ink_contract_hash, ink_contract_salt));
            Ok(().into())
        }

        /// Remove new token address pair
        #[pallet::weight(195_000_000)]
        pub fn remove_address_mapping(
            origin: OriginFor<T>,
            eth_token_addr: H160,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ensure!(
                AddressMapping::<T>::contains_key(&eth_token_addr),
                Error::<T>::AddressMappingNotFound
            );
            AddressMapping::<T>::remove(&eth_token_addr);
            Ok(().into())
        }
    }
}
