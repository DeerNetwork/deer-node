use super::*;
use crate as pallet_storage;

use frame_support::{
	construct_runtime, parameter_types,
	traits::{tokens::imbalance::Imbalance, GenesisBuild, Hooks},
	weights::constants::RocksDbWeight,
	PalletId,
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchResult};

use sp_std::collections::btree_map::BTreeMap;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

use std::cell::RefCell;

pub type AccountId = u64;
pub type AccountIndex = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

pub const MAX_FILE_SIZE: u64 = 64 * 1024 * 1024; // 64M
pub const MAX_POWER: u64 = 1024 * 1024 * 1024; // 1G
pub const FILE_BASE_PRICE: Balance = 1000;

thread_local! {
	pub static STASH_BALANCE: RefCell<Balance> = RefCell::new(default_stash_balance());
	pub static FILE_BYTE_PRICE: RefCell<Balance> = RefCell::new(default_file_byte_price());
	pub static TREASURY_UNBALANCED: RefCell<u128> = RefCell::new(0);
}

pub struct TreasuryMock;

impl OnUnbalanced<NegativeImbalanceOf<Test>> for TreasuryMock {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<Test>) {
		TREASURY_UNBALANCED.with(|v| {
			*v.borrow_mut() += amount.peek();
		});
		drop(amount);
	}
}

pub struct StashBalance;
impl Get<Balance> for StashBalance {
	fn get() -> Balance {
		STASH_BALANCE.with(|v| *v.borrow())
	}
}

pub struct FileBytePrice;
impl Get<Balance> for FileBytePrice {
	fn get() -> Balance {
		FILE_BYTE_PRICE.with(|v| *v.borrow())
	}
}

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		FileStorage: pallet_storage::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = RocksDbWeight;
	type Origin = Origin;
	type Index = AccountIndex;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

parameter_types! {
	pub const MaxReserves: u32 = 50;
	pub static ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
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
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const StoragePalletId: PalletId = PalletId(*b"filestor");
	pub const SlashBalance: Balance = 100;
	pub const RoundDuration: BlockNumber = 10;
	pub const FileOrderRounds: u32 = 3;
	pub const MaxFileReplicas: u32 = 5;
	pub const EffectiveFileReplicas: u32 = 4;
	pub const MaxFileSize: u64 = MAX_FILE_SIZE;
	pub const MaxPower: u64 = MAX_POWER;
	pub const MaxReportFiles: u32 = 10;
	pub const FileBaseFee: Balance = FILE_BASE_PRICE;
	pub const StoreRewardRatio: Perbill = Perbill::from_percent(50);
	pub static MineFactor: Perbill = Perbill::from_percent(0);
	pub const MaxMine: Balance = 4 * 1048576;
}

impl Config for Test {
	type Event = Event;
	type Currency = Balances;
	type PalletId = StoragePalletId;
	type Treasury = TreasuryMock;
	type UnixTime = Timestamp;
	type SlashBalance = SlashBalance;
	type RoundDuration = RoundDuration;
	type FileOrderRounds = FileOrderRounds;
	type MaxFileReplicas = MaxFileReplicas;
	type EffectiveFileReplicas = EffectiveFileReplicas;
	type MaxFileSize = MaxFileSize;
	type MaxPower = MaxPower;
	type MaxReportFiles = MaxReportFiles;
	type FileBaseFee = FileBaseFee;
	type FileBytePrice = FileBytePrice;
	type StoreRewardRatio = StoreRewardRatio;
	type StashBalance = StashBalance;
	type MineFactor = MineFactor;
	type MaxMine = MaxMine;
	type WeightInfo = ();
}

pub struct ExtBuilder {
	enclaves: Vec<(EnclaveId, BlockNumber)>,
	stashs: Vec<(AccountId, AccountId)>,
	registers: Vec<(AccountId, RegisterData)>,
	files: Vec<(FileId, u64, Balance)>,
	reports: Vec<(AccountId, RegisterData, ReportData)>,
	now: u64,
	mine_factor: Perbill,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			enclaves: vec![(MACHINES[0].get_enclave(), 1000), (MACHINES[1].get_enclave(), 1000)],
			stashs: vec![],
			registers: vec![],
			files: vec![],
			reports: vec![],
			mine_factor: Perbill::from_percent(0),
			now: 1627833600000,
		}
	}
}

