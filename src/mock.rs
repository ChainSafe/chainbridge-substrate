#![cfg(test)]

use super::*;
use frame_support::{
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types, weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use crate::{self as bridge, Trait};
use frame_system as system;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        bridge<T>,
    }
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        pallet_balances::Balances,
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Test {
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnReapAccount = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Trait for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = TestEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

impl Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
}

pub type System = frame_system::Module<Test>;
pub type Bridge = super::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;

// Bridge account and starting balance
pub const ENDOWED_ID: u64 = 0x1;
pub const ENDOWED_BALANCE: u64 = 100;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    t.into()
}

pub fn new_test_ext_endowed() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ENDOWED_ID, ENDOWED_BALANCE)],
    }
        .assimilate_storage(&mut t)
        .unwrap();

    GenesisConfig::<Test> {
        endowed: ENDOWED_ID,
    }
        .assimilate_storage(&mut t)
        .unwrap();

    t.into()
}

fn last_event() -> TestEvent {
    system::Module::<Test>::events()
        .pop()
        .map(|e| e.event)
        .expect("Event expected")
}

pub fn expect_event<E: Into<TestEvent>>(e: E) {
    assert_eq!(last_event(), e.into());
}
