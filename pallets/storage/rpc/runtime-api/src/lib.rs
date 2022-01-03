#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Encode, Decode, Default, TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
#[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
pub struct StoreFeeInfo<Balance> {
	#[cfg_attr(feature = "std", serde(with = "serde_balance"))]
	pub fee: Balance,
}

#[derive(Eq, PartialEq, Encode, Decode, Default, TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
#[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
pub struct NodeDepositInfo<Balance> {
	#[cfg_attr(feature = "std", serde(with = "serde_balance"))]
	pub current_deposit: Balance,
	#[cfg_attr(feature = "std", serde(with = "serde_balance"))]
	pub slash_deposit: Balance,
	#[cfg_attr(feature = "std", serde(with = "serde_balance"))]
	pub used_deposit: Balance,
}

#[cfg(feature = "std")]
mod serde_balance {
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S: Serializer, T: std::fmt::Display>(
		t: &T,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(&t.to_string())
	}

	pub fn deserialize<'de, D: Deserializer<'de>, T: std::str::FromStr>(
		deserializer: D,
	) -> Result<T, D::Error> {
		let s = String::deserialize(deserializer)?;
		s.parse::<T>().map_err(|_| serde::de::Error::custom("Parse from string failed"))
	}
}

sp_api::decl_runtime_apis! {
	pub trait FileStorageApi<AccountId, Balance, BlockNumber> where
		Balance: Codec + MaybeDisplay + MaybeFromStr,
		BlockNumber: Codec,
		AccountId: Codec,
	 {
		/// Get fee for store ipfs file.
		fn store_fee(file_size: u64, time: BlockNumber) -> StoreFeeInfo<Balance>;
		/// Get node deposit.
		fn node_deposit(controller: &AccountId) -> NodeDepositInfo<Balance>;
	}
}