impl ExtBuilder {
	pub fn enclaves(mut self, enclaves: Vec<(EnclaveId, BlockNumber)>) -> Self {
		self.enclaves = enclaves;
		self
	}

	pub fn stash(mut self, stasher: AccountId, controller: AccountId) -> Self {
		self.stashs.push((stasher, controller));
		self
	}

	pub fn now(mut self, now: u64) -> Self {
		self.now = now;
		self
	}

	pub fn files(mut self, files: Vec<(FileId, u64, Balance)>) -> Self {
		self.files = files;
		self
	}

	pub fn reports(mut self, reports: Vec<(AccountId, RegisterData, ReportData)>) -> Self {
		self.reports = reports;
		self
	}

	pub fn register(mut self, controller: AccountId, info: RegisterData) -> Self {
		self.registers.push((controller, info));
		self
	}

	pub fn mine_factor(mut self, factor: Perbill) -> Self {
		self.mine_factor = factor;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		pallet_storage::GenesisConfig::<Test> { enclaves: self.enclaves.clone() }
			.assimilate_storage(&mut t)
			.unwrap();

		pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 1_000_000_000),
				(11, 1_000_000_000),
				(1000, 1_000_000),
				(1001, 1_000_000),
				(9999, 1_000_000_000), // only used in this file
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		MINE_FACTOR.with(|v| *v.borrow_mut() = self.mine_factor);

		let mut ext = sp_io::TestExternalities::new(t);
		let ExtBuilder { registers, stashs, files, now, reports, .. } = self;
		ext.execute_with(|| {
			System::set_block_number(1);
			Timestamp::set_timestamp(now);
			for (stasher, controller) in stashs {
				FileStorage::stash(Origin::signed(stasher), controller).unwrap();
			}
			let mut file_sizes = BTreeMap::new();
			for (cid, file_size, fee) in files {
				file_sizes.insert(cid.clone(), file_size);
				FileStorage::store(Origin::signed(9999), cid.clone(), file_size, fee).unwrap();
			}
			for (controller, register_data) in registers {
				register_data.call(controller).unwrap();
			}
			for (node, register_data, report_data) in reports {
				let machine_id = register_data.machine_id.clone();
				if Registers::<Test>::contains_key(&machine_id) {
					Registers::<Test>::remove(&machine_id);
				}
				FileStorage::stash(Origin::signed(9999), node).unwrap();
				register_data.call(node).unwrap();
				report_data.call(node).unwrap();
			}
		});
		ext
	}
}

pub fn run_to_block(n: BlockNumber) {
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		<FileStorage as Hooks<u64>>::on_initialize(b);
	}
}

pub const fn default_stash_balance() -> Balance {
	10_000
}

pub const fn default_file_byte_price() -> Balance {
	100
}

pub fn balance_of_storage_pot() -> Balance {
	Balances::free_balance(&FileStorage::account_id())
}

pub fn change_stash_balance(v: Balance) {
	STASH_BALANCE.with(|f| *f.borrow_mut() = v);
}

pub fn change_file_byte_price(v: Balance) {
	FILE_BYTE_PRICE.with(|f| *f.borrow_mut() = v);
}

pub fn mock_file_id(suffix: char) -> FileId {
	str2bytes(&format!("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeee{}", suffix))
}

pub fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	assert_eq!(
		frame_system::Pallet::<T>::events()
			.pop()
			.map(|e| e.event)
			.expect("Event expected"),
		generic_event.into()
	);
}

#[derive(Debug, Clone, Default)]
pub struct MachineInfo<'a> {
	pub machine_id: &'a str,
	pub ias_cert: &'a str,
	pub ias_sig: &'a str,
	pub ias_body: &'a str,
	pub sig: &'a str,
	pub priv_key: &'a str,
	pub enclave: &'a str,
	pub pub_key: &'a str,
}

