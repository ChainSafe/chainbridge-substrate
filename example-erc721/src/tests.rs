#![cfg(test)]

use super::*;

use frame_support::{
    assert_noop, assert_ok, ord_parameter_types, parameter_types, weights::Weight,
};
use frame_system::{self as system};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, IdentityLookup},
    BuildStorage, Perbill,
};

use crate::{self as erc721, Trait};
pub use pallet_balances as balances;

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
    type AccountData = balances::AccountData<u64>;
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
        Erc721: erc721::{Module, Call, Storage, Event<T>},
    }
);

pub const USER_A: u64 = 0x1;
pub const USER_B: u64 = 0x2;
pub const USER_C: u64 = 0x3;
pub const ENDOWED_BALANCE: u64 = 100_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
    GenesisConfig {
        balances: Some(balances::GenesisConfig {
            balances: vec![(USER_A, ENDOWED_BALANCE)],
        }),
    }
    .build_storage()
    .unwrap()
    .into()
}

#[test]
fn mint_tokens() {
    new_test_ext().execute_with(|| {
        let id_a: U256 = 1.into();
        let id_b: U256 = 2.into();
        let metadata_a: Vec<u8> = vec![1, 2, 3];
        let metadata_b: Vec<u8> = vec![4, 5, 6];

        assert_ok!(Erc721::mint(Origin::ROOT, USER_A, id_a, metadata_a.clone()));
        assert_eq!(
            Erc721::tokens(id_a).unwrap(),
            Erc721Token {
                id: id_a,
                metadata: metadata_a.clone()
            }
        );
        assert_eq!(Erc721::token_count(), 1.into());
        assert_noop!(
            Erc721::mint(Origin::ROOT, USER_A, id_a, metadata_a.clone()),
            Error::<Test>::TokenAlreadyExists
        );

        assert_ok!(Erc721::mint(Origin::ROOT, USER_A, id_b, metadata_b.clone()));
        assert_eq!(
            Erc721::tokens(id_b).unwrap(),
            Erc721Token {
                id: id_b,
                metadata: metadata_b.clone()
            }
        );
        assert_eq!(Erc721::token_count(), 2.into());
        assert_noop!(
            Erc721::mint(Origin::ROOT, USER_A, id_b, metadata_b.clone()),
            Error::<Test>::TokenAlreadyExists
        );
    })
}

fn transfer_tokens() {
    new_test_ext().execute_with(|| {
        let id_a: U256 = 1.into();
        let id_b: U256 = 2.into();
        let id_c: U256 = 3.into();
        let metadata_a: Vec<u8> = vec![1, 2, 3];
        let metadata_b: Vec<u8> = vec![4, 5, 6];
        let metadata_c: Vec<u8> = vec![7, 8, 9];

        assert_ok!(Erc721::mint(Origin::ROOT, USER_A, id_a, metadata_a.clone()));
        assert_ok!(Erc721::mint(Origin::ROOT, USER_A, id_b, metadata_b.clone()));

        assert_ok!(Erc721::transfer(Origin::signed(USER_A), USER_B, id_a));
        assert_eq!(Erc721::owner_of(id_a).unwrap(), USER_B);

        assert_ok!(Erc721::transfer(Origin::signed(USER_A), USER_B, id_b));
        assert_eq!(Erc721::owner_of(id_b).unwrap(), USER_C);

        assert_ok!(Erc721::transfer(Origin::signed(USER_B), USER_A, id_a));
        assert_eq!(Erc721::owner_of(id_a).unwrap(), USER_A);

        assert_ok!(Erc721::transfer(Origin::signed(USER_C), USER_A, id_b));
        assert_eq!(Erc721::owner_of(id_b).unwrap(), USER_A);
    })
}
