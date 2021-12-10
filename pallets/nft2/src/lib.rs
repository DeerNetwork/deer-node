// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
// pub mod weights;

// pub mod migrations;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	dispatch::{DispatchError, DispatchResult},
	ensure,
	traits::{Currency, ExistenceRequirement, Get, ReservableCurrency, WithdrawReasons},
	transactional,
	weights::Weight,
};
use frame_system::Config as SystemConfig;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedAdd, CheckedSub, One, Saturating, StaticLookup, Zero},
	Perbill, RuntimeDebug, SaturatedConversion,
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

pub type BalanceOf<T, I = ()> =
	<<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub type ClassDetailsOf<T, I> =
	ClassDetails<<T as SystemConfig>::AccountId, BalanceOf<T, I>, <T as Config<I>>::TokenId>;
pub type TokenDetailsOf<T, I> =
	TokenDetails<<T as SystemConfig>::AccountId, BalanceOf<T, I>, <T as Config<I>>::TokenId>;

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
pub struct ClassDetails<AccountId, Balance, TokenId> {
	/// The owner of this class.
	pub owner: AccountId,
	/// Reserved balance for createing class
	pub deposit: Balance,
	/// Class metadata
	pub metadata: Vec<u8>,
	/// Summary of kind of tokens in class
	#[codec(compact)]
	pub total_tokens: TokenId,
	/// Summary of tokens in class
	#[codec(compact)]
	pub total_issuance: TokenId,
	/// Royalty rate
	#[codec(compact)]
	pub royalty_rate: Perbill,
}

/// Information concerning the ownership of token.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct TokenDetails<AccountId, Balance, TokenId> {
	/// Token metadata
	pub metadata: Vec<u8>,
	/// The total balance deposited for this asset class.
	pub deposit: Balance,
	/// Token's amount.
	#[codec(compact)]
	pub quantity: TokenId,
	/// Royalty rate
	#[codec(compact)]
	pub royalty_rate: Perbill,
	/// Royalty beneficiary
	pub royalty_beneficiary: AccountId,
}

