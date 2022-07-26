use super::*;
use super::StorageVersion as PalletStorageVersion;

pub mod v1 {
	use super::*;

	use frame_support::{pallet_prelude::*, storage::migration};

	type RoundIndex = u32;

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

	/// Record node's effictive storage size and power
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
	pub struct OldNodeStats {
		/// Node's power
		pub power: u64,
		/// Eeffictive storage size
		pub used: u64,
	}

	/// Record network's effictive storage size and power
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
	pub struct OldSummaryStats {
		/// Node's storage power
		pub power: u128,
		/// Eeffictive storage size
		pub used: u128,
	}

	/// Information round rewards
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
	pub struct OldRewardInfo<Balance> {
		/// Reward for node power
		pub mine_reward: Balance,
		/// Reward for node store file
		pub store_reward: Balance,
		/// How many mine reward that already assigned to the node
		pub paid_mine_reward: Balance,
		/// How many store reward that already assigned to the node
		pub paid_store_reward: Balance,
	}

	/// Derive from StoreFile, Record the replicas and expire time
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct OldFileOrder<AccountId, Balance, BlockNumber> {
		/// The cost of storing for a period of time
		pub fee: Balance,
		/// Store file size
		pub file_size: u64,
		/// When the order need to close or renew
		pub expire_at: BlockNumber,
		/// Nodes store the file
		pub replicas: Vec<AccountId>,
	}

	/// File that users submit to the network for storage
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct OldStoreFile<Balance, BlockNumber> {
		/// Funds gathered in this file
		pub reserved: Balance,
		/// Basic cost of sumit to network
		pub base_fee: Balance,
		// Store file size
		pub file_size: u64,
		// When added file
		pub add_at: BlockNumber,
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

	generate_storage_instance!(FileStorage, CurrentRound, CurrentRoundInstance);
	#[allow(type_alias_bounds)]
	pub type CurrentRound<T: Config> =
		StorageValue<CurrentRoundInstance<T>, RoundIndex, ValueQuery>;

	generate_storage_instance!(FileStorage, NextRoundAt, NextRoundAtInstance);
	#[allow(type_alias_bounds)]
	pub type NextRoundAt<T: Config> =
		StorageValue<NextRoundAtInstance<T>, BlockNumberFor<T>, ValueQuery>;

	generate_storage_instance!(FileStorage, RoundsReport, RoundsReportInstance);
	#[allow(type_alias_bounds)]
	pub type RoundsReport<T: Config> = StorageDoubleMap<
		RoundsReportInstance<T>,
		Twox64Concat,
		RoundIndex,
		Blake2_128Concat,
		T::AccountId,
		OldNodeStats,
	>;

	generate_storage_instance!(FileStorage, RoundsReward, RoundsRewardInstance);
	#[allow(type_alias_bounds)]
	pub type RoundsReward<T: Config> = StorageMap<
		RoundsRewardInstance<T>,
		Twox64Concat,
		RoundIndex,
		OldRewardInfo<BalanceOf<T>>,
		ValueQuery,
	>;

	generate_storage_instance!(FileStorage, RoundsSummary, RoundsSummaryInstance);
	#[allow(type_alias_bounds)]
	pub type RoundsSummary<T: Config> =
		StorageMap<RoundsSummaryInstance<T>, Twox64Concat, RoundIndex, OldSummaryStats, ValueQuery>;

	generate_storage_instance!(FileStorage, StoreFiles, StoreFilesInstance);
	#[allow(type_alias_bounds)]
	pub type StoreFiles<T: Config> = StorageMap<
		StoreFilesInstance<T>,
		Twox64Concat,
		FileId,
		OldStoreFile<BalanceOf<T>, BlockNumberFor<T>>,
	>;

	generate_storage_instance!(FileStorage, FileOrders, FileOrdersInstance);
	#[allow(type_alias_bounds)]
	pub type FileOrders<T: Config> = StorageMap<
		FileOrdersInstance<T>,
		Twox64Concat,
		FileId,
		OldFileOrder<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
	>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		assert!(PalletStorageVersion::<T>::get() == Releases::V0);
		assert!(CurrentRound::<T>::get() > 2);
		log::debug!(
			target: "runtime::file-storage",
			"migration: file storage storage version v2 PRE migration checks succesful!",
		);
		Ok(())
	}

