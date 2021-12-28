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
	traits::{Currency, Get, ReservableCurrency},
	transactional,
	weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedAdd, One, Saturating, StaticLookup},
	DispatchError, Perbill, RuntimeDebug,
};

pub use pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T, I = ()> = <<T as pallet_nft::Config<I>>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
pub type ClassIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::ClassId;
pub type TokenIdOf<T, I = ()> = <T as pallet_nft::Config<I>>::TokenId;
pub type QuantityOf<T, I = ()> = <T as pallet_nft::Config<I>>::Quantity;
pub type DutchAuctionOf<T, I = ()> = DutchAuction<
	ClassIdOf<T, I>,
	TokenIdOf<T, I>,
	QuantityOf<T, I>,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
>;
pub type EnglishAuctionOf<T, I = ()> = EnglishAuction<
	ClassIdOf<T, I>,
	TokenIdOf<T, I>,
	QuantityOf<T, I>,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
>;
pub type AuctionBidOf<T, I = ()> = AuctionBid<
	<T as frame_system::Config>::AccountId,
	BalanceOf<T, I>,
	<T as frame_system::Config>::BlockNumber,
>;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct DutchAuction<ClassId, TokenId, Quantity, Balance, BlockNumber> {
	/// Nft class id
	#[codec(compact)]
	pub class_id: ClassId,
	/// Nft token id
	#[codec(compact)]
	pub token_id: TokenId,
	/// Amount of token
	#[codec(compact)]
	pub quantity: Quantity,
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
	/// When opening auction
	#[codec(compact)]
	pub open_at: BlockNumber,
	/// The auction should be forced to be ended if current block number higher than this value.
	#[codec(compact)]
	pub deadline: BlockNumber,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct EnglishAuction<ClassId, TokenId, Quantity, Balance, BlockNumber> {
	/// Nft class id
	#[codec(compact)]
	pub class_id: ClassId,
	/// Nft token id
	#[codec(compact)]
	pub token_id: TokenId,
	/// Amount of token
	#[codec(compact)]
	pub quantity: Quantity,
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
	/// When opening auction
	#[codec(compact)]
	pub open_at: BlockNumber,
	/// The auction should be forced to be ended if current block number higher than this value.
	#[codec(compact)]
	pub deadline: BlockNumber,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
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

// A value placed in storage that represents the current version of the Scheduler storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum Releases {
	V0,
	V1,
	V2,
}

impl Default for Releases {
	fn default() -> Self {
		Releases::V0
	}
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

	/// An index mapping from auction_id to dutch auction.
	#[pallet::storage]
	#[pallet::getter(fn dutch_auctions)]
	pub type DutchAuctions<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		T::AuctionId,
		DutchAuctionOf<T, I>,
		OptionQuery,
	>;

	/// An index mapping from auction_id to english auction.
	#[pallet::storage]
	#[pallet::getter(fn english_auctions)]
	pub type EnglishAuctions<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		T::AuctionId,
		EnglishAuctionOf<T, I>,
		OptionQuery,
	>;

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
	#[pallet::getter(fn next_auction_id)]
	pub type NextAuctionId<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AuctionId, ValueQuery>;

	/// Storage version of the pallet.
	///
	/// New networks start with last version.
	#[pallet::storage]
	pub type StorageVersion<T: Config<I>, I: 'static = ()> = StorageValue<_, Releases, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Created ductch auction.
		CreatedDutchAuction {
			auction_id: T::AuctionId,
			class_id: T::ClassId,
			token_id: T::TokenId,
			quantity: T::Quantity,
			owner: T::AccountId,
		},
		/// Bid dutch auction.
		BidDutchAuction {
			auction_id: T::AuctionId,
			bidder: T::AccountId,
			owner: T::AccountId,
			price: BalanceOf<T, I>,
		},
		/// Canceled dutch auction.
		CanceledDutchAuction { auction_id: T::AuctionId, owner: T::AccountId },
		/// Redeemed dutch auction.
		RedeemedDutchAuction {
			auction_id: T::AuctionId,
			bidder: T::AccountId,
			owner: T::AccountId,
			price: BalanceOf<T, I>,
		},
		/// Created ductch auction.
		CreatedEnglishAuction {
			auction_id: T::AuctionId,
			class_id: T::ClassId,
			token_id: T::TokenId,
			quantity: T::Quantity,
			owner: T::AccountId,
		},
		/// Bid english auction.
		BidEnglishAuction {
			auction_id: T::AuctionId,
			bidder: T::AccountId,
			owner: T::AccountId,
			price: BalanceOf<T, I>,
		},
		/// Canceled english auction.
		CanceledEnglishAuction { auction_id: T::AuctionId, owner: T::AccountId },
		/// Redeemed english auction.
		RedeemedEnglishAuction {
			auction_id: T::AuctionId,
			bidder: T::AccountId,
			owner: T::AccountId,
			price: BalanceOf<T, I>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		InvalidDeadline,
		InvalidPrice,
		InvalidNextAuctionId,
		AuctionNotOpen,
		AuctionNotFound,
		AuctionBidNotFound,
		AuctionClosed,
		SelfBid,
		MissDutchBidPrice,
		InvalidBidPrice,
		InsufficientFunds,
		CannotRedeemNow,
		CannotRemoveAuction,
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
			if StorageVersion::<T, I>::get() == Releases::V1 {
				migrations::v2::migrate::<T, I>()
			} else {
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			if StorageVersion::<T, I>::get() == Releases::V1 {
				migrations::v2::pre_migrate::<T, I>()
			} else {
				Ok(())
			}
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			migrations::v2::post_migrate::<T, I>()
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Create an dutch auction.
		#[pallet::weight(<T as Config<I>>::WeightInfo::create_dutch())]
		#[transactional]
		pub fn create_dutch(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			#[pallet::compact] quantity: T::Quantity,
			#[pallet::compact] min_price: BalanceOf<T, I>,
			#[pallet::compact] max_price: BalanceOf<T, I>,
			#[pallet::compact] deadline: BlockNumberFor<T>,
			open_at: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			pallet_nft::Pallet::<T, I>::ensure_transferable(class_id, token_id, quantity, &who)?;
			ensure!(max_price > min_price, Error::<T, I>::InvalidPrice);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(
				deadline >= now.saturating_add(T::MinDeadline::get()),
				Error::<T, I>::InvalidDeadline
			);
			let open_at = open_at.map(|v| v.max(now)).unwrap_or(now);

			let deposit = T::AuctionDeposit::get();
			T::Currency::reserve(&who, deposit)?;
			pallet_nft::Pallet::<T, I>::reserve(class_id, token_id, quantity, &who)?;

			let auction_id = Self::new_auction_id()?;

			let auction = DutchAuction {
				class_id,
				token_id,
				quantity,
				min_price,
				max_price,
				created_at: now,
				open_at,
				deadline,
				deposit,
			};

			DutchAuctions::<T, I>::insert(who.clone(), auction_id, auction);

			Self::deposit_event(Event::CreatedDutchAuction {
				auction_id,
				class_id,
				token_id,
				quantity,
				owner: who,
			});
			Ok(())
		}

		/// Bid dutch auction
		///
		/// - `price`: bid price. If none, use current reduction price.
		#[pallet::weight(<T as Config<I>>::WeightInfo::bid_dutch())]
		#[transactional]
		pub fn bid_dutch(
			origin: OriginFor<T>,
			auction_owner: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] auction_id: T::AuctionId,
			price: Option<BalanceOf<T, I>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction_owner = T::Lookup::lookup(auction_owner)?;
			let auction = DutchAuctions::<T, I>::get(&auction_owner, auction_id)
				.ok_or(Error::<T, I>::AuctionNotFound)?;
			ensure!(&auction_owner != &who, Error::<T, I>::SelfBid);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(auction.open_at <= now, Error::<T, I>::AuctionNotOpen);
			let maybe_bid = DutchAuctionBids::<T, I>::get(auction_id);
			match (maybe_bid, price) {
				(None, price) => {
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
						Self::do_redeem_dutch_auction(&auction_owner, auction_id, &auction, &bid)?;
					} else {
						T::Currency::reserve(&who, new_price)?;
						DutchAuctionBids::<T, I>::insert(auction_id, bid);
						Self::deposit_event(Event::BidDutchAuction {
							auction_id,
							bidder: who,
							owner: auction_owner.clone(),
							price: new_price,
						});
					}
				},
				(Some(bid), Some(bid_price)) => {
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
						Self::do_redeem_dutch_auction(
							&auction_owner,
							auction_id,
							&auction,
							&new_bid,
						)?;
					} else {
						ensure!(bid_price > bid.price, Error::<T, I>::InvalidBidPrice);
						T::Currency::reserve(&who, bid_price)?;
						DutchAuctionBids::<T, I>::insert(auction_id, new_bid);
						Self::deposit_event(Event::BidDutchAuction {
							auction_id,
							bidder: who,
							owner: auction_owner.clone(),
							price: bid_price,
						});
					}
				},
				(Some(_), None) => return Err(Error::<T, I>::MissDutchBidPrice.into()),
			}
			Ok(())
		}

		/// Redeem duction
		#[pallet::weight(<T as Config<I>>::WeightInfo::redeem_dutch())]
		#[transactional]
		pub fn redeem_dutch(
			origin: OriginFor<T>,
			auction_owner: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let auction_owner = T::Lookup::lookup(auction_owner)?;
			let auction = DutchAuctions::<T, I>::get(&auction_owner, auction_id)
				.ok_or(Error::<T, I>::AuctionNotFound)?;
			let bid = DutchAuctionBids::<T, I>::get(auction_id)
				.ok_or(Error::<T, I>::AuctionBidNotFound)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(
				bid.bid_at.saturating_add(T::DelayOfAuction::get()) < now,
				Error::<T, I>::CannotRedeemNow
			);
			T::Currency::unreserve(&bid.account, bid.price);
			Self::do_redeem_dutch_auction(&auction_owner, auction_id, &auction, &bid)
		}

		/// Cancel auction, only auction without any bid can be canceled
		#[pallet::weight(<T as Config<I>>::WeightInfo::cancel_dutch())]
		#[transactional]
		pub fn cancel_dutch(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let auction_owner = ensure_signed(origin)?;
			let auction = DutchAuctions::<T, I>::get(&auction_owner, auction_id)
				.ok_or(Error::<T, I>::AuctionNotFound)?;

			let bid = DutchAuctionBids::<T, I>::get(auction_id);
			ensure!(bid.is_none(), Error::<T, I>::CannotRemoveAuction);
			pallet_nft::Pallet::<T, I>::unreserve(
				auction.class_id,
				auction.token_id,
				auction.quantity,
				&auction_owner,
			)?;
			Self::delete_dutch_auction(&auction_owner, auction_id)?;
			Self::deposit_event(Event::CanceledDutchAuction { auction_id, owner: auction_owner });
			Ok(())
		}

		/// Create an english auction.
		#[pallet::weight(<T as Config<I>>::WeightInfo::create_english())]
		#[transactional]
		pub fn create_english(
			origin: OriginFor<T>,
			#[pallet::compact] class_id: T::ClassId,
			#[pallet::compact] token_id: T::TokenId,
			#[pallet::compact] quantity: T::Quantity,
			#[pallet::compact] init_price: BalanceOf<T, I>,
			#[pallet::compact] min_raise_price: BalanceOf<T, I>,
			#[pallet::compact] deadline: BlockNumberFor<T>,
			open_at: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			pallet_nft::Pallet::<T, I>::ensure_transferable(class_id, token_id, quantity, &who)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(
				deadline >= now.saturating_add(T::MinDeadline::get()),
				Error::<T, I>::InvalidDeadline
			);
			let open_at = open_at.map(|v| v.max(now)).unwrap_or(now);

			let deposit = T::AuctionDeposit::get();
			T::Currency::reserve(&who, deposit)?;
			pallet_nft::Pallet::<T, I>::reserve(class_id, token_id, quantity, &who)?;

			let auction_id = Self::new_auction_id()?;

			let auction = EnglishAuction {
				class_id,
				token_id,
				quantity,
				init_price,
				min_raise_price,
				created_at: now,
				open_at,
				deadline,
				deposit,
			};

			EnglishAuctions::<T, I>::insert(who.clone(), auction_id, auction);

			Self::deposit_event(Event::CreatedEnglishAuction {
				auction_id,
				class_id,
				token_id,
				quantity,
				owner: who,
			});
			Ok(())
		}

		/// Bid english auction
		#[pallet::weight(<T as Config<I>>::WeightInfo::bid_english())]
		#[transactional]
		pub fn bid_english(
			origin: OriginFor<T>,
			auction_owner: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] auction_id: T::AuctionId,
			#[pallet::compact] price: BalanceOf<T, I>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let auction_owner = T::Lookup::lookup(auction_owner)?;
			let auction = EnglishAuctions::<T, I>::get(&auction_owner, auction_id)
				.ok_or(Error::<T, I>::AuctionNotFound)?;
			ensure!(&auction_owner != &who, Error::<T, I>::SelfBid);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(auction.open_at <= now, Error::<T, I>::AuctionNotOpen);
			let maybe_bid = EnglishAuctionBids::<T, I>::get(auction_id);
			match maybe_bid {
				None => {
					ensure!(auction.deadline >= now, Error::<T, I>::AuctionClosed);
					T::Currency::reserve(&who, price)?;
					EnglishAuctionBids::<T, I>::insert(
						auction_id,
						AuctionBid { account: who.clone(), price, bid_at: now },
					);
					Self::deposit_event(Event::BidEnglishAuction {
						auction_id,
						bidder: who,
						owner: auction_owner.clone(),
						price,
					});
				},
				Some(bid) => {
					ensure!(
						auction.deadline >= now ||
							bid.bid_at.saturating_add(T::DelayOfAuction::get()) >= now,
						Error::<T, I>::AuctionClosed
					);
					ensure!(
						price >= bid.price.saturating_add(auction.min_raise_price),
						Error::<T, I>::InvalidBidPrice
					);
					T::Currency::reserve(&who, price)?;
					T::Currency::unreserve(&bid.account, bid.price);
					EnglishAuctionBids::<T, I>::insert(
						auction_id,
						AuctionBid { account: who.clone(), price, bid_at: now },
					);
					Self::deposit_event(Event::BidEnglishAuction {
						auction_id,
						bidder: who,
						owner: auction_owner.clone(),
						price,
					});
				},
			}
			Ok(())
		}

		/// Redeem duction
		#[pallet::weight(<T as Config<I>>::WeightInfo::redeem_english())]
		#[transactional]
		pub fn redeem_english(
			origin: OriginFor<T>,
			auction_owner: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let auction_owner = T::Lookup::lookup(auction_owner)?;
			let auction = EnglishAuctions::<T, I>::get(&auction_owner, auction_id)
				.ok_or(Error::<T, I>::AuctionNotFound)?;
			let bid = EnglishAuctionBids::<T, I>::get(auction_id)
				.ok_or(Error::<T, I>::AuctionBidNotFound)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(
				bid.bid_at.saturating_add(T::DelayOfAuction::get()) < now && auction.deadline < now,
				Error::<T, I>::CannotRedeemNow
			);
			T::Currency::unreserve(&bid.account, bid.price);

			let class_id = auction.class_id;
			let token_id = auction.token_id;
			let quantity = auction.quantity;
			pallet_nft::Pallet::<T, I>::unreserve(class_id, token_id, quantity, &auction_owner)?;
			pallet_nft::Pallet::<T, I>::swap(
				class_id,
				token_id,
				quantity,
				&auction_owner,
				&bid.account,
				bid.price,
				T::AuctionFeeTaxRatio::get(),
				pallet_nft::TransferReason::EnglishAuction,
			)?;

			Self::delete_english_auction(&auction_owner, auction_id)?;
			Self::deposit_event(Event::RedeemedEnglishAuction {
				auction_id,
				bidder: bid.account,
				owner: auction_owner.clone(),
				price: bid.price,
			});
			Ok(())
		}

		/// Cancel auction, only auction without any bid can be canceled
		#[pallet::weight(<T as Config<I>>::WeightInfo::cancel_english())]
		#[transactional]
		pub fn cancel_english(
			origin: OriginFor<T>,
			#[pallet::compact] auction_id: T::AuctionId,
		) -> DispatchResult {
			let auction_owner = ensure_signed(origin)?;
			let auction = EnglishAuctions::<T, I>::get(&auction_owner, auction_id)
				.ok_or(Error::<T, I>::AuctionNotFound)?;
			let bid = EnglishAuctionBids::<T, I>::get(auction_id);
			ensure!(bid.is_none(), Error::<T, I>::CannotRemoveAuction);
			pallet_nft::Pallet::<T, I>::unreserve(
				auction.class_id,
				auction.token_id,
				auction.quantity,
				&auction_owner,
			)?;
			Self::delete_english_auction(&auction_owner, auction_id)?;
			Self::deposit_event(Event::CanceledEnglishAuction { auction_id, owner: auction_owner });
			Ok(())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	fn new_auction_id() -> Result<T::AuctionId, DispatchError> {
		NextAuctionId::<T, I>::try_mutate(|id| -> Result<T::AuctionId, DispatchError> {
			let next_id = *id;
			*id = id.checked_add(&One::one()).ok_or(Error::<T, I>::InvalidNextAuctionId)?;
			Ok(next_id)
		})
	}

	fn get_dutch_price(auction: &DutchAuctionOf<T, I>, now: BlockNumberFor<T>) -> BalanceOf<T, I> {
		let diff_price = auction.max_price.saturating_sub(auction.min_price);
		let lifetime = auction.deadline.saturating_sub(auction.created_at);
		let pasttime = now.saturating_sub(auction.created_at).min(lifetime);
		let rate = Perbill::from_rational(pasttime, lifetime);
		let dec_price = rate * diff_price;
		auction.max_price.saturating_sub(dec_price)
	}

	fn do_redeem_dutch_auction(
		auction_owner: &T::AccountId,
		auction_id: T::AuctionId,
		auction: &DutchAuctionOf<T, I>,
		bid: &AuctionBidOf<T, I>,
	) -> DispatchResult {
		let class_id = auction.class_id;
		let token_id = auction.token_id;
		let quantity = auction.quantity;
		pallet_nft::Pallet::<T, I>::unreserve(class_id, token_id, quantity, auction_owner)?;
		pallet_nft::Pallet::<T, I>::swap(
			class_id,
			token_id,
			quantity,
			auction_owner,
			&bid.account,
			bid.price,
			T::AuctionFeeTaxRatio::get(),
			pallet_nft::TransferReason::DutchAuction,
		)?;
		Self::delete_dutch_auction(&auction_owner, auction_id)?;
		Self::deposit_event(Event::RedeemedDutchAuction {
			auction_id,
			bidder: bid.account.clone(),
			owner: auction_owner.clone(),
			price: bid.price,
		});
		Ok(())
	}

	fn delete_dutch_auction(
		auction_owner: &T::AccountId,
		auction_id: T::AuctionId,
	) -> DispatchResult {
		DutchAuctions::<T, I>::try_mutate_exists(
			auction_owner,
			auction_id,
			|maybe_auction| -> DispatchResult {
				let auction = maybe_auction.as_mut().ok_or(Error::<T, I>::AuctionNotFound)?;
				T::Currency::unreserve(auction_owner, auction.deposit);
				DutchAuctionBids::<T, I>::remove(auction_id);
				*maybe_auction = None;
				Ok(())
			},
		)?;
		Ok(())
	}

	fn delete_english_auction(
		auction_owner: &T::AccountId,
		auction_id: T::AuctionId,
	) -> DispatchResult {
		EnglishAuctions::<T, I>::try_mutate_exists(
			auction_owner,
			auction_id,
			|maybe_auction| -> DispatchResult {
				let auction = maybe_auction.as_mut().ok_or(Error::<T, I>::AuctionNotFound)?;
				T::Currency::unreserve(auction_owner, auction.deposit);
				EnglishAuctionBids::<T, I>::remove(auction_id);
				*maybe_auction = None;
				Ok(())
			},
		)?;
		Ok(())
	}
}
