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
	traits::{Currency, ReservableCurrency},
	transactional,
	weights::Weight,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedAdd, One, Saturating, StaticLookup},
	Perbill, RuntimeDebug,
};
use sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T, I = ()> = <<T as pallet_nft::Config<I>>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
pub type ClassIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::ClassId;
pub type TokenIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::TokenId;
pub type QuantityOf<T, I = ()> = <T as pallet_nft::Config<I>>::Quantity;
pub type OrderDetailsOf<T, I = ()> = OrderDetails<
	ClassIdOf<T, I>,
	TokenIdOf<T, I>,
	QuantityOf<T, I>,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
>;
pub type OfferDetailsOf<T, I = ()> = OfferDetails<
	ClassIdOf<T, I>,
	TokenIdOf<T, I>,
	QuantityOf<T, I>,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
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

/// Order detail
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct OrderDetails<ClassId, TokenId, Quantity, Balance, BlockNumber> {
	/// Nft class id
	#[codec(compact)]
	pub class_id: ClassId,
	/// Nft token id
	#[codec(compact)]
	pub token_id: TokenId,
	/// Amount of tokens in sale
	#[codec(compact)]
	pub quantity: Quantity,
	/// Total amount of tokens
	#[codec(compact)]
	pub total_quantity: Quantity,
	/// Price of this order.
	pub price: Balance,
	/// The balances to create an order
	pub deposit: Balance,
	/// This order will be invalidated after `deadline` block number.
	pub deadline: Option<BlockNumber>,
}

