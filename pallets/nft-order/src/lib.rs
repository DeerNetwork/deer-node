#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	dispatch::DispatchResult,
	traits::{Currency, ReservableCurrency},
};
use sp_runtime::{Perbill, RuntimeDebug};
use sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T, I = ()> = <<T as pallet_nft::Config<I>>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
pub type ClassIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::ClassId;
pub type InstanceIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::InstanceId;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen)]
pub struct OrderDetails<AccountId, Balance, BlockNumber> {
	/// Who create the order.
	pub owner: AccountId,
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

	#[pallet::event]
	#[pallet::metadata(
		T::AccountId = "AccountId",
		T::ClassId = "ClassId",
		T::InstanceId = "InstanceId"
	)]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Selling a nft asset, \[ class, instance, account \]
		Selling(T::ClassId, T::InstanceId, T::AccountId),
		/// Make a deal with sell order, \[ class, instance, from, to \]
		Dealed(T::ClassId, T::InstanceId, T::AccountId, T::AccountId),
		/// Removed an sell order , \[ class, instance, account \]
		Removed(T::ClassId, T::InstanceId, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Not own the asset
		NotOwn,
		/// Invalid NFt
		InvalidNFT,
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
		T::InstanceId,
		OrderDetails<T::AccountId, BalanceOf<T, I>, BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// The set of account orders.
	#[pallet::storage]
	#[pallet::getter(fn account_orders)]
	pub type AccountOrders<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<(ClassIdOf<T, I>, InstanceIdOf<T, I>), T::MaxOrders>,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Create a order to sell a non-fungible asset
		#[pallet::weight(<T as Config<I>>::WeightInfo::sell())]
		pub fn sell(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
			#[pallet::compact] price: BalanceOf<T, I>,
			deadline: Option<T::BlockNumber>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pallet_nft::Pallet::<T, I>::validate(&class, &instance, &who),
				Error::<T, I>::InvalidNFT
			);
			if let Some(ref deadline) = deadline {
				ensure!(
					<frame_system::Pallet<T>>::block_number() < *deadline,
					Error::<T, I>::InvalidDeadline
				);
			}
			T::Currency::reserve(&who, T::OrderDeposit::get())?;
			pallet_nft::Pallet::<T, I>::reserve(&class, &instance, &who)?;
			let order = OrderDetails {
				owner: who.clone(),
				deposit: T::OrderDeposit::get(),
				price,
				deadline,
			};
			AccountOrders::<T, I>::try_mutate(&who, |ref mut orders| -> DispatchResult {
				orders.try_push((class, instance)).map_err(|_| Error::<T, I>::TooManyOrders)?;
				Ok(())
			})?;
			Orders::<T, I>::insert(class, instance, order);
			Self::deposit_event(Event::Selling(class, instance, who));
			Ok(())
		}

		/// Create a order to buy a non-fungible asset
		#[pallet::weight(<T as Config<I>>::WeightInfo::deal())]
		pub fn deal(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let order = Orders::<T, I>::try_get(class, instance)
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
				&class,
				&instance,
				&who,
				order.price,
				T::TradeFeeTaxRatio::get(),
			)?;
			Self::delete_order(class, instance)?;
			Self::deposit_event(Event::Dealed(class, instance, order.owner.clone(), who));
			Ok(())
		}

		/// Remove an order
		#[pallet::weight(<T as Config<I>>::WeightInfo::remove())]
		pub fn remove(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let order = Orders::<T, I>::try_get(class, instance)
				.map_err(|_| Error::<T, I>::OrderNotFound)?;
			ensure!(who == order.owner, Error::<T, I>::OrderNotFound);
			pallet_nft::Pallet::<T, I>::unreserve(&class, &instance)?;
			Self::delete_order(class, instance)?;
			Self::deposit_event(Event::Removed(class, instance, who));
			Ok(())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	/// Remove order
	pub fn delete_order(class: ClassIdOf<T, I>, instance: InstanceIdOf<T, I>) -> DispatchResult {
		Orders::<T, I>::try_mutate_exists(class, instance, |maybe_order| -> DispatchResult {
			let order = maybe_order.as_mut().ok_or(Error::<T, I>::OrderNotFound)?;
			T::Currency::unreserve(&order.owner, order.deposit);
			AccountOrders::<T, I>::try_mutate(&order.owner, |orders| -> DispatchResult {
				if let Some(idx) = orders.iter().position(|&v| v.0 == class && v.1 == instance) {
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
