use super::*;

pub mod v1 {
	use super::*;

	use frame_support::{pallet_prelude::*, weights::Weight};

	macro_rules! generate_storage_instance {
		($pallet:ident, $name:ident, $storage_instance:ident) => {
			pub struct $storage_instance<T>(core::marker::PhantomData<T>);
			impl<T: Config> frame_support::traits::StorageInstance for $storage_instance<T> {
				fn pallet_prefix() -> &'static str {
					stringify!($pallet)
				}
				const STORAGE_PREFIX: &'static str = stringify!($name);
			}
		};
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
	pub struct OldNodeInfo<BlockNumber> {
		/// A increment id of one report
		pub rid: u64,
		/// Effective storage space
		pub used: u64,
		/// Mine power of node, use this to distribute mining rewards
		pub power: u64,
		/// Latest report at
		pub reported_at: BlockNumber,
	}

	/// Information stashing a node
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct OldStashInfo<AccountId, Balance> {
		/// Stasher account
		pub stasher: AccountId,
		/// Stash funds
		pub deposit: Balance,
		/// Node's machine id
		pub machine_id: Option<MachineId>,
	}

	generate_storage_instance!(FileStorage, Stashs, StashsInstance);
	#[allow(type_alias_bounds)]
	pub type Stashs<T: Config> = StorageMap<
		StashsInstance<T>,
		Blake2_128Concat,
		T::AccountId,
		OldStashInfo<T::AccountId, BalanceOf<T>>,
	>;

	generate_storage_instance!(FileStorage, Nodes, NodesInstance);
	#[allow(type_alias_bounds)]
	pub type OldNodes<T: Config> = StorageMap<
		NodesInstance<T>,
		Blake2_128Concat,
		T::AccountId,
		OldNodeInfo<BlockNumberFor<T>>,
	>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T>::get() == Releases::V0);
		log::debug!(
			target: "runtime::file-storage",
			"migration: file storage storage version v2 PRE migration checks succesful!",
		);
		Ok(())
	}

	pub fn migrate<T: Config>() -> Weight {
		let mut stash_count = 0;
		let mut node_count = 0;
		for (controller, stash_info) in Stashs::<T>::drain() {
			if let Some(node_info) = OldNodes::<T>::take(&controller) {
				Nodes::<T>::insert(
					controller,
					NodeInfo {
						stash: stash_info.stasher,
						deposit: stash_info.deposit,
						machine_id: stash_info.machine_id,
						rid: node_info.rid,
						used: node_info.used,
						slash_used: 0,
						reward: Zero::zero(),
						power: node_info.power,
						reported_at: node_info.reported_at,
					},
				);
				node_count += 1;
			} else {
				Nodes::<T>::insert(
					controller,
					NodeInfo {
						stash: stash_info.stasher,
						deposit: stash_info.deposit,
						machine_id: stash_info.machine_id,
						rid: 0,
						used: 0,
						slash_used: 0,
						reward: Zero::zero(),
						power: 0,
						reported_at: Zero::zero(),
					},
				);
			}
			stash_count += 1;
		}
		StorageVersion::<T>::put(Releases::V1);

		log::info!(
			target: "runtime::file-storage",
			"Migrate {} stashs {} nodes",
			stash_count,
			node_count
		);

		T::DbWeight::get()
			.reads_writes((2 * stash_count) as Weight, (node_count + stash_count + 1) as Weight)
	}
	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T>::get() == Releases::V1);
		log::debug!(
			target: "runtime::file-storage",
			"migration: file storage storage version v1 POST migration checks succesful!",
		);
		Ok(())
	}
}
