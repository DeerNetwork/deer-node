use super::*;

pub mod v1 {
	use super::*;

	use frame_support::{pallet_prelude::*, weights::Weight};

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
		let mut num_nodes = 0;
		Nodes::<T>::translate::<OldNodeInfo<BlockNumberFor<T>>, _>(|_, p| {
			let node = NodeInfo {
				rid: p.rid,
				used: p.used,
				slash_used: 0,
				power: p.power,
				reported_at: p.reported_at,
			};
			num_nodes += 1;
			Some(node)
		});

		StorageVersion::<T>::put(Releases::V1);

		log::info!(
			target: "runtime::file-storage",
			"Migrate {} nodes",
			num_nodes
		);

		T::DbWeight::get().reads_writes(num_nodes as Weight, (num_nodes + 1) as Weight)
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
