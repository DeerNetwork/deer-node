#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

#[cfg(feature = "std")]
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	/// The helper API to calculate deposit.
	pub trait NFTApi<Balance> where
		Balance: Codec
	 {
		/// create_class_deposit.
		fn create_class_deposit(bytes_len: u32) -> Balance;
		/// mint_token_deposit.
		fn mint_token_deposit(bytes_len: u32) -> Balance;
	}
}
