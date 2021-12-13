#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub mod migrations;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	dispatch::DispatchResult,
	traits::{Currency, ReservableCurrency},
	transactional,
	weights::Weight,
};
use scale_info::TypeInfo;
use sp_runtime::{traits::One, Perbill, RuntimeDebug};
use sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T, I = ()> = <<T as pallet_nft::Config<I>>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
pub type ClassIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::ClassId;
pub type TokenIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::TokenId;
pub type OrderDetailsOf<T, I = ()> = OrderDetails<
	<T as frame_system::Config>::AccountId,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
	TokenIdOf<T, I>,
>;

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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct OrderDetails<AccountId, Balance, BlockNumber, TokenId> {
	/// Who create the order.
	pub owner: AccountId,
	/// Amount of token
	pub quantity: TokenId,
	/// Price of this order.
	pub price: Balance,
	/// The balances to create an order
	pub deposit: Balance,
	/// This order will be invalidated after `deadline` block number.
	pub deadline: Option<BlockNumber>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_nft::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The basic amount of funds that must be reserved for an order.
		#[pallet::constant]
		type OrderDeposit: Get<BalanceOf<Self, I>>;

		/// The maximum amount of order an account owned
		#[pallet::constant]
		type MaxOrders: Get<u32>;

		/// The amount of trade fee as tax
		#[pallet::constant]
		type TradeFeeTaxRatio: Get<Perbill>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

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
			migrations::v1::post_migrate::<T, I>()
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Selling a nft asset, \[ class_id, token_id, quantity, account \]
		Selling(T::ClassId, T::TokenId, T::TokenId, T::AccountId),
		/// Make a deal with sell order, \[ class_id, token_id, quantity, from, to \]
		Dealed(T::ClassId, T::TokenId, T::TokenId, T::AccountId, T::AccountId),
		/// Removed an sell order , \[ class_id, token_id, quantity, account \]
		Removed(T::ClassId, T::TokenId, T::TokenId, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Invalid deaeline
		InvalidDeadline,
		/// Order not found
		OrderNotFound,
		/// To many order exceed T::MaxOrders
		TooManyOrders,
		/// A sell order already expired
		OrderExpired,
		/// Insufficient account balance.
		InsufficientFunds,
	}

	/// An index mapping from token to order.
	#[pallet::storage]
	#[pallet::getter(fn orders)]
	pub type Orders<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::TokenId,
		OrderDetails<T::AccountId, BalanceOf<T, I>, BlockNumberFor<T>, T::TokenId>,
		OptionQuery,
	>;

	/// The set of account orders.
	#[pallet::storage]
	#[pallet::getter(fn account_orders)]
	pub type AccountOrders<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<(ClassIdOf<T, I>, TokenIdOf<T, I>), T::MaxOrders>,
		ValueQuery,
	>;

	/// Storage version of the pallet.
	///
	/// New networks start with last version.
	#[pallet::storage]
	pub type StorageVersion<T: Config<I>, I: 'static = ()> = StorageValue<_, Releases, ValueQuery>;

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Create a order to sell a non-fungible asset
		#[pallet::weight(<T as Config<I>>::WeightInfo::sell())]
		#[transactional]
		pub fn sell(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			#[pallet::compact] quantity: T::TokenId,
			#[pallet::compact] price: BalanceOf<T, I>,
			deadline: Option<T::BlockNumber>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			pallet_nft::Pallet::<T, I>::ensure_transferable(class_id, token_id, quantity, &who)?;
			if let Some(ref deadline) = deadline {
				ensure!(
					<frame_system::Pallet<T>>::block_number() < *deadline,
					Error::<T, I>::InvalidDeadline
				);
			}
			T::Currency::reserve(&who, T::OrderDeposit::get())?;
			pallet_nft::Pallet::<T, I>::reserve(class_id, token_id, quantity, &who)?;
			let order = OrderDetails {
				owner: who.clone(),
				quantity,
				deposit: T::OrderDeposit::get(),
				price,
				deadline,
			};
			AccountOrders::<T, I>::try_mutate(&who, |ref mut orders| -> DispatchResult {
				orders
					.try_push((class_id, token_id))
					.map_err(|_| Error::<T, I>::TooManyOrders)?;
				Ok(())
			})?;
			Orders::<T, I>::insert(class_id, token_id, order);
			Self::deposit_event(Event::Selling(class_id, token_id, quantity, who));
			Ok(())
		}

		/// Create a order to buy a non-fungible asset
		#[pallet::weight(<T as Config<I>>::WeightInfo::deal())]
		#[transactional]
		pub fn deal(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let order = Orders::<T, I>::try_get(class_id, token_id)
				.map_err(|_| Error::<T, I>::OrderNotFound)?;
			if let Some(ref deadline) = order.deadline {
				ensure!(
					<frame_system::Pallet<T>>::block_number() <= *deadline,
					Error::<T, I>::OrderExpired
				);
			}
			ensure!(
				T::Currency::free_balance(&who) > order.price,
				Error::<T, I>::InsufficientFunds
			);
			pallet_nft::Pallet::<T, I>::swap(
				class_id,
				token_id,
				order.quantity,
				&order.owner,
				&who,
				order.price,
				T::TradeFeeTaxRatio::get(),
			)?;
			Self::delete_order(class_id, token_id)?;
			Self::deposit_event(Event::Dealed(
				class_id,
				token_id,
				order.quantity,
				order.owner.clone(),
				who,
			));
			Ok(())
		}

		/// Remove an order
		#[pallet::weight(<T as Config<I>>::WeightInfo::remove())]
		#[transactional]
		pub fn remove(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let order = Orders::<T, I>::try_get(class_id, token_id)
				.map_err(|_| Error::<T, I>::OrderNotFound)?;
			ensure!(who == order.owner, Error::<T, I>::OrderNotFound);
			pallet_nft::Pallet::<T, I>::unreserve(class_id, token_id, order.quantity, &who)?;
			Self::delete_order(class_id, token_id)?;
			Self::deposit_event(Event::Removed(class_id, token_id, order.quantity, who));
			Ok(())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	/// Remove order
	pub fn delete_order(class_id: ClassIdOf<T, I>, token_id: TokenIdOf<T, I>) -> DispatchResult {
		Orders::<T, I>::try_mutate_exists(class_id, token_id, |maybe_order| -> DispatchResult {
			let order = maybe_order.as_mut().ok_or(Error::<T, I>::OrderNotFound)?;
			T::Currency::unreserve(&order.owner, order.deposit);
			AccountOrders::<T, I>::try_mutate(&order.owner, |orders| -> DispatchResult {
				if let Some(idx) = orders.iter().position(|&v| v.0 == class_id && v.1 == token_id) {
					orders.remove(idx);
				}
				Ok(())
			})?;
			*maybe_order = None;
			Ok(())
		})?;
		Ok(())
	}
}
