// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_nft
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-09-15, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/deer-node
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-nft
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/nft/src/weights.rs
// --template=./scripts/frame-weight-template.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_nft.
pub trait WeightInfo {
	fn create() -> Weight;
	fn mint() -> Weight;
	fn burn() -> Weight;
	fn ready_transfer() -> Weight;
	fn cancel_transfer() -> Weight;
	fn accept_transfer() -> Weight;
	fn set_attribute() -> Weight;
	fn clear_attribute() -> Weight;
}

/// Weights for pallet_nft using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: NFT Class (r:1 w:1)
	fn create() -> Weight {
		(41_837_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Class (r:1 w:1)
	// Storage: NFT Account (r:0 w:1)
	fn mint() -> Weight {
		(56_928_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: NFT Class (r:1 w:1)
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Account (r:0 w:1)
	fn burn() -> Weight {
		(66_609_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT AssetTransfer (r:0 w:1)
	fn ready_transfer() -> Weight {
		(31_610_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT AssetTransfer (r:0 w:1)
	fn cancel_transfer() -> Weight {
		(31_004_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT AssetTransfer (r:0 w:1)
	// Storage: NFT Account (r:0 w:2)
	fn accept_transfer() -> Weight {
		(93_067_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Attribute (r:1 w:1)
	fn set_attribute() -> Weight {
		(83_877_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Attribute (r:1 w:1)
	fn clear_attribute() -> Weight {
		(68_659_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: NFT Class (r:1 w:1)
	fn create() -> Weight {
		(41_837_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Class (r:1 w:1)
	// Storage: NFT Account (r:0 w:1)
	fn mint() -> Weight {
		(56_928_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
	// Storage: NFT Class (r:1 w:1)
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Account (r:0 w:1)
	fn burn() -> Weight {
		(66_609_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT AssetTransfer (r:0 w:1)
	fn ready_transfer() -> Weight {
		(31_610_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT AssetTransfer (r:0 w:1)
	fn cancel_transfer() -> Weight {
		(31_004_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT AssetTransfer (r:0 w:1)
	// Storage: NFT Account (r:0 w:2)
	fn accept_transfer() -> Weight {
		(93_067_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Attribute (r:1 w:1)
	fn set_attribute() -> Weight {
		(83_877_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: NFT Asset (r:1 w:1)
	// Storage: NFT Attribute (r:1 w:1)
	fn clear_attribute() -> Weight {
		(68_659_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
}
