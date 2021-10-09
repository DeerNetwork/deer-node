//! # NFT Module
//!
//! A simple, secure module for dealing with non-fungible assets.
//!
//! ## Related Modules
//!
//! * [`System`](../frame_system/index.html)
//! * [`Support`](../frame_support/index.html)

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub mod migrations;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::{Currency, ExistenceRequirement, Get, ReservableCurrency, WithdrawReasons},
	weights::Weight,
	BoundedVec,
};
use frame_system::Config as SystemConfig;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedAdd, CheckedSub, Saturating, StaticLookup, Zero},
	ArithmeticError, Perbill, RuntimeDebug,
};
use sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

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
pub type InstanceDetailsFor<T, I> =
	InstanceDetails<<T as SystemConfig>::AccountId, DepositBalanceOf<T, I>>;

// A value placed in storage that represents the current version of the Scheduler storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum Releases {
	V0,
	V1,
}

impl Default for Releases {
	fn default() -> Self {
		Releases::V0
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct ClassDetails<AccountId, DepositBalance> {
	/// The owner of this class.
	pub owner: AccountId,
	/// The total balance deposited for this asset class.
	pub deposit: DepositBalance,
	/// The total number of outstanding instances of this asset class.
	#[codec(compact)]
	pub instances: u32,
	/// Royalty rate
	#[codec(compact)]
	pub royalty_rate: Perbill,
}

/// Information concerning the ownership of a single unique asset.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct InstanceDetails<AccountId, DepositBalance> {
	/// The owner of this asset.
	pub owner: AccountId,
	/// The total balance deposited for this asset class.
	pub deposit: DepositBalance,
	/// Whether the asset can be reserved or not.
	pub reserved: bool,
	/// Set transfer target
	pub ready_transfer: Option<AccountId>,
	/// Royalty rate
	#[codec(compact)]
	pub royalty_rate: Perbill,
	/// Royalty beneficiary
	pub royalty_beneficiary: AccountId,
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
		type InstanceId: Member + Parameter + Default + Copy + HasCompact + AtLeast32BitUnsigned;

		/// The currency mechanism, used for paying for reserves.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The basic amount of funds that must be reserved for an asset class.
		#[pallet::constant]
		type ClassDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The basic amount of funds that must be reserved for an asset instance.
		#[pallet::constant]
		type InstanceDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The basic amount of funds that must be reserved when adding an attribute to an asset.
		#[pallet::constant]
		type DepositBase: Get<DepositBalanceOf<Self, I>>;

		/// The additional funds that must be reserved for the number of bytes store in metadata,
		/// either "normal" metadata or attribute metadata.
		#[pallet::constant]
		type DepositPerByte: Get<DepositBalanceOf<Self, I>>;

		/// The maximum length of an attribute key.
		#[pallet::constant]
		type KeyLimit: Get<u32>;

		/// The maximum length of an attribute value.
		#[pallet::constant]
		type ValueLimit: Get<u32>;

		/// The maximum of royalty rate
		#[pallet::constant]
		type RoyaltyRateLimit: Get<Perbill>;

		/// The new class id must in (MaxClassId, MaxClassId + T::ClassIdIncLimit]
		#[pallet::constant]
		type ClassIdIncLimit: Get<Self::ClassId>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	/// Details of an asset class.
	#[pallet::storage]
	pub type Class<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::ClassId,
		ClassDetails<T::AccountId, DepositBalanceOf<T, I>>,
	>;

	/// The assets held by any given account; set out this way so that assets owned by a single
	/// account can be enumerated.
	#[pallet::storage]
	pub type Account<T: Config<I>, I: 'static = ()> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::AccountId>, // owner
			NMapKey<Blake2_128Concat, T::ClassId>,
			NMapKey<Blake2_128Concat, T::InstanceId>,
		),
		(),
		OptionQuery,
	>;

	/// The assets in existence and their ownership details.
	#[pallet::storage]
	pub type Asset<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::InstanceId,
		InstanceDetails<T::AccountId, DepositBalanceOf<T, I>>,
		OptionQuery,
	>;

