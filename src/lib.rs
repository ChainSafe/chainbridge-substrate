#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use frame_system::{self as system, ensure_signed};
use sp_std::vec::Vec;
use codec::{Decode, Encode};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Counter(u32);

/// Tracks deposit count for an associated chain
impl Counter {
    fn increment(&mut self) {
        self.0 = self.0 + 1;
    }
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where <T as frame_system::Trait>::Hash {
        // dest_id, deposit_id, to, token_id, metadata
        AssetTransfer(Vec<u8>, u32, Vec<u8>, Vec<u8>, Vec<u8>),
        UselessEvent(Hash),
    }
);

decl_storage!(
    trait Store for Module<T: Trait> as Bridge {
        EmitterAddress get(emitter_address): Vec<u8>;
        Chains get(fn chains): map Vec<u8> => Counter;
    }
);

decl_module!(
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Default method for emitting events
        fn deposit_event() = default;

        /// Sets the address used to identify this chain
        pub fn set_address(origin, addr: Vec<u8>) -> DispatchResult {
            // TODO: Limit access
            ensure_signed(origin)?;
            <EmitterAddress>::put(addr);
            Ok(())
        }

        /// Enables a chain ID as a destination for a bridge transfer
        pub fn whitelist_chain(origin, id: Vec<u8>) -> DispatchResult {
            // TODO: Limit access
            ensure_signed(origin)?;
            <Chains>::insert(&id, Counter(0));
            Ok(())
        }

        /// Commits an asset transfer to the chain as an event to be acted on by the bridge.
        pub fn transfer_asset(origin, dest_id: Vec<u8>, to: Vec<u8>, token_id: Vec<u8>, metadata: Vec<u8>) -> DispatchResult {
            // TODO: Limit access
            ensure_signed(origin)?;
            // Ensure chain is whitelisted
            ensure!(<Chains>::exists(&dest_id), "Chain ID not whitelisted");
            let mut counter = <Chains>::get(&dest_id);
            Self::deposit_event(RawEvent::AssetTransfer(dest_id.clone(), counter.0, to, token_id, metadata));

            // Increment counter and store
            counter.increment();
            <Chains>::insert(&dest_id, counter);
            Ok(())
        }
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    use sp_core::H256;
    use sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, IdentityLookup},
        Perbill,
    };
    use frame_support::{assert_err, assert_ok, impl_outer_origin, parameter_types, weights::Weight};

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;

    type Bridge = super::Module<Test>;

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: Weight = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    }

    impl frame_system::Trait for Test {
        type Origin = Origin;
        type Call = ();
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
    }

    impl Trait for Test {
        type Event = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        t.into()
    }

    #[test]
    fn set_get_address() {
        new_test_ext().execute_with(|| {
            assert_ok!(Bridge::set_address(Origin::signed(1), vec![1,2,3,4]));
            assert_eq!(Bridge::emitter_address(), vec![1, 2, 3, 4])
        })
    }

    #[test]
    fn asset_transfer_success() {
        new_test_ext().execute_with(|| {
            let chain_id = vec![1];
            let to = vec![2];
            let token_id = vec![3];
            let metadata = vec![];

            assert_ok!(Bridge::whitelist_chain(Origin::signed(1), chain_id.clone()));
            assert_ok!(Bridge::transfer_asset(Origin::signed(1), chain_id, to, token_id, metadata));
            // TODO: Assert event
        })
    }

    #[test]
    fn asset_transfer_invalid_chain() {
        new_test_ext().execute_with(|| {
            let chain_id = vec![1];
            let to = vec![2];
            let bad_dest_id = vec![3];
            let token_id = vec![4];
            let metadata = vec![];

            assert_ok!(Bridge::whitelist_chain(Origin::signed(1), chain_id));
            assert_err!(Bridge::transfer_asset(Origin::signed(1), bad_dest_id, to, token_id, metadata), "Chain ID not whitelisted");
        })
    }
}