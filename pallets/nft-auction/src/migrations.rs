use super::{StorageVersion as PalletStorageVersion, *};

pub mod v2 {
	use super::*;

	use frame_support::{pallet_prelude::*, weights::Weight};
	use sp_std::collections::btree_map::BTreeMap;

	macro_rules! generate_storage_instance {
		($pallet:ident, $name:ident, $storage_instance:ident) => {
			pub struct $storage_instance<T, I>(core::marker::PhantomData<(T, I)>);
			impl<T: Config<I>, I: 'static> frame_support::traits::StorageInstance
				for $storage_instance<T, I>
			{
				fn pallet_prefix() -> &'static str {
					stringify!($pallet)
				}
				const STORAGE_PREFIX: &'static str = stringify!($name);
			}
		};
	}

	pub type OldDutchAuctionOf<T, I = ()> = OldDutchAuction<
		<T as frame_system::Config>::AccountId,
		ClassIdOf<T, I>,
		TokenIdOf<T, I>,
		BalanceOf<T, I>,
		<T as frame_system::Config>::BlockNumber,
	>;

	pub type OldEnglishAuctionOf<T, I = ()> = OldEnglishAuction<
		<T as frame_system::Config>::AccountId,
		ClassIdOf<T, I>,
		TokenIdOf<T, I>,
		BalanceOf<T, I>,
		<T as frame_system::Config>::BlockNumber,
	>;

	generate_storage_instance!(NFTAuction, Auctions, AuctionsInstance);
	#[allow(type_alias_bounds)]
	pub type Auctions<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		AuctionsInstance<T, I>,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::TokenId,
		T::AuctionId,
		OptionQuery,
	>;

	generate_storage_instance!(NFTAuction, DutchAuctions, DutchAuctionsInstance);
	#[allow(type_alias_bounds)]
	pub type OldDutchAuctions<T: Config<I>, I: 'static = ()> = StorageMap<
		DutchAuctionsInstance<T, I>,
		Blake2_128Concat,
		T::AuctionId,
		OldDutchAuctionOf<T, I>,
		OptionQuery,
	>;

	generate_storage_instance!(NFTAuction, EnglishAuctions, EnglishAuctionsInstance);
	#[allow(type_alias_bounds)]
	pub type OldEnglishAuctions<T: Config<I>, I: 'static = ()> = StorageMap<
		EnglishAuctionsInstance<T, I>,
		Blake2_128Concat,
		T::AuctionId,
		OldEnglishAuctionOf<T, I>,
		OptionQuery,
	>;

	generate_storage_instance!(NFTAuction, CurrentAuctionId, CurrentAuctionIdInstance);
	#[allow(type_alias_bounds)]
	pub type CurrentAuctionId<T: Config<I>, I: 'static = ()> =
		StorageValue<CurrentAuctionIdInstance<T, I>, T::AuctionId, ValueQuery>;

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct OldDutchAuction<AccountId, ClassId, InstanceId, Balance, BlockNumber> {
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
		/// When opening auction
		#[codec(compact)]
		pub open_at: BlockNumber,
		/// The auction should be forced to be ended if current block number higher than this
		/// value.
		#[codec(compact)]
		pub deadline: BlockNumber,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct OldEnglishAuction<AccountId, ClassId, InstanceId, Balance, BlockNumber> {
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
		/// When opening auction
		#[codec(compact)]
		pub open_at: BlockNumber,
		/// The auction should be forced to be ended if current block number higher than this
		/// value.
		#[codec(compact)]
		pub deadline: BlockNumber,
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(PalletStorageVersion::<T, I>::get() == Releases::V1);
		log::debug!(
			target: "runtime::nft-auction",
			"migration: nft auction storage version v2 PRE migration checks succesful!",
		);
		Ok(())
	}

	pub fn migrate<T: Config<I>, I: 'static>() -> Weight {
		log::info!(
			target: "runtime::nft-auction",
			"Migrating nft auction to Releases::V2",
		);

		let mut dutch_auction_count = 0;

		let mut dutch_auction_map: BTreeMap<T::AuctionId, OldDutchAuctionOf<T, I>> =
			BTreeMap::new();
		for (auction_id, old_auction) in OldDutchAuctions::<T, I>::drain() {
			dutch_auction_map.insert(auction_id, old_auction);
			dutch_auction_count += 1;
		}
		for (auction_id, old_auction) in dutch_auction_map.into_iter() {
			let auction_owner = old_auction.owner;
			let new_auction = DutchAuction {
				class_id: old_auction.class,
				token_id: old_auction.instance,
				quantity: One::one(),
				min_price: old_auction.min_price,
				max_price: old_auction.max_price,
				created_at: old_auction.created_at,
				open_at: old_auction.created_at,
				deadline: old_auction.deadline,
				deposit: old_auction.deposit,
			};
			DutchAuctions::<T, I>::insert(auction_owner, auction_id, new_auction);
		}

		let mut english_auction_count = 0;
		let mut english_auction_map: BTreeMap<T::AuctionId, OldEnglishAuctionOf<T, I>> =
			BTreeMap::new();
		for (auction_id, old_auction) in OldEnglishAuctions::<T, I>::drain() {
			english_auction_map.insert(auction_id, old_auction);
			english_auction_count += 1;
		}
		for (auction_id, old_auction) in english_auction_map.into_iter() {
			let auction_owner = old_auction.owner;
			let new_auction = EnglishAuction {
				class_id: old_auction.class,
				token_id: old_auction.instance,
				quantity: One::one(),
				init_price: old_auction.init_price,
				min_raise_price: old_auction.min_raise_price,
				created_at: old_auction.created_at,
				deadline: old_auction.deadline,
				open_at: old_auction.created_at,
				deposit: old_auction.deposit,
			};
			EnglishAuctions::<T, I>::insert(auction_owner, auction_id, new_auction);
		}

		let next_auction_id = CurrentAuctionId::<T, I>::take();
		NextAuctionId::<T, I>::put(next_auction_id);

		PalletStorageVersion::<T, I>::put(Releases::V2);

		log::info!(
			target: "runtime::nft-auction",
			"Migrate {} duction auctions, {} english auctions",
			dutch_auction_count,
			english_auction_count
		);

		T::DbWeight::get().reads_writes(
			(dutch_auction_count + english_auction_count + 1) as Weight,
			(dutch_auction_count + english_auction_count + 2) as Weight,
		)
	}
	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(PalletStorageVersion::<T, I>::get() == Releases::V2);
		for (_, _, auction) in DutchAuctions::<T, I>::iter() {
			assert_eq!(auction.quantity, One::one());
		}
		for (_, _, auction) in EnglishAuctions::<T, I>::iter() {
			assert_eq!(auction.quantity, One::one());
		}
		log::debug!(
			target: "runtime::nft-auction",
			"migration: nft auction storage version v2 POST migration checks succesful!",
		);
		Ok(())
	}
}
