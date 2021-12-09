// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
// #[cfg(test)]
// pub mod mock;
// #[cfg(test)]
// mod tests;
// pub mod weights;

// pub mod migrations;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::{Currency, ExistenceRequirement, Get, ReservableCurrency, WithdrawReasons},
	weights::Weight,
};
use frame_system::Config as SystemConfig;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedAdd, CheckedSub, Saturating, StaticLookup, Zero},
	ArithmeticError, Perbill, RuntimeDebug,
};
use sp_std::prelude::*;

pub use pallet::*;
// pub use weights::WeightInfo;

pub(crate) const LOG_TARGET: &'static str = "runtime::nft";

// syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: crate::LOG_TARGET,
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

pub type DepositBalanceOf<T, I = ()> =
	<<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;
pub type ClassDetailsFor<T, I> =
	ClassDetails<<T as SystemConfig>::AccountId, DepositBalanceOf<T, I>>;
pub type TokenDetailsFor<T, I> =
	TokenDetails<<T as SystemConfig>::AccountId, DepositBalanceOf<T, I>>;

// A value placed in storage that represents the current version of the Scheduler storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum Releases {
	V0,
	V1,
}

impl Default for Releases {
	fn default() -> Self {
		Releases::V0
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct ClassDetails<AccountId, DepositBalance> {
	/// The owner of this class.
	pub owner: AccountId,
	/// The total balance deposited for this asset class.
	pub deposit: DepositBalance,
	/// Class metadata
	pub metadata: Vec<u8>,
	/// The total number of outstanding instances of this asset class.
	#[codec(compact)]
	pub instances: u32,
	/// Royalty rate
	#[codec(compact)]
	pub royalty_rate: Perbill,
}

/// Information concerning the ownership of token.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct TokenDetails<AccountId, DepositBalance> {
	/// Token metadata
	pub metadata: Vec<u8>,
	/// The total balance deposited for this asset class.
	pub deposit: DepositBalance,
	/// Token's amount.
	#[codec(compact)]
	pub quantity: u32,
	/// Royalty rate
	#[codec(compact)]
	pub royalty_rate: Perbill,
	/// Royalty beneficiary
	pub royalty_beneficiary: AccountId,
}

/// Account Token
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct AccountToken<TokenId> {
	/// account token number.
	#[codec(compact)]
	pub quantity: TokenId,
	/// account reserved token number.
	#[codec(compact)]
	pub reserved: TokenId,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub trait Store)]
	pub struct Pallet<T, I = ()>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// Identifier for the class of asset.
		type ClassId: Member + Parameter + Default + Copy + HasCompact + AtLeast32BitUnsigned;

		/// The type used to identify a unique asset within an asset class.
		type TokenId: Member + Parameter + Default + Copy + HasCompact + AtLeast32BitUnsigned;

		/// The currency mechanism, used for paying for reserves.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The basic amount of funds that must be reserved for an asset class.
		#[pallet::constant]
		type ClassDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The basic amount of funds that must be reserved for an asset instance.
		#[pallet::constant]
		type TokenDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The amount of balance that must be deposited per byte of metadata.
		#[pallet::constant]
		type MetaDataByteDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The maximum of royalty rate
		#[pallet::constant]
		type RoyaltyRateLimit: Get<Perbill>;

		/// The new class id must in (MaxClassId, MaxClassId + T::ClassIdIncLimit]
		#[pallet::constant]
		type ClassIdIncLimit: Get<Self::ClassId>;

		// /// Weight information for extrinsics in this pallet.
		// type WeightInfo: WeightInfo;
	}

	/// Store class info.
	#[pallet::storage]
	pub type Classes<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::ClassId,
		ClassDetails<T::AccountId, DepositBalanceOf<T, I>>,
	>;

	/// Store token info.
	#[pallet::storage]
	pub type Tokens<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::TokenId,
		TokenDetails<T::AccountId, DepositBalanceOf<T, I>>,
		OptionQuery,
	>;

	/// Token existence check by owner and class ID.
	#[pallet::storage]
	pub type TokensByOwner<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		(T::ClassId, T::TokenId),
		AccountToken<T::TokenId>,
	>;

	/// An index to query owners by token
    #[pallet::storage]
	pub type OwnersByToken<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Twox64Concat, (T::ClassId, T::TokenId), Twox64Concat, T::AccountId, ()>;

	/// Maximum class id in this pallet
	#[pallet::storage]
	pub type MaxClassId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::ClassId, ValueQuery>;

	/// Storage version of the pallet.
	///
	/// New networks start with last version.
	#[pallet::storage]
	pub type StorageVersion<T: Config<I>, I: 'static = ()> = StorageValue<_, Releases, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// An asset class was created. \[ class, creator \]
		Created(T::ClassId, T::AccountId),
		/// An asset `instace` was issued. \[ class, instance, owner \]
		Issued(T::ClassId, T::TokenId, T::AccountId),
		/// An asset `instace` was transferred. \[ class, instance, from, to \]
		Transferred(T::ClassId, T::TokenId, T::AccountId, T::AccountId),
		/// An asset `instance` was destroyed. \[ class, instance, owner \]
		Burned(T::ClassId, T::TokenId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// The given asset ID is nof found.
		NotFound,
		/// Unknown error
		Unknown,
		/// The asset class Id or instance ID has already been used for an asset.
		AlreadyExists,
		/// The owner of class turned out to be different to what was expected.
		WrongClassOwner,
		/// The owner turned out to be different to what was expected.
		WrongOwner,
		/// Royalty rate great than RoyaltyRateLimit
		RoyaltyRateTooHigh,
		/// The class id is not in (MaxClassId, MaxClassId + T::ClassIdIncLimit]
		ClassIdTooLarge,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig;

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig {
		fn build(&self) {
			StorageVersion::<T, I>::put(Releases::V1);
		}
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		// fn on_runtime_upgrade() -> Weight {
		// 	if StorageVersion::<T, I>::get() == Releases::V0 {
		// 		migrations::v1::migrate::<T, I>()
		// 	} else {
		// 		T::DbWeight::get().reads(1)
		// 	}
		// }

		// #[cfg(feature = "try-runtime")]
		// fn pre_upgrade() -> Result<(), &'static str> {
		// 	if StorageVersion::<T, I>::get() == Releases::V0 {
		// 		migrations::v1::pre_migrate::<T, I>()
		// 	} else {
		// 		Ok(())
		// 	}
		// }

		// #[cfg(feature = "try-runtime")]
		// fn post_upgrade() -> Result<(), &'static str> {
		// 	Ok(())
		// }
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
        /// Create NFT(non fungible token) class
		#[pallet::weight(100_000)]
		pub fn create_class(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
            metadata: Vec<u8>,
			#[pallet::compact] royalty_rate: Perbill,
		) -> DispatchResult {
            todo!()
		}

		/// Update token royalty.
		#[pallet::weight(100_000)]
		pub fn update_token_royalty(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			charge_royalty: Option<Perbill>,
		) -> DispatchResult {
            todo!()
		}

		/// Update token royalty beneficiary.
		#[pallet::weight(100_000)]
		pub fn update_token_royalty_beneficiary(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			to: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
            todo!()
        }

		/// Mint NFT token
		///
		/// - `to`: the token owner's account
		/// - `class_id`: token belong to the class id
		/// - `metadata`: external metadata
		/// - `quantity`: token quantity
		#[pallet::weight(100_000)]
		pub fn mint(
			origin: OriginFor<T>,
			to: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] class_id: T::ClassId,
			metadata: Vec<u8>,
			#[pallet::compact] quantity: u32,
			charge_royalty: Option<Perbill>,
		) -> DispatchResult {
            todo!()
		}

		/// Burn NFT token
		///
		/// - `class_id`: class id
		/// - `token_id`: token id
		/// - `quantity`: quantity
		#[pallet::weight(100_000)]
		pub fn burn(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			#[pallet::compact] quantity: u32,
		) -> DispatchResult {
            todo!()
		}

		/// Transfer NFT tokens to another account
		///
		/// - `to`: the token owner's account
		/// - `class_id`: class id
		/// - `token_id`: token id
		/// - `quantity`: quantity
		#[pallet::weight(100_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] instance: T::TokenId,
			#[pallet::compact] quantity: u32,
		) -> DispatchResult {
            todo!()
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn do_transfer(
		class: &T::ClassId,
		instance: &T::TokenId,
		owner: &T::AccountId,
		dest: &T::AccountId,
	) -> DispatchResult {
        todo!()
	}

	pub fn info(class: &T::ClassId, instance: &T::TokenId) -> Option<(T::AccountId, bool)> {
        todo!()
	}
	pub fn validate(class: &T::ClassId, instance: &T::TokenId, owner: &T::AccountId) -> bool {
		if let Some((token_owner, reserved)) = Self::info(class, instance) {
			&token_owner == owner && !reserved
		} else {
			false
		}
	}
	pub fn reserve(
		class: &T::ClassId,
		instance: &T::TokenId,
		owner: &T::AccountId,
	) -> DispatchResult {
        todo!()
	}
	pub fn unreserve(class: &T::ClassId, instance: &T::TokenId) -> DispatchResult {
        todo!()
	}
	pub fn swap(
		class: &T::ClassId,
		instance: &T::TokenId,
		who: &T::AccountId,
		price: DepositBalanceOf<T, I>,
		tax_ratio: Perbill,
	) -> DispatchResult {
        todo!()
	}
}
