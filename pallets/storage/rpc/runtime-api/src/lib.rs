#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

#[cfg(feature = "std")]
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	pub trait FileStorageApi<Balance, BlockNumber> where
		Balance: Codec,
		BlockNumber: Codec,
	 {
		/// Get fee for store ipfs file.
		fn store_fee(file_size: u64, time: BlockNumber) -> Balance;
	}
}
