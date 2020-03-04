#![cfg(test)]

use super::*;

use frame_support::{
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, ord_parameter_types, parameter_types,
    weights::Weight,
};
use frame_system::EnsureSignedBy;
use frame_system::{self as system};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, Block as BlockT, IdentityLookup},
    BuildStorage, Perbill,
};

use crate::{self as bridge, Trait};
use pallet_balances as balances;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
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
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

ord_parameter_types! {
    pub const One: u64 = 1;
}

impl pallet_balances::Trait for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

impl Trait for Test {
    type Event = Event;
    type Currency = Balances;
    // type ValidatorOrigin = EnsureSignedBy<One, u64>;
    type Proposal = Call;
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, Call, ()>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: system::{Module, Call, Event<T>},
        Balances: balances::{Module, Call, Storage, Config<T>, Event<T>},
        Bridge: bridge::{Module, Call, Event<T>, Config<T>}
    }
);

pub const ENDOWED_ID: u64 = 0x1;
pub const VALIDATOR_A: u64 = 0x2;
pub const VALIDATOR_B: u64 = 0x3;
pub const VALIDATOR_C: u64 = 0x4;
pub const USER: u64 = 0x4;
pub const ENDOWED_BALANCE: u64 = 100;

pub fn new_test_ext(threshold: u32) -> sp_io::TestExternalities {
    GenesisConfig {
        bridge: Some(bridge::GenesisConfig {
            endowed: ENDOWED_ID,
            validators: vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C],
            validator_threshold: threshold,
        }),
        balances: Some(balances::GenesisConfig {
            balances: vec![(ENDOWED_ID, ENDOWED_BALANCE)],
        }),
    }
    .build_storage()
    .unwrap()
    .into()
}
