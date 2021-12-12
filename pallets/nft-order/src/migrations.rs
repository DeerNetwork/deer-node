use super::*;

pub mod v1 {
	use super::*;

	use frame_support::traits::Get;

	pub type OldOrderDetailsOf<T, I = ()> = OldOrderDetails<
		<T as frame_system::Config>::AccountId,
		BalanceOf<T, I>,
		<T as frame_system::Config>::BlockNumber,
	>;

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct OldOrderDetails<AccountId, Balance, BlockNumber> {
		/// Who create the order.
		pub owner: AccountId,
		/// Price of this order.
		pub price: Balance,
		/// The balances to create an order
		pub deposit: Balance,
		/// This order will be invalidated after `deadline` block number.
		pub deadline: Option<BlockNumber>,
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T, I>::get() == Releases::V0);
		log::debug!(
			target: "runtime::nft-order",
			"migration: nft order storage version v1 PRE migration checks succesful!",
		);
		Ok(())
	}

	pub fn migrate<T: Config<I>, I: 'static>() -> Weight {
		log::info!(
			target: "runtime::nft-order",
			"Migrating nft order to Releases::V1",
		);

		let mut order_count = 0;
		Orders::<T, I>::translate::<OldOrderDetailsOf<T, I>, _>(|_, _, p| {
			let new_order = OrderDetails {
				owner: p.owner,
				quantity: One::one(),
				price: p.price,
				deposit: p.deposit,
				deadline: p.deadline,
			};
			order_count += 1;
			Some(new_order)
		});

		StorageVersion::<T, I>::put(Releases::V1);

		log::info!(
			target: "runtime::nft-order",
			"Migrate {} orders",
			order_count,
		);

		T::DbWeight::get().reads_writes(order_count as Weight, (order_count + 1) as Weight)
	}
	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		log::debug!(
			target: "runtime::nft-order",
			"migration: nft order storage version v1 POST migration checks succesful!",
		);
		Ok(())
	}
}
