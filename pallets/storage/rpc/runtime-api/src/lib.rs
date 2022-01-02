#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

#[cfg(feature = "std")]
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	/// The helper API to calculate deposit.
	pub trait FileStorageApi<Balance, BlockNumber> where
		Balance: Codec,
		BlockNumber: Codec,
	 {
		/// Deposit for store ipfs file
		fn store_fee(file_size: u64, time: BlockNumber) -> Balance;
	}
}
