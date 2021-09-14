use super::*;

pub mod v1 {
	use super::*;

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
	pub struct OldClassDetails<AccountId, DepositBalance> {
		/// The owner of this class.
		pub owner: AccountId,
		/// The total balance deposited for this asset class.
		pub deposit: DepositBalance,
		/// The total number of outstanding instances of this asset class.
		pub instances: u32,
	}

	/// Information concerning the ownership of a single unique asset.
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
	pub struct OldInstanceDetails<AccountId, DepositBalance> {
		/// The owner of this asset.
		pub owner: AccountId,
		/// The total balance deposited for this asset class.
		pub deposit: DepositBalance,
		/// Whether the asset can be reserved or not.
		pub reserved: bool,
		/// Set transfer target
		pub ready_transfer: Option<AccountId>,
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T, I>::get() == Releases::V0);
		log!(debug, "migration: nft storage version v1 PRE migration checks succesful!");
		Ok(())
	}

	pub fn migrate<T: Config<I>, I: 'static>() -> Weight {
		log!(info, "Migrating nft to Releases::V1");

		let mut class_count = 0;
		Class::<T, I>::translate::<OldClassDetails<T::AccountId, DepositBalanceOf<T, I>>, _>(
			|_, p| {
				let new_class = ClassDetails {
					owner: p.owner,
					deposit: p.deposit,
					instances: p.instances,
					royalty_rate: Default::default(),
				};
				class_count += 1;
				Some(new_class)
			},
		);

		let mut asset_count = 0;
		Asset::<T, I>::translate::<OldInstanceDetails<T::AccountId, DepositBalanceOf<T, I>>, _>(
			|_, _, p| {
				let new_asset = InstanceDetails {
					owner: p.owner,
					deposit: p.deposit,
					reserved: p.reserved,
					ready_transfer: p.ready_transfer,
					royalty_rate: Default::default(),
					royalty_beneficiary: Default::default(),
				};
				asset_count += 1;
				Some(new_asset)
			},
		);

		StorageVersion::<T, I>::put(Releases::V1);

		log!(info, "Migrate {} classes, {} tokens", class_count, asset_count);

		T::DbWeight::get().reads_writes(
			(class_count + asset_count) as Weight,
			(class_count + asset_count) as Weight + 1,
		)
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T, I>::get() == Releases::V1);
		for (_, class) in Class::<T, I>::iter() {
			assert!(class.royalty_rate.is_zero());
		}
		for (_, _, token) in Asset::<T, I>::iter() {
			assert!(token.royalty_rate.is_zero());
		}
		log!(debug, "migration: nft storage version v1 POST migration checks succesful!");
		Ok(())
	}
}
