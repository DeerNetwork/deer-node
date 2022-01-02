#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Eq, PartialEq, Encode, Decode, Default, TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct BalanceInfo<Balance> {
	#[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub amount: Balance,
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
	t: &T,
	serializer: S,
) -> Result<S::Ok, S::Error> {
	serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
	deserializer: D,
) -> Result<T, D::Error> {
	let s = String::deserialize(deserializer)?;
	s.parse::<T>().map_err(|_| serde::de::Error::custom("Parse from string failed"))
}

sp_api::decl_runtime_apis! {
	pub trait NFTApi<Balance> where
		Balance: Codec + MaybeDisplay + MaybeFromStr,
	 {
		/// Get deposit for create nft class.
		fn create_class_deposit(bytes_len: u32) -> BalanceInfo<Balance>;
		/// Get deposit for mint nft token.
		fn mint_token_deposit(bytes_len: u32) -> BalanceInfo<Balance>;
	}
}
