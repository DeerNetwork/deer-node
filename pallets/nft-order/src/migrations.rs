use super::{StorageVersion as PalletStorageVersion, *};

pub mod v1 {
	use super::*;

	use frame_support::{pallet_prelude::*, parameter_types, weights::Weight};
	use sp_runtime::traits::{One, Saturating, Zero};
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

	parameter_types! {
		pub const MaxOrders: u32 = 50;
	}

	pub type OldOrderDetailsOf<T, I = ()> = OldOrderDetails<
		<T as frame_system::Config>::AccountId,
		BalanceOf<T, I>,
		<T as frame_system::Config>::BlockNumber,
	>;

	generate_storage_instance!(NFTOrder, Orders, OrdersInstance);
	#[allow(type_alias_bounds)]
	pub type OldOrders<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		OrdersInstance<T, I>,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::TokenId,
		OldOrderDetails<T::AccountId, BalanceOf<T, I>, T::BlockNumber>,
		OptionQuery,
	>;

	generate_storage_instance!(NFTOrder, AccountOrders, AccountOrdersInstance);
	#[allow(type_alias_bounds)]
	pub type AccountOrders<T: Config<I>, I: 'static = ()> = StorageMap<
		AccountOrdersInstance<T, I>,
		Twox64Concat,
		T::AccountId,
		BoundedVec<(ClassIdOf<T, I>, TokenIdOf<T, I>), MaxOrders>,
		ValueQuery,
	>;

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
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
		assert!(PalletStorageVersion::<T, I>::get() == Releases::V0);
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
		let mut next_order_id: T::OrderId = Zero::zero();
		let mut order_map: BTreeMap<T::OrderId, (T::ClassId, T::TokenId, OldOrderDetailsOf<T, I>)> =
			BTreeMap::new();
		for (class_id, token_id, old_order) in OldOrders::<T, I>::drain() {
			order_map.insert(next_order_id, (class_id, token_id, old_order));
			next_order_id = next_order_id.saturating_add(One::one());
			order_count += 1;
		}

		for (order_id, (class_id, token_id, old_order)) in order_map.into_iter() {
			let new_order = OrderDetails {
				class_id,
				token_id,
				quantity: One::one(),
				total_quantity: One::one(),
				price: old_order.price,
				deposit: old_order.deposit,
				deadline: old_order.deadline,
			};
			Orders::<T, I>::insert(old_order.owner, order_id, new_order);
		}

		let mut account_order_count = 0;
		for _ in AccountOrders::<T, I>::drain() {
			account_order_count += 1;
		}

		NextOrderId::<T, I>::put(next_order_id);

		PalletStorageVersion::<T, I>::put(Releases::V1);

		log::info!(
			target: "runtime::nft-order",
			"Migrate {} orders",
			order_count,
		);

		T::DbWeight::get().reads_writes(
			(order_count + account_order_count) as Weight,
			(order_count + account_order_count + 2) as Weight,
		)
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(PalletStorageVersion::<T, I>::get() == Releases::V1);
		Ok(())
	}
}
