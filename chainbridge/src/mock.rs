#![deny(warnings)]
use crate as pallet_chainbridge;
use frame_support::{
    assert_ok,
    parameter_types,
    traits::{
        SortedMembers,
        StorageMapShim,
    },
    PalletId,
};
use frame_system as system;
use frame_system::EnsureSignedBy;
use pallet_chainbridge::{
    types::ChainId,
    ResourceId,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{
        BlakeTwo256,
        IdentityLookup,
    },
};

type Balance = u64;
type UncheckedExtrinsic =
    frame_system::mocking::MockUncheckedExtrinsic<MockRuntime>;
type Block = frame_system::mocking::MockBlock<MockRuntime>;

// Constants definition
pub(crate) const RELAYER_A: u64 = 0x2;
pub(crate) const RELAYER_B: u64 = 0x3;
pub(crate) const RELAYER_C: u64 = 0x4;
pub(crate) const ENDOWED_BALANCE: u64 = 100_000_000;
pub(crate) const TEST_THRESHOLD: u32 = 2;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum MockRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {

        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Bridge: pallet_chainbridge::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Config<T>, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

// Implement FRAME balances pallet configuration trait for the mock runtime
impl pallet_balances::Config for MockRuntime {
    // https://stackoverflow.com/questions/66511734/how-do-we-use-the-balances-pallet-instead-of-the-system-pallet-to-store-the-bala
    type AccountStore = StorageMapShim<
        pallet_balances::Account<MockRuntime>,
        frame_system::Provider<MockRuntime>,
        Self::AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
    type WeightInfo = ();
}

impl system::Config for MockRuntime {
    type AccountData = ();
    type AccountId = u64;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockHashCount = BlockHashCount;
    type BlockLength = ();
    type BlockNumber = u64;
    type BlockWeights = ();
    type Call = Call;
    type DbWeight = ();
    type Event = Event;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Index = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type Origin = Origin;
    type PalletInfo = PalletInfo;
    type SS58Prefix = SS58Prefix;
    type SystemWeightInfo = ();
    type Version = ();
}

// Parameterize default test user identifier (with id 1)
parameter_types! {
    pub const TestUserId: u64 = 1;
    pub const TestChainId: ChainId = 5;
    pub const ProposalLifetime: u64 = 10;
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
}

impl SortedMembers<u64> for TestUserId {
    fn sorted_members() -> Vec<u64> {
        vec![1]
    }
}

// Parameterize FRAME balances pallet
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_chainbridge::Config for MockRuntime {
    type AdminOrigin = EnsureSignedBy<TestUserId, u64>;
    type ChainId = TestChainId;
    type Event = Event;
    type PalletId = ChainBridgePalletId;
    type Proposal = Call;
    type ProposalLifetime = ProposalLifetime;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<MockRuntime>()
        .unwrap()
        .into()
}

pub fn new_test_ext_initialized(
    src_id: ChainId,
    r_id: ResourceId,
    resource: Vec<u8>,
) -> sp_io::TestExternalities {
    let mut t = new_test_ext();
    t.execute_with(|| {
        // Set and check threshold
        assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD));
        assert_eq!(Bridge::relayer_threshold(), TEST_THRESHOLD);
        // Add relayers
        assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
        assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
        assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_C));
        // Whitelist chain
        assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
        // Set and check resource ID mapped to some junk data
        assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource));
        assert_eq!(Bridge::resource_exists(r_id), true);
    });
    t
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<Event>) {
    let mut actual: Vec<Event> = system::Pallet::<MockRuntime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();
    dbg!(&actual);

    expected.reverse();

    for evt in expected {
        dbg!(&evt);
        let next = actual.pop().expect("event expected");
        assert_eq!(next, evt.into(), "Events don't match (actual,expected)");
    }
}