pub const MACHINES: &'static [MachineInfo<'static>] = &[
    MachineInfo {
        machine_id: "2663554671a5f2c3050e1cec37f31e55",
        ias_cert: "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==",
        ias_sig: "OcghuZnUiFmEs85hC0Ri2uJfyWR6lhhuCKY/U3UJTRee8GiENQCNj9dAQEYuUbUG4qEhdJeW4sM3RhV1MuOgYjut6UYXnhGXLDVg48ba+L+lDRQng+E26JYnQ0MOv0mMMJCNX1l3mHTUHM8e0C/kIWQJ+esuhR6G4WuHp7xyReZfJGbuKAkc6tC+q7e9XU9HvbSRaowjIfFMrXgJUZh5VG3Cj+6rDi807rL9oAxFTweivHiz6Tcvp3aZ7pH2QpDBL9OD68gwYfDxGvBi6+S1chqI7P6pFfWHcT+CISbOo2M6p9HpSVLf/07/9xxCrDU2/M5hDxSlVbXqKQKW2Mxt8A==",
        ias_body: "{\"id\":\"327849746623058382595462695863525135492\",\"timestamp\":\"2021-07-21T07:23:39.696594\",\"version\":4,\"epidPseudonym\":\"ybSBDhwKvtRIx76tLCjLNVH+zI6JLGEEuu/c0mcQwk0OGYFRSsJfLApOkp+B/GFAzhTIIEXmYmAOSGDdbc2mFu/wx1HiK1+mFI+isaCe6ZN7IeLOrfbnVfeR6E7OhvFtc9e1xwyviVa6a9+bCVhQV1THJq7lW7HbaOxW9ZQu6g0=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00477\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00233\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000111102040180070000000000000000000C00000C000000020000000000000B2FD11FE6C355B3AB0F453E92C88F565CB58ACDCA00D3E13716CE6BDB92A372DA54784987293BE9EF77C00D94F090A9193BD6147A3C994E3086D14C57C089F35D39\",\"isvEnclaveQuoteBody\":\"AgABAC8LAAAMAAsAAAAAAAbkva5mzdO2S8iey0QRTKEAAAAAAAAAAAAAAAAAAAAABRICBf+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAPmJXfzjBbEIHCQkIXgTZKSee1RznLfSzwv1eOTzk7+jAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH9m21/giIxl3atpQAIEkv0v5hVBPxPY2RMcR4xoxsgN+kc2W/n++sKQA8+PFpoHZis8WQdRHpnkOc3mnzlv+C\"}",
        sig: "90639853f8e815ede625c0b786c8453230790193aa5b29f5dca76e48845344503c8373a5cd9536d02504e0d74dfaef791af7f65e081a7be827f6d5e492424ca4",
        priv_key: "e394cf1de366242a772f44904ba475f5317ce8baedac5485ccd812db2ccf28ab",
        enclave: "f9895dfce305b1081c242421781364a49e7b54739cb7d2cf0bf578e4f393bfa3",
        pub_key: "87f66db5fe0888c65ddab6940020492fd2fe615413f13d8d9131c478c68c6c80dfa47365bf9fefac29003cf8f169a07662b3c5907511e99e439cde69f396ff82",
    },
    MachineInfo {
        machine_id: "2663554671a5f2c3050e1cec37f31e55",
        ias_cert:"MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==",
        ias_sig:"hnWwQzypPCPQavil8v7tCE7WdJq1Skyj5/kMDsaXPUMp78uCgRVcDOtS+HI7/MGZ1aHweeXtub8cbmu2TIfxIf/HMoz13Ec3KrZEQwnV6gv3H+iwWGmbJLbvf3mFCUs7LoR2NDZe3rS2jjc5MR8z1AF2ibiUvpGktmaBdIfv6G5gb5fi0uTwIZg6j1SM+uvjl63ejUJONzAGBg09VpGIe6R9nkoo2Sj3gAKGQJWSsyAdmrtAdTijkONWFOk3Cau4wFbAOcATl2snnVop7gD5eHA2GeS4LaTo5m6nWC7xXoTiXwefKuL4nFZcxYEDJI1Aco8lrkwgKyxtkCgBz8iJuA==",
        ias_body:"{\"id\":\"37636908292053551191961084853934181455\",\"timestamp\":\"2021-07-21T08:06:44.436360\",\"version\":4,\"epidPseudonym\":\"ybSBDhwKvtRIx76tLCjLNVH+zI6JLGEEuu/c0mcQwk0OGYFRSsJfLApOkp+B/GFAzhTIIEXmYmAOSGDdbc2mFu/wx1HiK1+mFI+isaCe6ZN7IeLOrfbnVfeR6E7OhvFtc9e1xwyviVa6a9+bCVhQV1THJq7lW7HbaOxW9ZQu6g0=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00477\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00233\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000111102040180070000000000000000000C00000C000000020000000000000B2F485621743EA97D22ED8DFD3E8A970C8BECADD71E9D6E82601B4BE49AB9527A686C50EE466F5D0A9236B96E569602B7461A79B4428736834623B250F462973ACB\",\"isvEnclaveQuoteBody\":\"AgABAC8LAAAMAAsAAAAAAAbkva5mzdO2S8iey0QRTKEAAAAAAAAAAAAAAAAAAAAABRICBf+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAADjQGFxbqFLZdojQET4jE78FHcmXAHsueqQRl2v0Mak5AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABBS8SRUCg3MgDkrbPWpDvlIbfWmRJAQ8BqoPwmh7qhZ1u0e66jKHyE01IjR67NkRfLqZW2hkQfVOAilr5O/PBB\"}",
        sig:"8b6131910cf18e2733d9812aa5692d3057a8fee5ce203e5242008be76e600f02a9b2b1b35d1567fb47352017b6343589207f742c26cda942e7e28aeede6fb1ea",
        priv_key: "9496aeba1604c00d5f003307e32ac888c644694eb122688bb3af186b1559f0b3",
        enclave:"38d0185c5ba852d97688d0113e2313bf051dc997007b2e7aa411976bf431a939",
        pub_key:"414bc4915028373200e4adb3d6a43be521b7d699124043c06aa0fc2687baa1675bb47baea3287c84d3522347aecd9117cba995b686441f54e02296be4efcf041",
    },
    MachineInfo {
        machine_id:"2663554671a5f2c3050e1cec37f31e55",
        ias_cert:"MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==",
        ias_sig:"VCSj8LQ1baU234S+G6HYoXp79dlB7kpmxPITyA94sE9nVHOX5POvWQv3IIhwSo3swr093XwxwJoHieeEhDM+/Oht65Gcpa7pjYUXSZaSGK9ttcJ4PC0zDGZfCaQXfI/H+VeZIvQyP4rUPCiSo83VZhhmk0resYpJg3JKd9NksgiNs5ldCQnd1uwjc7qLxmz9RBK5ixFhCI1HtGJ5sUnnUIfgEh/7YU4gt49Bz9s0V4GStjzJ9LVCXPtf3H3n8ShQaUxrYFJxJEtHNPa30uB5qxXplhArjxeCXn6olPcUL1ct29ZEb81UIW/k6OyNiZNKhbbmgoqTY4vVkYjIUlPg0Q==",
        ias_body:"{\"id\":\"147170265343287952121166579147491110158\",\"timestamp\":\"2021-07-22T02:51:29.674396\",\"version\":4,\"epidPseudonym\":\"ybSBDhwKvtRIx76tLCjLNVH+zI6JLGEEuu/c0mcQwk0OGYFRSsJfLApOkp+B/GFAzhTIIEXmYmAOSGDdbc2mFu/wx1HiK1+mFI+isaCe6ZN7IeLOrfbnVfeR6E7OhvFtc9e1xwyviVa6a9+bCVhQV1THJq7lW7HbaOxW9ZQu6g0=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00477\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00233\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000111102040180070000000000000000000C00000C000000020000000000000B2F6EB4074389FB4C554BAEDFED95A54DD70EE8BACADD7DA97B6E190E2D80DAA0AB8E9D81AD8F64D96772422ABE9A2B195810D3D209591E69E8BADE2576B126A864\",\"isvEnclaveQuoteBody\":\"AgABAC8LAAAMAAsAAAAAAAbkva5mzdO2S8iey0QRTKEAAAAAAAAAAAAAAAAAAAAABRICBf+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAIySM7YVctOR5dl8nAmjjtxNJCKzOAjZYR3bsft9egCYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAdGRCbkomiFyRvS+VWbHYVgYQKh72h59ptYnPQ8ETscgawzNvzYsAf28Ru8xUTyOKd6Opzg2e9f34gMeCkpGxq\"}",
        sig:"fdbee145ea0e77b25b49b05e94ceb58363199e15e2ac88e270270b8967fce5b715a34c209dc3a0a89c62695392cb8313aadbcf9d6c1d51ccefd04e582bb8b8f7",
        priv_key: "9496aeba1604c00d5f003307e32ac888c644694eb122688bb3af186b1559f0b3",
        enclave:"8c9233b61572d391e5d97c9c09a38edc4d2422b33808d9611ddbb1fb7d7a0098",
        pub_key:"1d19109b9289a217246f4be5566c761581840a87bda1e7da6d6273d0f044ec7206b0ccdbf362c01fdbc46ef31513c8e29de8ea738367bd7f7e2031e0a4a46c6a",
    },
    MachineInfo {
        machine_id:"ae93e7bae33732a4b1276436c4519ce9",
        ias_cert:"MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==",
        ias_sig:"YFsCtOPPsrto600NtrxY2dWW8dj43kubYRL/9Ml46fYZEr4MPW1we7quEgqxD7LjrA/Iu+LuqTwDoW1opCaABHBd0jVnCtctjlKbf2BRoWzYhhU2EM1QgrqDhVLVZNULCiSPG90Id6qO2coJV4W7TYZOj/0k4lJG/f43mlEoXbiJrOi6F0FvQnu3hZUr+DLfmYqIFaLPvU0iBRWX4CfW2bx7+JaItEeARz5h84ogpZwjeEHVpXVMFNsQCzHPekIaB6ZPKKTzDsNyHAp7VSlI109mJGZlac4bKnV2WnftsHl/jN/3zX+aiS6V3jSH7YWahrJYm6jJs75dsF/73GCvpQ==",
        ias_body:"{\"id\":\"57151568852533705191859061879081447542\",\"timestamp\":\"2021-07-21T08:13:40.395723\",\"version\":4,\"epidPseudonym\":\"ybSBDhwKvtRIx76tLCjLNVH+zI6JLGEEuu/c0mcQwk0OGYFRSsJfLApOkp+B/GFAzhTIIEXmYmAOSGDdbc2mFu/wx1HiK1+mFI+isaCe6ZN7IeLOrfbnVfeR6E7OhvFtc9e1xwyviVa6a9+bCVhQV1THJq7lW7HbaOxW9ZQu6g0=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00477\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00233\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000111102040180070000000000000000000C00000C000000020000000000000B2F68EEC10B45BDA728F3495C5C6910EB5480EDF0A1B66DC0A1C406FD3ADC2A1ED990D8B6FD6DA139BBC81158571E32F2F948FEF6C959D4B939E07E9B7761ED37F2\",\"isvEnclaveQuoteBody\":\"AgABAC8LAAAMAAsAAAAAAAbkva5mzdO2S8iey0QRTKEAAAAAAAAAAAAAAAAAAAAABRICBf+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAADjQGFxbqFLZdojQET4jE78FHcmXAHsueqQRl2v0Mak5AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA2zk9trubIh+XiXWsa+kGTukF2vQZFDYi9UpmmbZQGaDajV1X5U+WJ1ueiIoxpdrYhHlK3xexWZ76M9CpOBbPf\"}",
        sig:"d3ad42d07e29c5f30a767e3b3d5e6e237871e657ba394502682379497c88aaa619b45c22fc10bfeac7c80c5e0d8f40f1a5a167951f2b28b3fb9a1b87de3152e4",
        priv_key: "2980d074e1aa9441ee84c9f2f8fe43666dac319d8c016dbc6faa6781610a906d",
        enclave:"38d0185c5ba852d97688d0113e2313bf051dc997007b2e7aa411976bf431a939",
        pub_key:"36ce4f6daee6c887e5e25d6b1afa4193ba4176bd06450d88bd5299a66d94066836a35755f953e589d6e7a2228c6976b6211e52b7c5ec5667be8cf42a4e05b3df",
    }
];

