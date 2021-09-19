use super::*;
use crate as pallet_nft_order;

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
		NFTOrder: pallet_nft_order::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for Test {
	type BaseCallFilter = ();
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
	pub const InstanceDeposit: u64 = 1;
	pub const KeyLimit: u32 = 50;
	pub const ValueLimit: u32 = 50;
	pub const DepositBase: u64 = 1;
	pub const DepositPerByte: u64 = 1;
	pub const RoyaltyRateLimit: Perbill = Perbill::from_percent(20);
	pub const ClassIdIncLimit: u32 = 10;
}

impl pallet_nft::Config for Test {
	type Event = Event;
	type ClassId = u32;
	type InstanceId = u32;
	type Currency = Balances;
	type ClassDeposit = ClassDeposit;
	type InstanceDeposit = InstanceDeposit;
	type DepositBase = DepositBase;
	type DepositPerByte = DepositPerByte;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type RoyaltyRateLimit = RoyaltyRateLimit;
	type ClassIdIncLimit = ClassIdIncLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub const OrderDeposit: u64 = 10;
	pub const MaxOrders: u32 = 50;
	pub const TradeFeeTaxRatio: Perbill = Perbill::from_percent(10);
}

impl Config for Test {
	type Event = Event;
	type OrderDeposit = OrderDeposit;
	type MaxOrders = MaxOrders;
	type TradeFeeTaxRatio = TradeFeeTaxRatio;
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
