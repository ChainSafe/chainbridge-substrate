#![deny(warnings)]
#![cfg(test)]

use crate::{self as pallet_example, Config};
use frame_support::PalletId;
use frame_support::{ord_parameter_types, parameter_types, weights::Weight};
use pallet_example_erc721::WeightInfo;
use sp_core::hashing::blake2_128;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const MaxLocks: u32 = 100;
}

impl frame_system::Config for MockRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type PalletInfo = PalletInfo;
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

ord_parameter_types! {
    pub const One: u64 = 1;
}

impl pallet_balances::Config for MockRuntime {
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = MaxLocks;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

parameter_types! {
    pub const TestChainId: u8 = 5;
    pub const ProposalLifetime: u64 = 100;
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
}

impl chainbridge::Config for MockRuntime {
    type Event = Event;
    type PalletId = ChainBridgePalletId;
    type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type Proposal = Call;
    type ChainId = TestChainId;
    type ProposalLifetime = ProposalLifetime;
}

parameter_types! {
    pub HashId: chainbridge::ResourceId = chainbridge::derive_resource_id(1, &blake2_128(b"hash"));
    pub NativeTokenId: chainbridge::ResourceId = chainbridge::derive_resource_id(1, &blake2_128(b"DAV"));
    pub Erc721Id: chainbridge::ResourceId = chainbridge::derive_resource_id(1, &blake2_128(b"NFT"));
}

pub struct MockWeightInfo;
impl WeightInfo for MockWeightInfo {
    fn mint() -> Weight {
        0 as Weight
    }

    fn transfer() -> Weight {
        0 as Weight
    }

    fn burn() -> Weight {
        0 as Weight
    }
}

impl pallet_example_erc721::Config for MockRuntime {
    type Event = Event;
    type Identifier = Erc721Id;
    type WeightInfo = MockWeightInfo;
}

impl Config for MockRuntime {
    type Event = Event;
    type BridgeOrigin = chainbridge::EnsureBridge<MockRuntime>;
    type Currency = Balances;
    type HashId = HashId;
    type NativeTokenId = NativeTokenId;
    type Erc721Id = Erc721Id;
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic =
    sp_runtime::generic::UncheckedExtrinsic<u32, u64, Call, ()>;

frame_support::construct_runtime!(
    pub enum MockRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Bridge: chainbridge::{Pallet, Call, Storage, Event<T>},
        Erc721: pallet_example_erc721::{Pallet, Call, Storage, Event<T>},
        Example: pallet_example::{Pallet, Call, Event<T>}
    }
);

pub const RELAYER_A: u64 = 0x2;
pub const RELAYER_B: u64 = 0x3;
pub const RELAYER_C: u64 = 0x4;
pub const ENDOWED_BALANCE: u64 = 100_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let bridge_id = chainbridge::Pallet::<MockRuntime>::account_id();
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<MockRuntime>()
        .unwrap();
    pallet_balances::GenesisConfig::<MockRuntime> {
        balances: vec![
            (bridge_id, ENDOWED_BALANCE),
            (RELAYER_A, ENDOWED_BALANCE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn last_event() -> Event {
    frame_system::Pallet::<MockRuntime>::events()
        .pop()
        .map(|e| e.event)
        .expect("Event expected")
}

pub fn expect_event<E: Into<Event>>(e: E) {
    assert_eq!(last_event(), e.into());
}

// Asserts that the event was emitted at some point.
pub fn event_exists<E: Into<Event>>(e: E) {
    let actual: Vec<Event> = frame_system::Pallet::<MockRuntime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();
    let e: Event = e.into();
    let mut exists = false;
    for evt in actual {
        if evt == e {
            exists = true;
            break;
        }
    }
    assert!(exists);
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<Event>) {
    let mut actual: Vec<Event> = frame_system::Pallet::<MockRuntime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();

    expected.reverse();

    for evt in expected {
        let next = actual.pop().expect("event expected");
        assert_eq!(next, evt.into(), "Events don't match");
    }
}
