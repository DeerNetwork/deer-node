use super::*;

pub mod v2 {
	use super::*;

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
		assert!(StorageVersion::<T, I>::get() == Releases::V1);
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
		DutchAuctions::<T, I>::translate::<OldDutchAuctionOf<T, I>, _>(|_, p| {
			let new_auction = DutchAuction {
				owner: p.owner,
				class_id: p.class,
				token_id: p.instance,
				quantity: One::one(),
				min_price: p.min_price,
				max_price: p.max_price,
				created_at: p.created_at,
				open_at: p.created_at,
				deadline: p.deadline,
				deposit: p.deposit,
			};
			dutch_auction_count += 1;
			Some(new_auction)
		});

		let mut english_auction_count = 0;
		EnglishAuctions::<T, I>::translate::<OldEnglishAuctionOf<T, I>, _>(|_, p| {
			let new_auction = EnglishAuction {
				owner: p.owner,
				class_id: p.class,
				token_id: p.instance,
				quantity: One::one(),
				init_price: p.init_price,
				min_raise_price: p.min_raise_price,
				created_at: p.created_at,
				deadline: p.deadline,
				open_at: p.created_at,
				deposit: p.deposit,
			};

			english_auction_count += 1;
			Some(new_auction)
		});

		StorageVersion::<T, I>::put(Releases::V2);

		log::info!(
			target: "runtime::nft-auction",
			"Migrate {} duction auctions, {} english auctions",
			dutch_auction_count,
			english_auction_count
		);

		T::DbWeight::get().reads_writes(
			(dutch_auction_count + english_auction_count) as Weight,
			(dutch_auction_count + english_auction_count) as Weight + 1,
		)
	}
	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		log::debug!(
			target: "runtime::nft-auction",
			"migration: nft auction storage version v2 POST migration checks succesful!",
		);
		Ok(())
	}
}
