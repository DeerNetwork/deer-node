#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

#[cfg(feature = "std")]
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	pub trait NFTApi<Balance> where
		Balance: Codec
	 {
		/// Get deposit for create nft class.
		fn create_class_deposit(bytes_len: u32) -> Balance;
		/// Get deposit for mint nft token.
		fn mint_token_deposit(bytes_len: u32) -> Balance;
	}
}