	pub fn migrate<T: Config>() -> Weight {
		let pallet_name = <Pallet<T>>::name().as_bytes();

		let mut stash_count = 0u32;
		let mut node_count = 0;
		let mut reported_node_count = 0u32;
		let mut store_file_count = 0u32;
		let mut file_order_count = 0u32;

		let current_round = CurrentRound::<T>::take();
		let next_round_at = NextRoundAt::<T>::take();
		let prev_round = current_round.saturating_add(One::one());
		let duration = T::SessionDuration::get();
		let begin_at = next_round_at.saturating_sub(duration.saturating_sub(One::one()));
		let prev_begin_at = begin_at.saturating_sub(duration);
		Session::<T>::set(SessionState {
			current: current_round,
			prev_begin_at,
			begin_at,
			end_at: next_round_at,
		});
		for (controller, stash_info) in Stashs::<T>::drain() {
			if let Some(node_info) = OldNodes::<T>::take(&controller) {
				let prev_reported_at = match RoundsReport::<T>::take(prev_round, &controller) {
					Some(_) => prev_begin_at,
					None => Zero::zero(),
				};
				if node_info.reported_at >= begin_at {
					reported_node_count += 1;
				}
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
						prev_reported_at,
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
						prev_reported_at: Zero::zero(),
					},
				);
			}
			stash_count += 1;
		}
		for i in 0u32..3 {
			let round = current_round.saturating_sub(i.saturated_into());
			let summary = RoundsSummary::<T>::take(round);
			let reward = RoundsReward::<T>::take(round);
			let count = if i == 0 { reported_node_count } else { 0 };
			Summarys::<T>::insert(
				round,
				SummaryInfo {
					count,
					power: summary.power,
					used: summary.used,
					mine_reward: reward.mine_reward,
					store_reward: reward.store_reward,
					paid_mine_reward: reward.paid_mine_reward,
					paid_store_reward: reward.paid_store_reward,
				},
			);
		}

		for (cid, store_file) in StoreFiles::<T>::drain() {
			if let Some(file_order) = FileOrders::<T>::take(&cid) {
				Files::<T>::insert(
					cid.clone(),
					FileInfo {
						reserved: store_file.reserved,
						base_fee: store_file.base_fee,
						file_size: file_order.file_size,
						add_at: store_file.add_at,
						fee: file_order.fee,
						liquidate_at: file_order.expire_at,
						replicas: file_order.replicas,
					},
				);
				file_order_count += 1;
			} else {
				Files::<T>::insert(
					cid.clone(),
					FileInfo {
						reserved: store_file.reserved,
						base_fee: store_file.base_fee,
						file_size: store_file.file_size,
						add_at: store_file.add_at,
						fee: Zero::zero(),
						liquidate_at: Zero::zero(),
						replicas: vec![],
					},
				);
			}
			store_file_count += 1;
		}

		migration::remove_storage_prefix(pallet_name, b"RoundsReport", b"");
		PalletStorageVersion::<T>::put(Releases::V1);

		log::info!(
			target: "runtime::file-storage",
			"Migrate {} stashs {} nodes {} store_files {} file_orders",
			stash_count,
			node_count,
			store_file_count,
			file_order_count,
		);

		T::DbWeight::get().reads_writes(
			(2 * stash_count + 2 * store_file_count + node_count + 8) as Weight,
			(node_count * 4 + stash_count + store_file_count + file_order_count + 13) as Weight,
		)
	}
	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
		assert!(PalletStorageVersion::<T>::get() == Releases::V1);
		log::debug!(
			target: "runtime::file-storage",
			"migration: file storage storage version v1 POST migration checks succesful!",
		);
		Ok(())
	}
}