impl<'a> MachineInfo<'a> {
	pub fn register_data(&self) -> RegisterData {
		RegisterData {
			machine_id: hex::decode(self.machine_id).unwrap(),
			ias_cert: str2bytes(self.ias_cert),
			ias_sig: str2bytes(self.ias_sig),
			ias_body: str2bytes(self.ias_body),
			sig: hex::decode(self.sig).unwrap(),
		}
	}
	pub fn register_info(&self) -> RegisterInfo {
		RegisterInfo {
			enclave: hex::decode(self.enclave).unwrap(),
			key: hex::decode(self.pub_key).unwrap(),
		}
	}
	pub fn get_enclave(&self) -> Vec<u8> {
		hex::decode(self.enclave).unwrap()
	}
	pub fn get_machine_id(&self) -> Vec<u8> {
		hex::decode(self.machine_id).unwrap()
	}
	pub fn register(&self, node: AccountId) -> DispatchResult {
		self.register_data().call(node)
	}
}

#[derive(Debug, Clone)]
pub struct MockData {
	pub rid: u64,
	pub prev_rid: u64,
	pub sig: Vec<u8>,
	pub add_files: Vec<(FileId, u64)>,
	pub del_files: Vec<FileId>,
	pub power: u64,
	pub settle_files: Vec<FileId>,
}

