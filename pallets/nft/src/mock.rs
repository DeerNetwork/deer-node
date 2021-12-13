//! Test environment for Assets pallet.

use super::*;
use crate as pallet_nft;

use frame_support::{assert_ok, construct_runtime, parameter_types};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		NFT: pallet_nft::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
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
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = u64;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const ClassDeposit: u64 = 2;
	pub const TokenDeposit: u64 = 1;
	pub const MetaDataByteDeposit: u64 = 1;
	pub const RoyaltyRateLimit: Perbill = Perbill::from_percent(20);
}

impl Config for Test {
	type Event = Event;
	type ClassId = u32;
	type TokenId = u32;
	type Currency = Balances;
	type ClassDeposit = ClassDeposit;
	type TokenDeposit = TokenDeposit;
	type MetaDataByteDeposit = MetaDataByteDeposit;
	type RoyaltyRateLimit = RoyaltyRateLimit;
	type WeightInfo = ();
}

pub(crate) fn rate(v: u32) -> Perbill {
	Perbill::from_percent(v)
}

pub(crate) fn add_class(caller: u32) -> u32 {
	let permission = ClassPermission(Permission::Burnable | Permission::Transferable);
	assert_ok!(NFT::create_class(
		Origin::signed(caller.into()),
		vec![0, 0, 0],
		rate(5),
		permission
	));
	NextClassId::<Test>::get() - 1
}

pub(crate) fn add_token(caller: u32, class_id: u32) -> u32 {
	assert_ok!(NFT::mint(
		Origin::signed(caller.into()),
		caller.into(),
		class_id,
		2,
		vec![0, 0, 1],
		None,
		None
	));
	NextTokenId::<Test>::get(class_id) - 1
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