	/// Metadata of an asset class.
	#[pallet::storage]
	pub type Attribute<T: Config<I>, I: 'static = ()> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::ClassId>,
			NMapKey<Blake2_128Concat, Option<T::InstanceId>>,
			NMapKey<Blake2_128Concat, BoundedVec<u8, T::KeyLimit>>,
		),
		(BoundedVec<u8, T::ValueLimit>, DepositBalanceOf<T, I>),
		OptionQuery,
	>;

	#[pallet::storage]
	pub type AssetTransfer<T: Config<I>, I: 'static = ()> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::AccountId>, // target
			NMapKey<Blake2_128Concat, T::ClassId>,
			NMapKey<Blake2_128Concat, T::InstanceId>,
		),
		(),
		OptionQuery,
	>;

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
	#[pallet::metadata(
		T::AccountId = "AccountId",
		T::ClassId = "ClassId",
		T::InstanceId = "InstanceId"
	)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// An asset class was created. \[ class, creator \]
		Created(T::ClassId, T::AccountId),
		/// An asset `instace` was issued. \[ class, instance, owner \]
		Issued(T::ClassId, T::InstanceId, T::AccountId),
		/// An asset `instace` was transferred. \[ class, instance, from, to \]
		Transferred(T::ClassId, T::InstanceId, T::AccountId, T::AccountId),
		/// An asset `instance` was destroyed. \[ class, instance, owner \]
		Burned(T::ClassId, T::InstanceId, T::AccountId),
		/// New attribute metadata has been set for an asset class or instance.
		/// \[ class, maybe_instance, key, value \]
		AttributeSet(
			T::ClassId,
			Option<T::InstanceId>,
			BoundedVec<u8, T::KeyLimit>,
			BoundedVec<u8, T::ValueLimit>,
		),
		/// Attribute metadata has been cleared for an asset class or instance.
		/// \[ class, maybe_instance, key, maybe_value \]
		AttributeCleared(T::ClassId, Option<T::InstanceId>, BoundedVec<u8, T::KeyLimit>),
		/// An asset `instace` was ready to transfer. \[ class, instance, from, to \]
		ReadyTransfer(T::ClassId, T::InstanceId, T::AccountId, T::AccountId),
		/// An asset `instace` was ready to transfer. \[ class, instance, owner \]
		CancelTransfer(T::ClassId, T::InstanceId, T::AccountId),
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
		/// The asset is ready reserved
		AlreadyReserved,
		/// The asset is not reserved
		NotReserved,
		/// The asset is not ready to transer
		NotReadyTransfer,
		/// The transfer target is not origin
		NotTranserTarget,
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
		fn on_runtime_upgrade() -> Weight {
			if StorageVersion::<T, I>::get() == Releases::V0 {
				migrations::v1::migrate::<T, I>()
			} else {
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			if StorageVersion::<T, I>::get() == Releases::V0 {
				migrations::v1::pre_migrate::<T, I>()
			} else {
				Ok(())
			}
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Issue a new class of non-fungible assets from a public origin.
		///
		/// This new asset class has no assets initially and its owner is the origin.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// `AssetDeposit` funds of sender are reserved.
		///
		/// Parameters:
		/// - `class`: The identifier of the new asset class. This must not be currently in use.
		///
		/// Emits `Created` event when successful.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] royalty_rate: Perbill,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			ensure!(!Class::<T, I>::contains_key(class), Error::<T, I>::AlreadyExists);
			ensure!(T::RoyaltyRateLimit::get() >= royalty_rate, Error::<T, I>::RoyaltyRateTooHigh);

			let max_class = MaxClassId::<T, I>::get();
			ensure!(class <= max_class + T::ClassIdIncLimit::get(), Error::<T, I>::ClassIdTooLarge);

			let deposit = T::ClassDeposit::get();
			T::Currency::reserve(&owner, deposit)?;

			Class::<T, I>::insert(
				class,
				ClassDetails { owner: owner.clone(), deposit, instances: 0, royalty_rate },
			);
			if class > max_class {
				MaxClassId::<T, I>::put(class);
			}
			Self::deposit_event(Event::Created(class, owner));
			Ok(())
		}

		/// Mint an asset instance of a particular class.
		///
		/// The origin must be Signed and the sender must be the Issuer of the asset `class`.
		///
		/// - `class`: The class of the asset to be minted.
		/// - `instance`: The instance value of the asset to be minted.
		///
		/// Emits `Issued` event when successful.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
			royalty_rate: Option<Perbill>,
			royalty_beneficiary: Option<T::AccountId>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			ensure!(!Asset::<T, I>::contains_key(class, instance), Error::<T, I>::AlreadyExists);
			if let Some(rate) = royalty_rate {
				ensure!(T::RoyaltyRateLimit::get() >= rate, Error::<T, I>::RoyaltyRateTooHigh);
			}

			Class::<T, I>::try_mutate(&class, |maybe_class_details| -> DispatchResult {
				let class_details = maybe_class_details.as_mut().ok_or(Error::<T, I>::NotFound)?;
				ensure!(class_details.owner == owner, Error::<T, I>::WrongClassOwner);

				let instances =
					class_details.instances.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				class_details.instances = instances;

				let deposit = T::InstanceDeposit::get();
				T::Currency::reserve(&owner, deposit)?;

				Account::<T, I>::insert((&owner, &class, &instance), ());
				let details = InstanceDetails {
					owner: owner.clone(),
					deposit,
					reserved: false,
					ready_transfer: None,
					royalty_rate: royalty_rate.unwrap_or(class_details.royalty_rate),
					royalty_beneficiary: royalty_beneficiary.unwrap_or(owner.clone()),
				};
				Asset::<T, I>::insert(&class, &instance, details);
				Ok(())
			})?;

			Self::deposit_event(Event::Issued(class, instance, owner));
			Ok(())
		}

		/// Destroy a single asset instance.
		///
		/// Origin must be Signed and the sender should be the Admin of the asset `class`.
		///
		/// - `class`: The class of the asset to be burned.
		/// - `instance`: The instance of the asset to be burned.
		///
		/// Emits `Burned` with the actual amount burned.
		///
		/// Weight: `O(1)`
		/// Modes: `check_owner.is_some()`.
		#[pallet::weight(T::WeightInfo::burn())]
		pub fn burn(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			Class::<T, I>::try_mutate(&class, |maybe_class_details| -> DispatchResult {
				let class_details = maybe_class_details.as_mut().ok_or(Error::<T, I>::Unknown)?;
				let details =
					Asset::<T, I>::get(&class, &instance).ok_or(Error::<T, I>::Unknown)?;
				ensure!(details.owner == owner, Error::<T, I>::WrongOwner);
				ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);
				T::Currency::unreserve(&owner, details.deposit);
				class_details.instances.saturating_dec();
				Attribute::<T, I>::remove_prefix((class, Some(instance)), None);
				Ok(())
			})?;
			Asset::<T, I>::remove(&class, &instance);
			Account::<T, I>::remove((&owner, &class, &instance));
			Self::deposit_event(Event::Burned(class, instance, owner));
			Ok(())
		}

		/// Ready to transfer an asset from the sender account to another.
		///
		/// Arguments:
		/// - `class`: The class of the asset to be transferred.
		/// - `instance`: The instance of the asset to be transferred.
		/// - `dest`: The account to receive ownership of the asset.
		///
		/// Emits `ReadyTransfer`.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::ready_transfer())]
		pub fn ready_transfer(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
			dest: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let dest = T::Lookup::lookup(dest)?;
			Self::update_asset(&class, &instance, |details| {
				ensure!(&details.owner == &owner, Error::<T, I>::WrongOwner);
				ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);
				details.ready_transfer = Some(dest.clone());
				AssetTransfer::<T, I>::insert((dest.clone(), class, instance), ());
				Self::deposit_event(Event::ReadyTransfer(class, instance, owner, dest));
				Ok(())
			})
		}

		/// Cancel transfer
		///
		/// Arguments:
		/// - `class`: The class of the asset to be transferred.
		/// - `instance`: The instance of the asset to be transferred.
		///
		/// Emits `CancelTransfer`.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::cancel_transfer())]
		pub fn cancel_transfer(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			Self::update_asset(&class, &instance, |details| {
				ensure!(&details.owner == &owner, Error::<T, I>::WrongOwner);
				ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);
				let dest =
					details.ready_transfer.as_mut().ok_or(Error::<T, I>::NotReadyTransfer)?;
				AssetTransfer::<T, I>::remove((dest.clone(), class, instance));
				details.ready_transfer = None;
				Self::deposit_event(Event::CancelTransfer(class, instance, owner));
				Ok(())
			})
		}

		/// Accept transfer
		///
		/// Arguments:
		/// - `class`: The class of the asset to be transferred.
		/// - `instance`: The instance of the asset to be transferred.
		///
		/// Emits `CancelTransfer`.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::accept_transfer())]
		pub fn accept_transfer(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			let details =
				Asset::<T, I>::try_get(&class, &instance).map_err(|_| Error::<T, I>::NotFound)?;
			let dest = details.ready_transfer.as_ref().ok_or(Error::<T, I>::NotReadyTransfer)?;
			ensure!(dest == &origin, Error::<T, I>::NotTranserTarget);
			ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);
			Self::transfer(&class, &instance, &details.owner, dest)?;
			AssetTransfer::<T, I>::remove((dest.clone(), class, instance));
			Ok(())
		}

		/// Set an attribute for an asset class or instance.
		///
		/// If the origin is Signed, then funds of signer are reserved according to the formula:
		/// `DepositBase + DepositPerByte * (key.len + value.len)` taking into
		/// account any already reserved funds.
		///
		/// - `class`: The identifier of the asset class whose instance's metadata to set.
		/// - `maybe_instance`: The identifier of the asset instance whose metadata to set.
		/// - `key`: The key of the attribute.
		/// - `value`: The value to which to set the attribute.
		///
		/// Emits `AttributeSet`.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::set_attribute())]
		pub fn set_attribute(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			maybe_instance: Option<T::InstanceId>,
			key: BoundedVec<u8, T::KeyLimit>,
			value: BoundedVec<u8, T::ValueLimit>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			if let Some(ref instance) = maybe_instance {
				let details =
					Asset::<T, I>::get(&class, instance).ok_or(Error::<T, I>::NotFound)?;
				ensure!(&details.owner == &owner, Error::<T, I>::WrongOwner);
				ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);
			} else {
				let class_details = Class::<T, I>::get(&class).ok_or(Error::<T, I>::NotFound)?;
				ensure!(&class_details.owner == &owner, Error::<T, I>::WrongClassOwner);
			}
			let attribute = Attribute::<T, I>::get((class, maybe_instance, &key));
			let old_deposit = attribute.map_or(Zero::zero(), |m| m.1);
			let deposit = T::DepositPerByte::get()
				.saturating_mul(((key.len() + value.len()) as u32).into())
				.saturating_add(T::DepositBase::get());
			if deposit > old_deposit {
				T::Currency::reserve(&owner, deposit - old_deposit)?;
			} else if deposit < old_deposit {
				T::Currency::unreserve(&owner, old_deposit - deposit);
			}
			if let Some(ref instance) = maybe_instance {
				Asset::<T, I>::mutate(&class, instance, |maybe_details| -> DispatchResult {
					let details = maybe_details.as_mut().ok_or(Error::<T, I>::Unknown)?;
					Self::update_deposit(&mut details.deposit, &deposit, &old_deposit)
				})?;
			} else {
				Class::<T, I>::mutate(&class, |maybe_class_details| -> DispatchResult {
					let details = maybe_class_details.as_mut().ok_or(Error::<T, I>::Unknown)?;
					Self::update_deposit(&mut details.deposit, &deposit, &old_deposit)
				})?;
			}
			Attribute::<T, I>::insert(
				(class, maybe_instance, key.clone()),
				(value.clone(), deposit),
			);
			Self::deposit_event(Event::AttributeSet(class, maybe_instance, key, value));
			Ok(())
		}

		/// Clear attribute for an asset class or instance.
		///
		/// Origin must be either `ForceOrigin` or Signed and the sender should be the Owner of the
		/// asset `class`.
		///
		/// - `class`: The identifier of the asset class whose instance's metadata to set.
		/// - `instance`: The identifier of the asset instance whose metadata to set.
		/// - `key`: The key of the attribute.
		///
		/// Emits `AttributeCleared`.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::clear_attribute())]
		pub fn clear_attribute(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			maybe_instance: Option<T::InstanceId>,
			key: BoundedVec<u8, T::KeyLimit>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			if let Some(ref instance) = maybe_instance {
				let details =
					Asset::<T, I>::get(&class, instance).ok_or(Error::<T, I>::NotFound)?;
				ensure!(&details.owner == &owner, Error::<T, I>::WrongOwner);
				ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);
			} else {
				let class_details = Class::<T, I>::get(&class).ok_or(Error::<T, I>::NotFound)?;
				ensure!(&class_details.owner == &owner, Error::<T, I>::WrongClassOwner);
			}
			if let Some((_, deposit)) = Attribute::<T, I>::take((class, maybe_instance, &key)) {
				if let Some(ref instance) = maybe_instance {
					Asset::<T, I>::mutate(&class, instance, |maybe_details| -> DispatchResult {
						let details = maybe_details.as_mut().ok_or(Error::<T, I>::Unknown)?;
						let new_deposit = details
							.deposit
							.checked_sub(&deposit)
							.ok_or(ArithmeticError::Overflow)?;
						details.deposit = new_deposit;
						Ok(())
					})?;
				} else {
					Class::<T, I>::mutate(&class, |maybe_class_details| -> DispatchResult {
						let details = maybe_class_details.as_mut().ok_or(Error::<T, I>::Unknown)?;
						let new_deposit = details
							.deposit
							.checked_sub(&deposit)
							.ok_or(ArithmeticError::Overflow)?;
						details.deposit = new_deposit;
						Ok(())
					})?;
				}
				T::Currency::unreserve(&owner, deposit);
				Self::deposit_event(Event::AttributeCleared(class, maybe_instance, key));
			}
			Ok(())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn transfer(
		class: &T::ClassId,
		instance: &T::InstanceId,
		owner: &T::AccountId,
		dest: &T::AccountId,
	) -> DispatchResult {
		Self::update_asset(class, instance, |details| {
			ensure!(&details.owner == owner, Error::<T, I>::WrongOwner);
			ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);

			Account::<T, I>::insert((dest, class, instance), ());
			T::Currency::reserve(dest, details.deposit)?;

			Account::<T, I>::remove((owner, class, instance));
			T::Currency::unreserve(owner, details.deposit);

			if let Some(ref ready_transfer) = details.ready_transfer {
				AssetTransfer::<T, I>::remove((ready_transfer.clone(), class, instance));
				details.ready_transfer = None;
			}
			details.owner = dest.clone();
			Self::deposit_event(Event::Transferred(
				class.clone(),
				instance.clone(),
				owner.clone(),
				dest.clone(),
			));
			Ok(())
		})
	}
	pub fn info(class: &T::ClassId, instance: &T::InstanceId) -> Option<(T::AccountId, bool)> {
		Asset::<T, I>::get(class, instance).map(|v| (v.owner, v.reserved))
	}
	pub fn validate(class: &T::ClassId, instance: &T::InstanceId, owner: &T::AccountId) -> bool {
		if let Some((token_owner, reserved)) = Self::info(class, instance) {
			&token_owner == owner && !reserved
		} else {
			false
		}
	}
	pub fn reserve(
		class: &T::ClassId,
		instance: &T::InstanceId,
		owner: &T::AccountId,
	) -> DispatchResult {
		Self::update_asset(class, instance, |details| {
			ensure!(&details.owner == owner, Error::<T, I>::WrongOwner);
			ensure!(!details.reserved, Error::<T, I>::AlreadyReserved);
			if let Some(ref ready_transfer) = details.ready_transfer {
				AssetTransfer::<T, I>::remove((ready_transfer.clone(), class, instance));
				details.ready_transfer = None;
			}
			details.reserved = true;
			Ok(())
		})
	}
	pub fn unreserve(class: &T::ClassId, instance: &T::InstanceId) -> DispatchResult {
		Self::update_asset(class, instance, |details| {
			ensure!(details.reserved, Error::<T, I>::NotReserved);
			details.reserved = false;
			Ok(())
		})
	}
	pub fn swap(
		class: &T::ClassId,
		instance: &T::InstanceId,
		who: &T::AccountId,
		price: DepositBalanceOf<T, I>,
		tax_ratio: Perbill,
	) -> DispatchResult {
		let token = Asset::<T, I>::get(class, instance).ok_or(Error::<T, I>::NotFound)?;
		let mut royalty_fee = token.royalty_rate * price;
		if royalty_fee < T::Currency::minimum_balance() &&
			T::Currency::free_balance(&token.royalty_beneficiary).is_zero()
		{
			royalty_fee = Zero::zero();
		}
		let tax_fee = tax_ratio * price;
		let order_fee = price.saturating_sub(royalty_fee).saturating_sub(tax_fee);
		if !royalty_fee.is_zero() {
			T::Currency::transfer(
				who,
				&token.royalty_beneficiary,
				royalty_fee,
				ExistenceRequirement::KeepAlive,
			)?;
		}
		if !tax_fee.is_zero() {
			T::Currency::withdraw(
				who,
				tax_fee,
				WithdrawReasons::TRANSFER,
				ExistenceRequirement::KeepAlive,
			)?;
		}
		T::Currency::transfer(who, &token.owner, order_fee, ExistenceRequirement::KeepAlive)?;
		Self::unreserve(&class, &instance)?;
		Self::transfer(&class, &instance, &token.owner, &who)?;
		Ok(())
	}
	fn update_asset(
		class: &T::ClassId,
		instance: &T::InstanceId,
		with_details: impl FnOnce(&mut InstanceDetailsFor<T, I>) -> DispatchResult,
	) -> DispatchResult {
		Asset::<T, I>::try_mutate(class, instance, |maybe_details| -> DispatchResult {
			let details = maybe_details.as_mut().ok_or(Error::<T, I>::NotFound)?;
			with_details(details)
		})
	}
	fn update_deposit(
		target: &mut DepositBalanceOf<T, I>,
		new: &DepositBalanceOf<T, I>,
		old: &DepositBalanceOf<T, I>,
	) -> DispatchResult {
		*target = target
			.checked_add(new)
			.and_then(|sum| sum.checked_sub(old))
			.ok_or(ArithmeticError::Overflow)?;
		Ok(())
	}
}