impl MockData {
	pub fn new(prev_rid: u64, rid: u64, power: u64, files: &[(char, u64)]) -> Self {
		MockData {
			rid,
			prev_rid,
			sig: vec![],
			add_files: files.iter().cloned().map(|(c, size)| (mock_file_id(c), size)).collect(),
			del_files: vec![],
			power,
			settle_files: vec![],
		}
	}
	pub fn del_files(mut self, files: &[char]) -> Self {
		self.del_files = files.iter().cloned().map(|c| mock_file_id(c)).collect();
		self
	}
	pub fn settle_files(mut self, files: &[char]) -> Self {
		self.settle_files = files.iter().cloned().map(|c| mock_file_id(c)).collect();
		self
	}
	pub fn report_data(&self, machine_index: usize) -> ReportData {
		let machine = &MACHINES[machine_index];
		let machine_id = hex::decode(machine.machine_id).unwrap();
		let priv_k = hex::decode(machine.priv_key).unwrap();
		let pub_k = hex::decode(machine.pub_key).unwrap();
		let sig = sign::p256_sign(
			&machine_id,
			&priv_k,
			&pub_k,
			self.prev_rid,
			self.rid,
			&self.add_files,
			&self.del_files,
			self.power,
		);
		ReportData {
			rid: self.rid,
			sig,
			add_files: self.add_files.clone(),
			del_files: self.del_files.clone(),
			power: self.power,
			settle_files: self.settle_files.clone(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct RegisterData {
	pub machine_id: MachineId,
	pub ias_cert: Vec<u8>,
	pub ias_sig: Vec<u8>,
	pub ias_body: Vec<u8>,
	pub sig: Vec<u8>,
}

impl RegisterData {
	pub fn call(&self, node: AccountId) -> DispatchResult {
		FileStorage::register(
			Origin::signed(node),
			self.machine_id.clone(),
			self.ias_cert.clone(),
			self.ias_sig.clone(),
			self.ias_body.clone(),
			self.sig.clone(),
		)
	}
}

#[derive(Debug, Clone)]
pub struct ReportData {
	pub rid: u64,
	pub sig: Vec<u8>,
	pub add_files: Vec<(FileId, u64)>,
	pub del_files: Vec<FileId>,
	pub power: u64,
	pub settle_files: Vec<FileId>,
}

impl ReportData {
	pub fn call(&self, node: AccountId) -> DispatchResult {
		FileStorage::report(
			Origin::signed(node),
			self.rid,
			self.power,
			self.sig.clone(),
			self.add_files.clone(),
			self.del_files.clone(),
			self.settle_files.clone(),
		)
	}
}

fn str2bytes(v: &str) -> Vec<u8> {
	v.as_bytes().to_vec()
}
