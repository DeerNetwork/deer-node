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

//! Autogenerated weights for pallet_nft_order
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-12-13, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/deer-node
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-nft-order
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/nft-order/src/weights.rs
// --template=./scripts/frame-weight-template.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_nft_order.
pub trait WeightInfo {
	fn sell() -> Weight;
	fn deal() -> Weight;
	fn remove() -> Weight;
}

/// Weights for pallet_nft_order using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: NFT TokensByOwner (r:1 w:1)
	// Storage: NFT Classes (r:1 w:0)
	// Storage: NFTOrder NextOrderId (r:1 w:1)
	// Storage: NFTOrder Orders (r:0 w:1)
	fn sell() -> Weight {
		(56_860_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: NFTOrder Orders (r:1 w:1)
	// Storage: NFT Tokens (r:1 w:0)
	// Storage: NFT TokensByOwner (r:2 w:2)
	// Storage: NFT OwnersByToken (r:0 w:2)
	fn deal() -> Weight {
		(120_785_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: NFTOrder Orders (r:1 w:1)
	// Storage: NFT TokensByOwner (r:1 w:1)
	fn remove() -> Weight {
		(47_244_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: NFT TokensByOwner (r:1 w:1)
	// Storage: NFT Classes (r:1 w:0)
	// Storage: NFTOrder NextOrderId (r:1 w:1)
	// Storage: NFTOrder Orders (r:0 w:1)
	fn sell() -> Weight {
		(56_860_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
	// Storage: NFTOrder Orders (r:1 w:1)
	// Storage: NFT Tokens (r:1 w:0)
	// Storage: NFT TokensByOwner (r:2 w:2)
	// Storage: NFT OwnersByToken (r:0 w:2)
	fn deal() -> Weight {
		(120_785_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: NFTOrder Orders (r:1 w:1)
	// Storage: NFT TokensByOwner (r:1 w:1)
	fn remove() -> Weight {
		(47_244_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
}