/// Account Token
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct TokenAmount<TokenId> {
	/// account free token number.
	#[codec(compact)]
	pub free: TokenId,
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
		type ClassDeposit: Get<BalanceOf<Self, I>>;

		/// The basic amount of funds that must be reserved for an asset instance.
		#[pallet::constant]
		type TokenDeposit: Get<BalanceOf<Self, I>>;

		/// The amount of balance that must be deposited per byte of metadata.
		#[pallet::constant]
		type MetaDataByteDeposit: Get<BalanceOf<Self, I>>;

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
		ClassDetails<T::AccountId, BalanceOf<T, I>, T::TokenId>,
	>;

	/// Store token info.
	#[pallet::storage]
	pub type Tokens<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::TokenId,
		TokenDetails<T::AccountId, BalanceOf<T, I>, T::TokenId>,
		OptionQuery,
	>;

	/// Token existence check by owner and class ID.
	#[pallet::storage]
	#[pallet::getter(fn tokens_by_owner)]
	pub type TokensByOwner<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		(T::ClassId, T::TokenId),
		TokenAmount<T::TokenId>,
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
		/// An asset class was created. \[ class_id, creator \]
		CreatedClass(T::ClassId, T::AccountId),
		/// An asset `instace` was minted. \[ class_id, token_id, quantity, to, who \]
		MintedToken(T::ClassId, T::TokenId, T::TokenId, T::AccountId, T::AccountId),
		/// An asset `instance` was burned. \[ class_id, token_id, quantity, owner \]
		BurnedToken(T::ClassId, T::TokenId, T::TokenId, T::AccountId),
		/// An asset `instace` was transferred. \[ class_id, token_id, quantity, from, to \]
		TransferredToken(T::ClassId, T::TokenId, T::TokenId, T::AccountId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Class not found
		ClassNotFound,
		/// Token not found
		TokenNotFound,
		/// The operator is not the owner of the token and has no permission
		NoPermission,
		/// Class id in used
		ConflictClassId,
		/// Token id in used
		ConflictTokenId,
		/// Class id is invalid
		InvalidClassId,
		/// Royalty rate great than RoyaltyRateLimit
		RoyaltyRateTooHigh,
		/// Quantity is invalid
		InvalidQuantity,
		/// Num overflow
		NumOverflow,
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
		#[transactional]
		pub fn create_class(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			metadata: Vec<u8>,
			#[pallet::compact] royalty_rate: Perbill,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			ensure!(!Classes::<T, I>::contains_key(class_id), Error::<T, I>::ConflictClassId);
			ensure!(T::RoyaltyRateLimit::get() >= royalty_rate, Error::<T, I>::RoyaltyRateTooHigh);

			let max_class = MaxClassId::<T, I>::get();
			ensure!(
				class_id <= max_class.saturating_add(T::ClassIdIncLimit::get()),
				Error::<T, I>::InvalidClassId
			);

			let deposit =
				Self::caculate_deposit(T::ClassDeposit::get(), metadata.len().saturated_into());
			T::Currency::reserve(&owner, deposit)?;

			let class_details = ClassDetails {
				owner: owner.clone(),
				deposit,
				metadata,
				total_tokens: Zero::zero(),
				total_issuance: Zero::zero(),
				royalty_rate,
			};

			Classes::<T, I>::insert(class_id, class_details);
			if class_id > max_class {
				MaxClassId::<T, I>::put(class_id);
			}
			Self::deposit_event(Event::CreatedClass(class_id, owner));
			Ok(().into())
		}

		/// Mint NFT token
		#[pallet::weight(100_000)]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			to: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			metadata: Vec<u8>,
			#[pallet::compact] quantity: T::TokenId,
			royalty_rate: Option<Perbill>,
			royalty_beneficiary: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(quantity >= One::one(), Error::<T, I>::InvalidQuantity);
			ensure!(
				!Tokens::<T, I>::contains_key(class_id, token_id),
				Error::<T, I>::ConflictTokenId
			);

			let to = T::Lookup::lookup(to)?;

			Classes::<T, I>::try_mutate(&class_id, |maybe_class_details| -> DispatchResult {
				let class_details =
					maybe_class_details.as_mut().ok_or(Error::<T, I>::ClassNotFound)?;
				ensure!(&who == &class_details.owner, Error::<T, I>::NoPermission);

				let royalty_rate = royalty_rate.unwrap_or(class_details.royalty_rate);
				ensure!(
					T::RoyaltyRateLimit::get() >= royalty_rate,
					Error::<T, I>::RoyaltyRateTooHigh
				);

				let total_tokens = class_details
					.total_tokens
					.checked_add(&One::one())
					.ok_or(Error::<T, I>::NumOverflow)?;

				let total_issuance = class_details
					.total_issuance
					.checked_add(&quantity)
					.ok_or(Error::<T, I>::NumOverflow)?;

				class_details.total_tokens = total_tokens;
				class_details.total_issuance = total_issuance;

				let deposit =
					Self::caculate_deposit(T::TokenDeposit::get(), metadata.len().saturated_into());
				T::Currency::reserve(&class_details.owner, deposit)?;

				let token_details = TokenDetails {
					metadata,
					deposit,
					quantity,
					royalty_rate,
					royalty_beneficiary: royalty_beneficiary.unwrap_or(to.clone()),
				};
				Tokens::<T, I>::insert(&class_id, &token_id, token_details);
				TokensByOwner::<T, I>::insert(
					&to,
					(class_id, token_id),
					TokenAmount { free: quantity, reserved: Zero::zero() },
				);
				OwnersByToken::<T, I>::insert((class_id, token_id), &to, ());

				Self::deposit_event(Event::MintedToken(class_id, token_id, quantity, to, who));

				Ok(().into())
			})
		}

		/// Burn NFT token
		#[pallet::weight(100_000)]
		#[transactional]
		pub fn burn(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			#[pallet::compact] quantity: T::TokenId,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			ensure!(quantity >= One::one(), Error::<T, I>::InvalidQuantity);

			Classes::<T, I>::try_mutate(&class_id, |maybe_class_details| -> DispatchResult {
				let class_details =
					maybe_class_details.as_mut().ok_or(Error::<T, I>::ClassNotFound)?;

				let token_details = Tokens::<T, I>::try_mutate_exists(
					&class_id,
					&token_id,
					|maybe_token_details| -> Result<TokenDetailsOf<T, I>, DispatchError> {
						let token_details =
							maybe_token_details.as_mut().ok_or(Error::<T, I>::TokenNotFound)?;
						token_details.quantity = token_details
							.quantity
							.checked_sub(&quantity)
							.ok_or(Error::<T, I>::NumOverflow)?;
						let copyed_token_details = token_details.clone();
						if token_details.quantity.is_zero() {
							*maybe_token_details = None;
						}
						Ok(copyed_token_details)
					},
				)?;

				if token_details.quantity.is_zero() {
					T::Currency::unreserve(&class_details.owner, token_details.deposit);
					class_details.total_tokens = class_details
						.total_tokens
						.checked_sub(&One::one())
						.ok_or(Error::<T, I>::NumOverflow)?;
				}

				class_details.total_issuance = class_details
					.total_issuance
					.checked_sub(&quantity)
					.ok_or(Error::<T, I>::NumOverflow)?;

				TokensByOwner::<T, I>::try_mutate_exists(
					owner.clone(),
					(class_id, token_id),
					|maybe_token_amount| -> DispatchResult {
						let mut token_amount = maybe_token_amount.unwrap_or_default();
						token_amount.free = token_amount
							.free
							.checked_sub(&quantity)
							.ok_or(Error::<T, I>::NumOverflow)?;
						if token_amount.free.is_zero() && token_amount.reserved.is_zero() {
							*maybe_token_amount = None;
							OwnersByToken::<T, I>::remove((class_id, token_id), owner.clone());
						} else {
							*maybe_token_amount = Some(token_amount);
						}
						Ok(())
					},
				)?;

				Self::deposit_event(Event::BurnedToken(class_id, token_id, quantity, owner));
				Ok(().into())
			})
		}

		/// Update token royalty.
		#[pallet::weight(100_000)]
		pub fn update_token_royalty(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			royalty_rate: Perbill,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(T::RoyaltyRateLimit::get() >= royalty_rate, Error::<T, I>::RoyaltyRateTooHigh);
			Tokens::<T, I>::try_mutate(
				class_id,
				token_id,
				|maybe_token_details| -> DispatchResult {
					let token_details =
						maybe_token_details.as_mut().ok_or(Error::<T, I>::TokenNotFound)?;
					ensure!(who == token_details.royalty_beneficiary, Error::<T, I>::NoPermission);

					let account_token =
						Self::tokens_by_owner(&who, (class_id, token_id)).unwrap_or_default();
					ensure!(
						account_token.reserved.is_zero() &&
							account_token.free == token_details.quantity,
						Error::<T, I>::NoPermission
					);
					token_details.royalty_rate = royalty_rate;
					Ok(().into())
				},
			)
		}

		/// Update token royalty beneficiary.
		#[pallet::weight(100_000)]
		pub fn update_token_royalty_beneficiary(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			to: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Tokens::<T, I>::try_mutate(
				class_id,
				token_id,
				|maybe_token_details| -> DispatchResult {
					let token_details =
						maybe_token_details.as_mut().ok_or(Error::<T, I>::TokenNotFound)?;
					ensure!(who == token_details.royalty_beneficiary, Error::<T, I>::NoPermission);
					let to = T::Lookup::lookup(to)?;
					token_details.royalty_beneficiary = to;
					Ok(().into())
				},
			)
		}

		/// Transfer NFT tokens to another account
		///
		/// - `to`: the token owner's account
		/// - `class_id`: class id
		/// - `token_id`: token id
		/// - `quantity`: quantity
		#[pallet::weight(100_000)]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			#[pallet::compact] quantity: T::TokenId,
			to: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let to = T::Lookup::lookup(to)?;
			ensure!(quantity >= One::one(), Error::<T, I>::InvalidQuantity);

			Self::transfer_token(class_id, token_id, quantity, &who, &to)?;
			Ok(())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn transfer_token(
		class_id: T::ClassId,
		token_id: T::TokenId,
		quantity: T::TokenId,
		from: &T::AccountId,
		to: &T::AccountId,
	) -> Result<bool, DispatchError> {
		if from == to || quantity.is_zero() {
			return Ok(false)
		}
		let token = (class_id, token_id);

		TokensByOwner::<T, I>::try_mutate_exists(
			from,
			token,
			|maybe_from_amount| -> Result<bool, DispatchError> {
				let mut from_amount = maybe_from_amount.ok_or(Error::<T, I>::TokenNotFound)?;
				from_amount.free =
					from_amount.free.checked_sub(&quantity).ok_or(Error::<T, I>::NumOverflow)?;

				TokensByOwner::<T, I>::try_mutate_exists(
					to,
					token,
					|maybe_to_amount| -> DispatchResult {
						match maybe_to_amount {
							Some(to_amount) => {
								to_amount.free = to_amount
									.free
									.checked_add(&quantity)
									.ok_or(Error::<T, I>::NumOverflow)?;
							},
							None => {
								*maybe_to_amount =
									Some(TokenAmount { free: quantity, reserved: Zero::zero() });
								OwnersByToken::<T, I>::insert(token, to, ());
							},
						}
						Ok(())
					},
				)?;

				if from_amount.free.is_zero() && from_amount.reserved.is_zero() {
					*maybe_from_amount = None;
					OwnersByToken::<T, I>::remove(token, from);
				} else {
					*maybe_from_amount = Some(from_amount);
				}

				Self::deposit_event(Event::TransferredToken(
					class_id,
					token_id,
					quantity,
					from.clone(),
					to.clone(),
				));

				Ok(true)
			},
		)
	}

	pub fn can_trade(
		class_id: T::ClassId,
		token_id: T::TokenId,
		quantity: T::TokenId,
		owner: &T::AccountId,
	) -> bool {
		let token_amount = Self::tokens_by_owner(owner, (class_id, token_id)).unwrap_or_default();
		return token_amount.free >= quantity
	}

	pub fn reserve(
		class_id: T::ClassId,
		token_id: T::TokenId,
		quantity: T::TokenId,
		owner: &T::AccountId,
	) -> DispatchResult {
		TokensByOwner::<T, I>::try_mutate_exists(
			owner,
			(class_id, token_id),
			|maybe_amount| -> DispatchResult {
				let mut amount = maybe_amount.unwrap_or_default();
				amount.free =
					amount.free.checked_sub(&quantity).ok_or(Error::<T, I>::NumOverflow)?;
				amount.reserved =
					amount.reserved.checked_add(&quantity).ok_or(Error::<T, I>::NumOverflow)?;
				*maybe_amount = Some(amount);
				Ok(())
			},
		)
	}

	pub fn unreserve(
		class_id: T::ClassId,
		token_id: T::TokenId,
		quantity: T::TokenId,
		owner: &T::AccountId,
	) -> DispatchResult {
		TokensByOwner::<T, I>::try_mutate_exists(
			owner,
			(class_id, token_id),
			|maybe_amount| -> DispatchResult {
				let mut amount = maybe_amount.unwrap_or_default();
				amount.reserved =
					amount.reserved.checked_sub(&quantity).ok_or(Error::<T, I>::NumOverflow)?;
				amount.free =
					amount.free.checked_add(&quantity).ok_or(Error::<T, I>::NumOverflow)?;
				*maybe_amount = Some(amount);
				Ok(())
			},
		)
	}

	pub fn swap(
		class_id: T::ClassId,
		token_id: T::TokenId,
		quantity: T::TokenId,
		from: &T::AccountId,
		to: &T::AccountId,
		price: BalanceOf<T, I>,
		tax_ratio: Perbill,
	) -> DispatchResult {
		let token = Tokens::<T, I>::get(class_id, token_id).ok_or(Error::<T, I>::TokenNotFound)?;
		Self::unreserve(class_id, token_id, quantity, from)?;
		Self::transfer_token(class_id, token_id, quantity, from, to)?;
		let mut royalty_fee = token.royalty_rate * price;
		if royalty_fee < T::Currency::minimum_balance() &&
			T::Currency::free_balance(&token.royalty_beneficiary).is_zero()
		{
			royalty_fee = Zero::zero();
		}
		if !royalty_fee.is_zero() {
			T::Currency::transfer(
				to,
				&token.royalty_beneficiary,
				royalty_fee,
				ExistenceRequirement::KeepAlive,
			)?;
		}
		let tax_fee = tax_ratio * price;
		if !tax_fee.is_zero() {
			T::Currency::withdraw(
				to,
				tax_fee,
				WithdrawReasons::TRANSFER,
				ExistenceRequirement::KeepAlive,
			)?;
		}
		let order_fee = price.saturating_sub(royalty_fee).saturating_sub(tax_fee);
		T::Currency::transfer(to, from, order_fee, ExistenceRequirement::KeepAlive)?;
		Ok(())
	}

	fn caculate_deposit(base: BalanceOf<T, I>, metadata_len: u32) -> BalanceOf<T, I> {
		base.saturating_add(T::MetaDataByteDeposit::get().saturating_mul(metadata_len.into()))
	}
}
