// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test environment for Assets pallet.

use super::*;
use crate as pallet_storage;

use sp_core::H256;
use sp_runtime::{traits::{IdentityLookup}, testing::Header};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{GenesisBuild, Hooks},
	weights::constants::RocksDbWeight,
};
use hex_literal::hex;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

use std::{cell::RefCell};

pub const INIT_TIMESTAMP: u64 = 30_000;
pub const BLOCK_TIME: u64 = 1000;

pub type AccountId = u64;
pub type AccountIndex = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

thread_local! {
    static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
}

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
    fn get() -> Balance {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
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
	type BaseCallFilter = ();
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
	pub const SlashBalance: Balance = 1_000_000;
	pub const RoundDuration: BlockNumber = 10;
	pub const FileOrderRounds: u32 = 6;
	pub const MaxFileReplicas: u32 = 3;
	pub const MaxFileSize: u64 = 137_438_953_472; // 128G
	pub const FileBasePrice: Balance = 1_000;
	pub const FileBytePrice: Balance = 1;
	pub const StoreRewardRatio: Perbill = Perbill::from_percent(20);
	pub const StashBalance: Balance = 3_000;
	pub const HistoryRoundDepth: u32 = 90;
}

impl Config for Test {
	type Event = Event;
	type Currency = Balances;
	type UnixTime = Timestamp;
	type RoundPayout = ();
	type SlashBalance = SlashBalance;
	type RoundDuration = RoundDuration;
	type FileOrderRounds = FileOrderRounds;
	type MaxFileReplicas = MaxFileReplicas;
	type MaxFileSize = MaxFileSize;
	type FileBasePrice = FileBasePrice;
	type FileBytePrice = FileBytePrice;
	type StoreRewardRatio = StoreRewardRatio;
	type StashBalance = StashBalance;
	type HistoryRoundDepth = HistoryRoundDepth;
}

pub struct ExtBuilder {
    enclave: EnclaveId,
	enclave_expire_at: u64,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            enclave: hex!("0000000000000000000000000000000000000000000000000000000000000000").into(),
			enclave_expire_at: 1000,
        }
    }
}

impl ExtBuilder {
	pub fn enclave(mut self, enclave: EnclaveId) -> Self {
		self.enclave = enclave;
		self
	}

	pub fn build(self)  -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

			let fake_enclave = hex!("0000000000000000000000000000000000000000000000000000000000000000").into();
			pallet_storage::GenesisConfig::<Test> {
				enclaves: vec![(self.enclave, self.enclave_expire_at), (fake_enclave, 3000)]
			}.assimilate_storage(&mut t).unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub fn run_to_block(n: BlockNumber) {
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		<FileStorage as Hooks<u64>>::on_initialize(b);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
	}
}
