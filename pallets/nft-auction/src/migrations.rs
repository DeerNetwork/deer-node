use super::*;

pub mod v1 {
	use super::*;

    pub type OldDutchAuctionOf<T, I = ()> = OldDutchAuction<
        <T as frame_system::Config>::AccountId,
        ClassIdOf<T, I>,
        InstanceIdOf<T, I>,
        BalanceOf<T, I>,
        <T as frame_system::Config>::BlockNumber,
    >;

    pub type OldEnglishAuctionOf<T, I = ()> = EnglishAuction<
        <T as frame_system::Config>::AccountId,
        ClassIdOf<T, I>,
        InstanceIdOf<T, I>,
        BalanceOf<T, I>,
        <T as frame_system::Config>::BlockNumber,
    >;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
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
        /// The auction should be forced to be ended if current block number higher than this value.
        #[codec(compact)]
        pub deadline: BlockNumber,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
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
        /// The auction should be forced to be ended if current block number higher than this value.
        #[codec(compact)]
        pub deadline: BlockNumber,
    }

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T, I>::get() == Releases::V0);
		log!(debug, "migration: nft auction storage version v1 PRE migration checks succesful!");
		Ok(())
	}

	pub fn migrate<T: Config<I>, I: 'static>() -> Weight {
		log!(info, "Migrating nft auction to Releases::V1");

		let mut dutch_auction_count = 0;
		DutchAuctions::<T, I>::translate::<OldDutchAuctionOf<T, I>, _>(
			|_, p| {
				let new_class = DutchAuction {
                    owner: p.owner,
                    class: p.class,
                    instance: p.instance,
                    min_price: p.min_price,
                    max_price: p.max_price,
                    created_at: p.created_at,
                    open_at: p.created_at,
                    deadline: p.deadline,
                    deposit: p.deposit,
				};
				dutch_auction_count += 1;
				Some(new_class)
			},
		);

        let mut english_auction_count = 0;
		EnglishAuctions::<T, I>::translate::<OldEnglishAuctionOf<T, I>, _>(
			|_, p| {
				let new_class = EnglishAuction {

				owner: p.owner,
				class: p.class,
				instance: p.instance,
				init_price: p.init_price,
				min_raise_price: p.min_raise_price,
				created_at: p.created_at,
				deadline: p.deadline,
                open_at: p.created_at,
				deposit: p.deposit,
				};

				english_auction_count += 1;
				Some(new_class)
			},
		);

		StorageVersion::<T, I>::put(Releases::V1);

		log!(info, "Migrate {} duction auctions, {} english auctions", dutch_auction_count, english_auction_count);

		T::DbWeight::get().reads_writes(
			(dutch_auction_count + english_auction_count) as Weight,
			(dutch_auction_count + english_auction_count) as Weight + 1,
		)
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T, I>::get() == Releases::V1);
		for (_, auction) in DutchAuctions::<T, I>::iter() {
			assert!(auction.open_at == auction.created_at);
		}
		for (_, auction) in EnglishAuctions::<T, I>::iter() {
			assert!(auction.open_at == auction.created_at);
		}
		log!(debug, "migration: nft auction storage version v1 POST migration checks succesful!");
		Ok(())
	}
}