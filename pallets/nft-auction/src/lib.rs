#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	dispatch::DispatchResult,
	traits::{Currency, Get, ReservableCurrency},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedAdd, One, Saturating},
	DispatchError, Perbill, RuntimeDebug,
};

pub use pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T, I = ()> = <<T as pallet_nft::Config<I>>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
pub type ClassIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::ClassId;
pub type InstanceIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::InstanceId;
pub type DutchAuctionOf<T, I = ()> = DutchAuction<
	<T as frame_system::Config>::AccountId,
	ClassIdOf<T, I>,
	InstanceIdOf<T, I>,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
>;
pub type EnglishAuctionOf<T, I = ()> = EnglishAuction<
	<T as frame_system::Config>::AccountId,
	ClassIdOf<T, I>,
	InstanceIdOf<T, I>,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
>;
pub type AuctionBidOf<T, I = ()> = AuctionBid<
	<T as frame_system::Config>::AccountId,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
>;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct DutchAuction<AccountId, ClassId, InstanceId, Balance, BlockNumber> {
	/// auction creator
	pub owner: AccountId,
	/// Nft class id
	#[codec(compact)]
	pub class: ClassId,
	/// Nft instance id
	#[codec(compact)]
	pub instance: InstanceId,
	/// The initial price of auction
	#[codec(compact)]
	pub min_price: Balance,
	/// If encountered this price, the auction should be finished.
	#[codec(compact)]
	pub max_price: Balance,
	/// The auction owner/creator should deposit some balances to create an auction.
	/// After this auction finishing or deleting, this balances
	/// will be returned to the auction owner.
	#[codec(compact)]
	pub deposit: Balance,
	/// When creating auction
	#[codec(compact)]
	pub created_at: BlockNumber,
	/// The auction should be forced to be ended if current block number higher than this value.
	#[codec(compact)]
	pub deadline: BlockNumber,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct EnglishAuction<AccountId, ClassId, InstanceId, Balance, BlockNumber> {
	/// auction creator
	pub owner: AccountId,
	/// Nft class id
	#[codec(compact)]
	pub class: ClassId,
	/// Nft instance id
	#[codec(compact)]
	pub instance: InstanceId,
	/// The initial price of auction
	#[codec(compact)]
	pub init_price: Balance,
	/// The next price of bid should be larger than old_price * ( 1 + min_raise_price )
	#[codec(compact)]
	pub min_raise_price: Balance,
	/// The auction owner/creator should deposit some balances to create an auction.
	/// After this auction finishing or deleting, this balances
	/// will be returned to the auction owner.
	#[codec(compact)]
	pub deposit: Balance,
	/// When creating auction
	#[codec(compact)]
	pub created_at: BlockNumber,
	/// The auction should be forced to be ended if current block number higher than this value.
	#[codec(compact)]
	pub deadline: BlockNumber,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct AuctionBid<AccountId, Balance, BlockNumber> {
	/// Who bid the auction
	pub account: AccountId,
	/// auction amount
	#[codec(compact)]
	pub price: Balance,
	/// When bid auction
	#[codec(compact)]
	pub bid_at: BlockNumber,
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
		/// Identifier for the auction
		type AuctionId: Member + Parameter + Default + Copy + HasCompact + AtLeast32BitUnsigned;
		/// The basic amount of funds that must be reserved for an order.
		#[pallet::constant]
		type AuctionDeposit: Get<BalanceOf<Self, I>>;
		/// The amount of auction fee as tax
		#[pallet::constant]
		type AuctionFeeTaxRatio: Get<Perbill>;
		/// Minimum deadline of auction
		#[pallet::constant]
		type MinDeadline: Get<BlockNumberFor<Self>>;
		/// Delay of auction after bidding
		#[pallet::constant]
		type DelayOfAuction: Get<BlockNumberFor<Self>>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	/// An index mapping from token to order.
	#[pallet::storage]
	#[pallet::getter(fn orders)]
	pub type Auctions<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::InstanceId,
		T::AuctionId,
		OptionQuery,
	>;

	/// An index mapping from auction_id to dutch auction.
	#[pallet::storage]
	#[pallet::getter(fn dutch_auctions)]
	pub type DutchAuctions<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::AuctionId, DutchAuctionOf<T, I>, OptionQuery>;

	/// An index mapping from auction_id to english auction.
	#[pallet::storage]
	#[pallet::getter(fn english_auctions)]
	pub type EnglishAuctions<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::AuctionId, EnglishAuctionOf<T, I>, OptionQuery>;

	/// An index mapping from auction_id to dutch auction bid.
	#[pallet::storage]
	#[pallet::getter(fn dutch_auction_bids)]
	pub type DutchAuctionBids<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::AuctionId, AuctionBidOf<T, I>, OptionQuery>;

	/// An index mapping from auction_id to english auction bid.
	#[pallet::storage]
	#[pallet::getter(fn english_auction_bids)]
	pub type EnglishAuctionBids<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::AuctionId, AuctionBidOf<T, I>, OptionQuery>;

	/// Current auction id, automate incr
	#[pallet::storage]
	#[pallet::getter(fn current_auction_id)]
	pub type CurrentAuctionId<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AuctionId, ValueQuery>;

	#[pallet::event]
	#[pallet::metadata(
		T::AccountId = "AccountId",
		T::ClassId = "ClassId",
		T::InstanceId = "InstanceId",
		T::AuctionId = "AuctionId"
	)]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Created ductch auction \[who, auction_id\]
		CreatedDutchAuction(T::AccountId, T::AuctionId),
		/// Bid dutch auction \[who auction_id\]
		BidDutchAuction(T::AccountId, T::AuctionId),
		/// Canceled dutch auction \[who, auction_id\]
		CanceledDutchAuction(T::AccountId, T::AuctionId),
		/// Redeemed dutch auction \[who, auction_id\]
		RedeemedDutchAuction(T::AccountId, T::AuctionId),
		/// Created ductch auction \[who, auction_id\]
		CreatedEnglishAuction(T::AccountId, T::AuctionId),
		/// Bid english auction \[who auction_id\]
		BidEnglishAuction(T::AccountId, T::AuctionId),
		/// Canceled english auction \[who, auction_id\]
		CanceledEnglishAuction(T::AccountId, T::AuctionId),
		/// Redeemed english auction \[who, auction_id\]
		RedeemedEnglishAuction(T::AccountId, T::AuctionId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		InvalidNFT,
		InvalidDeadline,
		InvalidPrice,
		InvalidNextAuctionId,
		AuctionNotFound,
		AuctionBidNotFound,
		AuctionClosed,
		SelfBid,
		MissDutchBidPrice,
		InvalidBidPrice,
		InsufficientFunds,
		NotBidAccount,
		NotOwnerAccount,
		CannotRedeemNow,
		CannotRemoveAuction,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Create an dutch auction.
		#[pallet::weight(<T as Config<I>>::WeightInfo::create_dutch())]
		pub fn create_dutch(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
			#[pallet::compact] min_price: BalanceOf<T, I>,
			#[pallet::compact] max_price: BalanceOf<T, I>,
			#[pallet::compact] deadline: BlockNumberFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pallet_nft::Pallet::<T, I>::validate(&class, &instance, &who),
				Error::<T, I>::InvalidNFT
			);
			ensure!(deadline >= T::MinDeadline::get(), Error::<T, I>::InvalidDeadline);
			ensure!(max_price > min_price, Error::<T, I>::InvalidPrice);

			let deposit = T::AuctionDeposit::get();
			T::Currency::reserve(&who, deposit)?;
			pallet_nft::Pallet::<T, I>::reserve(&class, &instance, &who)?;

			let auction_id = Self::gen_auction_id()?;
			let now = frame_system::Pallet::<T>::block_number();

			let auction = DutchAuction {
				owner: who.clone(),
				class,
				instance,
				min_price,
				max_price,
				created_at: now,
				deadline,
				deposit,
			};

			Auctions::<T, I>::insert(class, instance, auction_id);
			DutchAuctions::<T, I>::insert(auction_id, auction);

			Self::deposit_event(Event::CreatedDutchAuction(who, auction_id));
			Ok(().into())
		}

		/// Bid dutch auction
		///
		/// - `price`: bid price. If none, use current reduction price.
		#[pallet::weight(<T as Config<I>>::WeightInfo::bid_dutch())]
		pub fn bid_dutch(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
			price: Option<BalanceOf<T, I>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction =
				DutchAuctions::<T, I>::get(auction_id).ok_or(Error::<T, I>::AuctionNotFound)?;
			ensure!(auction.owner != who, Error::<T, I>::SelfBid);
			let maybe_bid = DutchAuctionBids::<T, I>::get(auction_id);
			match (maybe_bid, price) {
				(None, price) => {
					let now = frame_system::Pallet::<T>::block_number();
					ensure!(auction.deadline >= now, Error::<T, I>::AuctionClosed);
					let mut new_price = Self::get_dutch_price(&auction, now);
					if let Some(bid_price) = price {
						ensure!(bid_price >= new_price, Error::<T, I>::InvalidBidPrice);
						new_price = bid_price
					}
					let bid = AuctionBid { account: who.clone(), price: new_price, bid_at: now };
					if new_price >= auction.max_price {
						ensure!(
							T::Currency::free_balance(&who) > new_price,
							Error::<T, I>::InsufficientFunds
						);
						Self::do_redeem_dutch_auction(&auction_id, &auction, &bid)?;
					} else {
						T::Currency::reserve(&who, new_price)?;
						DutchAuctionBids::<T, I>::insert(auction_id, bid);
						Self::deposit_event(Event::BidDutchAuction(who, auction_id));
					}
				},
				(Some(bid), Some(bid_price)) => {
					let now = frame_system::Pallet::<T>::block_number();
					ensure!(
						bid.bid_at.saturating_add(T::DelayOfAuction::get()) >= now,
						Error::<T, I>::AuctionClosed
					);
					T::Currency::unreserve(&bid.account, bid.price);
					let new_bid =
						AuctionBid { account: who.clone(), price: bid_price, bid_at: now };
					if bid_price >= auction.max_price {
						ensure!(
							T::Currency::free_balance(&who) > bid_price,
							Error::<T, I>::InsufficientFunds
						);
						Self::do_redeem_dutch_auction(&auction_id, &auction, &new_bid)?;
					} else {
						ensure!(bid_price > bid.price, Error::<T, I>::InvalidBidPrice);
						T::Currency::reserve(&who, bid_price)?;
						DutchAuctionBids::<T, I>::insert(auction_id, new_bid);
						Self::deposit_event(Event::BidDutchAuction(who, auction_id));
					}
				},
				(Some(_), None) => return Err(Error::<T, I>::MissDutchBidPrice.into()),
			}
			Ok(().into())
		}

		/// Redeem duction
		#[pallet::weight(<T as Config<I>>::WeightInfo::redeem_dutch())]
		pub fn redeem_dutch(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction =
				DutchAuctions::<T, I>::get(auction_id).ok_or(Error::<T, I>::AuctionNotFound)?;
			let bid = DutchAuctionBids::<T, I>::get(auction_id)
				.ok_or(Error::<T, I>::AuctionBidNotFound)?;
			ensure!(bid.account == who, Error::<T, I>::NotBidAccount);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(
				bid.bid_at.saturating_add(T::DelayOfAuction::get()) < now,
				Error::<T, I>::CannotRedeemNow
			);
			T::Currency::unreserve(&bid.account, bid.price);
			Self::do_redeem_dutch_auction(&auction_id, &auction, &bid)
		}

		/// Cancel auction, only auction without any bid can be canceled
		#[pallet::weight(<T as Config<I>>::WeightInfo::cancel_dutch())]
		pub fn cancel_dutch(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction =
				DutchAuctions::<T, I>::get(auction_id).ok_or(Error::<T, I>::AuctionNotFound)?;
			ensure!(auction.owner == who, Error::<T, I>::NotOwnerAccount);
			let bid = DutchAuctionBids::<T, I>::get(auction_id);
			ensure!(bid.is_none(), Error::<T, I>::CannotRemoveAuction);
			Self::delete_dutch_auction(&auction_id)?;
			Self::deposit_event(Event::CanceledDutchAuction(who, auction_id));
			Ok(().into())
		}

		/// Create an english auction.
		#[pallet::weight(<T as Config<I>>::WeightInfo::create_english())]
		pub fn create_english(
			origin: OriginFor<T>,
			#[pallet::compact] class: T::ClassId,
			#[pallet::compact] instance: T::InstanceId,
			#[pallet::compact] init_price: BalanceOf<T, I>,
			#[pallet::compact] min_raise_price: BalanceOf<T, I>,
			#[pallet::compact] deadline: BlockNumberFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pallet_nft::Pallet::<T, I>::validate(&class, &instance, &who),
				Error::<T, I>::InvalidNFT
			);
			ensure!(deadline >= T::MinDeadline::get(), Error::<T, I>::InvalidDeadline);

			let deposit = T::AuctionDeposit::get();
			T::Currency::reserve(&who, deposit)?;
			pallet_nft::Pallet::<T, I>::reserve(&class, &instance, &who)?;

			let auction_id = Self::gen_auction_id()?;
			let now = frame_system::Pallet::<T>::block_number();

			let auction = EnglishAuction {
				owner: who.clone(),
				class,
				instance,
				init_price,
				min_raise_price,
				created_at: now,
				deadline,
				deposit,
			};

			Auctions::<T, I>::insert(class, instance, auction_id);
			EnglishAuctions::<T, I>::insert(auction_id, auction);

			Self::deposit_event(Event::CreatedEnglishAuction(who, auction_id));
			Ok(().into())
		}

		/// Bid english auction
		#[pallet::weight(<T as Config<I>>::WeightInfo::bid_english())]
		pub fn bid_english(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
			#[pallet::compact] price: BalanceOf<T, I>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction =
				EnglishAuctions::<T, I>::get(auction_id).ok_or(Error::<T, I>::AuctionNotFound)?;
			ensure!(auction.owner != who, Error::<T, I>::SelfBid);
			let maybe_bid = EnglishAuctionBids::<T, I>::get(auction_id);
			match maybe_bid {
				None => {
					let now = frame_system::Pallet::<T>::block_number();
					ensure!(auction.deadline >= now, Error::<T, I>::AuctionClosed);
					T::Currency::reserve(&who, price)?;
					EnglishAuctionBids::<T, I>::insert(
						auction_id,
						AuctionBid { account: who.clone(), price, bid_at: now },
					);
					Self::deposit_event(Event::BidEnglishAuction(who, auction_id));
				},
				Some(bid) => {
					let now = frame_system::Pallet::<T>::block_number();
					ensure!(
						bid.bid_at.saturating_add(T::DelayOfAuction::get()) >= now,
						Error::<T, I>::AuctionClosed
					);
					T::Currency::unreserve(&bid.account, bid.price);
					ensure!(
						price >= bid.price.saturating_add(auction.min_raise_price),
						Error::<T, I>::InvalidBidPrice
					);
					T::Currency::reserve(&who, price)?;
					EnglishAuctionBids::<T, I>::insert(
						auction_id,
						AuctionBid { account: who.clone(), price, bid_at: now },
					);
					Self::deposit_event(Event::BidEnglishAuction(who, auction_id));
				},
			}
			Ok(().into())
		}

		/// Redeem duction
		#[pallet::weight(<T as Config<I>>::WeightInfo::redeem_english())]
		pub fn redeem_english(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction =
				EnglishAuctions::<T, I>::get(auction_id).ok_or(Error::<T, I>::AuctionNotFound)?;
			let bid = EnglishAuctionBids::<T, I>::get(auction_id)
				.ok_or(Error::<T, I>::AuctionBidNotFound)?;
			ensure!(bid.account == who, Error::<T, I>::NotBidAccount);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(
				bid.bid_at.saturating_add(T::DelayOfAuction::get()) < now,
				Error::<T, I>::CannotRedeemNow
			);
			T::Currency::unreserve(&bid.account, bid.price);

			pallet_nft::Pallet::<T, I>::swap(
				&auction.class,
				&auction.instance,
				&bid.account,
				bid.price,
				T::AuctionFeeTaxRatio::get(),
			)?;

			Self::delete_english_auction(&auction_id)?;
			Self::deposit_event(Event::RedeemedEnglishAuction(
				bid.account.clone(),
				auction_id.clone(),
			));
			Ok(().into())
		}

		/// Cancel auction, only auction without any bid can be canceled
		#[pallet::weight(<T as Config<I>>::WeightInfo::cancel_english())]
		pub fn cancel_english(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction =
				EnglishAuctions::<T, I>::get(auction_id).ok_or(Error::<T, I>::AuctionNotFound)?;
			ensure!(auction.owner == who, Error::<T, I>::NotOwnerAccount);
			let bid = EnglishAuctionBids::<T, I>::get(auction_id);
			ensure!(bid.is_none(), Error::<T, I>::CannotRemoveAuction);
			Self::delete_english_auction(&auction_id)?;
			Self::deposit_event(Event::CanceledEnglishAuction(who, auction_id));
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn gen_auction_id() -> Result<T::AuctionId, DispatchError> {
		CurrentAuctionId::<T, I>::try_mutate(|id| -> Result<T::AuctionId, DispatchError> {
			let current_id = *id;
			*id = id.checked_add(&One::one()).ok_or(Error::<T, I>::InvalidNextAuctionId)?;
			Ok(current_id)
		})
	}

	pub fn get_dutch_price(
		auction: &DutchAuctionOf<T, I>,
		now: BlockNumberFor<T>,
	) -> BalanceOf<T, I> {
		let diff_price = auction.max_price.saturating_sub(auction.min_price);
		let rate = Perbill::from_rational(
			now.saturating_sub(auction.created_at).min(auction.deadline),
			auction.deadline,
		);
		let dec_price = rate * diff_price;
		auction.max_price.saturating_sub(dec_price)
	}

	fn do_redeem_dutch_auction(
		auction_id: &T::AuctionId,
		auction: &DutchAuctionOf<T, I>,
		bid: &AuctionBidOf<T, I>,
	) -> DispatchResult {
		pallet_nft::Pallet::<T, I>::swap(
			&auction.class,
			&auction.instance,
			&bid.account,
			bid.price,
			T::AuctionFeeTaxRatio::get(),
		)?;
		Self::delete_dutch_auction(&auction_id)?;
		Self::deposit_event(Event::RedeemedDutchAuction(bid.account.clone(), auction_id.clone()));
		Ok(())
	}

	fn delete_dutch_auction(auction_id: &T::AuctionId) -> DispatchResult {
		DutchAuctions::<T, I>::try_mutate_exists(auction_id, |maybe_auction| -> DispatchResult {
			let auction = maybe_auction.as_mut().ok_or(Error::<T, I>::AuctionNotFound)?;
			T::Currency::unreserve(&auction.owner, auction.deposit);
			DutchAuctionBids::<T, I>::remove(auction_id);
			Auctions::<T, I>::remove(auction.class, auction.instance);
			*maybe_auction = None;
			Ok(())
		})?;
		Ok(())
	}

	fn delete_english_auction(auction_id: &T::AuctionId) -> DispatchResult {
		EnglishAuctions::<T, I>::try_mutate_exists(
			auction_id,
			|maybe_auction| -> DispatchResult {
				let auction = maybe_auction.as_mut().ok_or(Error::<T, I>::AuctionNotFound)?;
				T::Currency::unreserve(&auction.owner, auction.deposit);
				EnglishAuctionBids::<T, I>::remove(auction_id);
				Auctions::<T, I>::remove(auction.class, auction.instance);
				*maybe_auction = None;
				Ok(())
			},
		)?;
		Ok(())
	}
}
