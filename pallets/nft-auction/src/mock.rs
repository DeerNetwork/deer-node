use super::*;
use crate as pallet_nft_auction;

use frame_support::{construct_runtime, parameter_types};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
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
		NFTAuction: pallet_nft_auction::{Pallet, Call, Storage, Event<T>},
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
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 2;
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

impl pallet_nft::Config for Test {
	type Event = Event;
	type ClassId = u32;
	type TokenId = u32;
	type Quantity = u32;
	type Currency = Balances;
	type ClassDeposit = ClassDeposit;
	type TokenDeposit = TokenDeposit;
	type MetaDataByteDeposit = MetaDataByteDeposit;
	type RoyaltyRateLimit = RoyaltyRateLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub const AuctionDeposit: u64 = 10;
	pub const AuctionFeeTaxRatio: Perbill = Perbill::from_percent(10);
	pub const DelayOfAuction: u64 = 60;
}

impl Config for Test {
	type Event = Event;
	type AuctionId = u32;
	type AuctionDeposit = AuctionDeposit;
	type AuctionFeeTaxRatio = AuctionFeeTaxRatio;
	type DelayOfAuction = DelayOfAuction;
	type WeightInfo = ();
}

pub(crate) fn rate(v: u32) -> Perbill {
	Perbill::from_percent(v)
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
	}
}