/// Offer detail
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct OfferDetails<ClassId, TokenId, Quantity, Balance, BlockNumber> {
	/// Nft class id
	#[codec(compact)]
	pub class_id: ClassId,
	/// Nft token id
	#[codec(compact)]
	pub token_id: TokenId,
	/// Amount of tokens
	#[codec(compact)]
	pub quantity: Quantity,
	/// Price of this order.
	pub price: Balance,
	/// This order will be invalidated after `deadline` block number.
	pub deadline: Option<BlockNumber>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, Blake2_128Concat};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_nft::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// Identifier for the order and offer
		type OrderId: Member + Parameter + Default + Copy + HasCompact + AtLeast32BitUnsigned;

		/// The basic amount of funds that must be reserved for an order.
		#[pallet::constant]
		type OrderDeposit: Get<BalanceOf<Self, I>>;

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
		/// Create sell order.
		CreatedOrder { order_id: T::OrderId, seller: T::AccountId },
		/// Make a deal with sell order.
		DealedOrder {
			order_id: T::OrderId,
			seller: T::AccountId,
			buyer: T::AccountId,
			quantity: T::Quantity,
			fee: BalanceOf<T, I>,
		},
		/// Remove an sell order.
		RemovedOrder { order_id: T::OrderId, seller: T::AccountId },
		/// Create buy offer.
		CreatedOffer { offer_id: T::OrderId, buyer: T::AccountId },
		/// Make a deal with buy offer.
		DealedOffer {
			offer_id: T::OrderId,
			buyer: T::AccountId,
			seller: T::AccountId,
			quantity: T::Quantity,
			fee: BalanceOf<T, I>,
		},
		/// Remove an buy offer.
		RemovedOffer { offer_id: T::OrderId, buyer: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Invalid deaeline
		InvalidDeadline,
		/// Invalid quantity
		InvalidQuantity,
		/// Order not found
		OrderNotFound,
		/// A sell order already expired
		OrderExpired,
		/// Insufficient account balance.
		InsufficientFunds,
		/// No available order ID
		NoAvailableOrderId,
		/// Offer not found
		OfferNotFound,
		/// A buy offer already expired
		OfferExpired,
		/// No available offer ID
		NoAvailableOfferId,
	}

	/// Order collections
	#[pallet::storage]
	#[pallet::getter(fn orders)]
	pub type Orders<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		T::OrderId,
		OrderDetailsOf<T, I>,
		OptionQuery,
	>;

	/// Next order id
	#[pallet::storage]
	#[pallet::getter(fn next_order_id)]
	pub type NextOrderId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::OrderId, ValueQuery>;

	/// Offer collections
	#[pallet::storage]
	#[pallet::getter(fn offers)]
	pub type Offers<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		T::OrderId,
		OfferDetailsOf<T, I>,
		OptionQuery,
	>;

	/// Next offer id
	#[pallet::storage]
	#[pallet::getter(fn next_offer_id)]
	pub type NextOfferId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::OrderId, ValueQuery>;

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
			#[pallet::compact] quantity: T::Quantity,
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
			NextOrderId::<T, I>::try_mutate(|id| -> DispatchResult {
				let order_id = *id;
				*id = id.checked_add(&One::one()).ok_or(Error::<T, I>::NoAvailableOrderId)?;

				T::Currency::reserve(&who, T::OrderDeposit::get())?;
				pallet_nft::Pallet::<T, I>::reserve(class_id, token_id, quantity, &who)?;
				let order = OrderDetails {
					class_id,
					token_id,
					quantity,
					total_quantity: quantity,
					deposit: T::OrderDeposit::get(),
					price,
					deadline,
				};
				Orders::<T, I>::insert(who.clone(), order_id, order);

				Self::deposit_event(Event::CreatedOrder { order_id, seller: who });
				Ok(())
			})
		}

		/// Deal an order
		#[pallet::weight(<T as Config<I>>::WeightInfo::deal_order())]
		#[transactional]
		pub fn deal_order(
			origin: OriginFor<T>,
			order_owner: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] order_id: T::OrderId,
			#[pallet::compact] quantity: T::Quantity,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let seller = T::Lookup::lookup(order_owner)?;

			Orders::<T, I>::try_mutate_exists(
				seller.clone(),
				order_id,
				|maybe_order| -> DispatchResult {
					let order = maybe_order.as_mut().ok_or(Error::<T, I>::OrderNotFound)?;
					let order_quantity = order.quantity;

					ensure!(
						quantity <= order_quantity && quantity >= One::one(),
						Error::<T, I>::InvalidQuantity
					);

					if let Some(ref deadline) = order.deadline {
						ensure!(
							<frame_system::Pallet<T>>::block_number() <= *deadline,
							Error::<T, I>::OrderExpired
						);
					}

					let fee = Perbill::from_rational(quantity, order.total_quantity) * order.price;

					ensure!(
						T::Currency::free_balance(&who) >= fee,
						Error::<T, I>::InsufficientFunds
					);

					let class_id = order.class_id;
					let token_id = order.token_id;
					pallet_nft::Pallet::<T, I>::unreserve(class_id, token_id, quantity, &seller)?;
					pallet_nft::Pallet::<T, I>::swap(
						class_id,
						token_id,
						quantity,
						&seller,
						&who,
						fee,
						T::TradeFeeTaxRatio::get(),
						pallet_nft::TransferReason::Order,
					)?;

					if quantity == order_quantity {
						T::Currency::unreserve(&seller, order.deposit);
						*maybe_order = None;
					} else {
						order.quantity = order.quantity.saturating_sub(quantity);
					}

					Self::deposit_event(Event::DealedOrder {
						order_id,
						seller,
						buyer: who,
						quantity,
						fee,
					});
					Ok(())
				},
			)
		}

		/// Remove an order
		#[pallet::weight(<T as Config<I>>::WeightInfo::remove_order())]
		#[transactional]
		pub fn remove_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: T::OrderId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Orders::<T, I>::try_mutate_exists(
				who.clone(),
				order_id,
				|maybe_order| -> DispatchResult {
					let order = maybe_order.as_mut().ok_or(Error::<T, I>::OrderNotFound)?;

					let class_id = order.class_id;
					let token_id = order.token_id;
					let quantity = order.quantity;

					pallet_nft::Pallet::<T, I>::unreserve(class_id, token_id, quantity, &who)?;
					T::Currency::unreserve(&who, order.deposit);

					*maybe_order = None;

					Self::deposit_event(Event::RemovedOrder { order_id, seller: who });
					Ok(())
				},
			)
		}

		/// Create a offer to buy a non-fungible asset
		#[pallet::weight(<T as Config<I>>::WeightInfo::buy())]
		#[transactional]
		pub fn buy(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			#[pallet::compact] quantity: T::Quantity,
			#[pallet::compact] price: BalanceOf<T, I>,
			deadline: Option<T::BlockNumber>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			if let Some(ref deadline) = deadline {
				ensure!(
					<frame_system::Pallet<T>>::block_number() < *deadline,
					Error::<T, I>::InvalidDeadline
				);
			}

			pallet_nft::Pallet::<T, I>::inc_consumers(class_id, token_id)?;

			NextOfferId::<T, I>::try_mutate(|id| -> DispatchResult {
				let offer_id = *id;
				*id = id.checked_add(&One::one()).ok_or(Error::<T, I>::NoAvailableOfferId)?;

				T::Currency::reserve(&who, price)?;
				let offer = OfferDetails { class_id, token_id, quantity, price, deadline };
				Offers::<T, I>::insert(who.clone(), offer_id, offer);

				Self::deposit_event(Event::CreatedOffer { offer_id, buyer: who });
				Ok(())
			})
		}

		/// Deal an offer
		#[pallet::weight(<T as Config<I>>::WeightInfo::deal_offer())]
		#[transactional]
		pub fn deal_offer(
			origin: OriginFor<T>,
			offer_owner: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] offer_id: T::OrderId,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let buyer = T::Lookup::lookup(offer_owner)?;

			Offers::<T, I>::try_mutate_exists(
				buyer.clone(),
				offer_id,
				|maybe_offer| -> DispatchResult {
					let offer = maybe_offer.as_mut().ok_or(Error::<T, I>::OfferNotFound)?;

					if let Some(ref deadline) = offer.deadline {
						ensure!(
							<frame_system::Pallet<T>>::block_number() <= *deadline,
							Error::<T, I>::OfferExpired
						);
					}

					T::Currency::unreserve(&buyer, offer.price);

					let class_id = offer.class_id;
					let token_id = offer.token_id;
					let quantity = offer.quantity;

					pallet_nft::Pallet::<T, I>::dec_consumers(class_id, token_id)?;

					pallet_nft::Pallet::<T, I>::swap(
						class_id,
						token_id,
						quantity,
						&owner,
						&buyer,
						offer.price,
						T::TradeFeeTaxRatio::get(),
						pallet_nft::TransferReason::Offer,
					)?;

					Self::deposit_event(Event::DealedOffer {
						offer_id,
						buyer,
						seller: owner,
						quantity,
						fee: offer.price,
					});

					*maybe_offer = None;
					Ok(())
				},
			)
		}

		/// Remove an offer
		#[pallet::weight(<T as Config<I>>::WeightInfo::remove_offer())]
		#[transactional]
		pub fn remove_offer(
			origin: OriginFor<T>,
			#[pallet::compact] offer_id: T::OrderId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Offers::<T, I>::try_mutate_exists(
				who.clone(),
				offer_id,
				|maybe_offer| -> DispatchResult {
					let offer = maybe_offer.as_mut().ok_or(Error::<T, I>::OfferNotFound)?;

					let class_id = offer.class_id;
					let token_id = offer.token_id;

					pallet_nft::Pallet::<T, I>::dec_consumers(class_id, token_id)?;

					T::Currency::unreserve(&who, offer.price);

					*maybe_offer = None;

					Self::deposit_event(Event::RemovedOffer { offer_id, buyer: who });
					Ok(())
				},
			)
		}
	}
}
