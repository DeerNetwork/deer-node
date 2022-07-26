use super::*;
use super::StorageVersion as PalletStorageVersion;

pub mod v2 {
	use super::*;
	use frame_support::{pallet_prelude::*, parameter_types, storage::migration, weights::Weight};
	use sp_runtime::traits::Zero;
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
		pub const KeyLimit: u32 = 256;
		pub const ValueLimit: u32 = 4096;
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct OldClassDetails<AccountId, DepositBalance> {
		/// The owner of this class.
		pub owner: AccountId,
		/// The total balance deposited for this asset class.
		pub deposit: DepositBalance,
		/// The total number of outstanding instances of this asset class.
		#[codec(compact)]
		pub instances: u32,
		/// Royalty rate
		#[codec(compact)]
		pub royalty_rate: Perbill,
	}

	/// Information concerning the ownership of a single unique asset.
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
	pub struct OldTokenDetails<AccountId, DepositBalance> {
		/// The owner of this asset.
		pub owner: AccountId,
		/// The total balance deposited for this asset class.
		pub deposit: DepositBalance,
		/// Whether the asset can be reserved or not.
		pub reserved: bool,
		/// Set transfer target
		pub ready_transfer: Option<AccountId>,
		/// Royalty rate
		#[codec(compact)]
		pub royalty_rate: Perbill,
		/// Royalty beneficiary
		pub royalty_beneficiary: AccountId,
	}

	generate_storage_instance!(NFT, Class, ClassInstance);
	#[allow(type_alias_bounds)]
	pub type Class<T: Config<I>, I: 'static = ()> = StorageMap<
		ClassInstance<T, I>,
		Blake2_128Concat,
		T::ClassId,
		OldClassDetails<T::AccountId, BalanceOf<T, I>>,
	>;

	generate_storage_instance!(NFT, Asset, AssetInstance);
	#[allow(type_alias_bounds)]
	pub type Asset<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		AssetInstance<T, I>,
		Blake2_128Concat,
		T::ClassId,
		Blake2_128Concat,
		T::TokenId,
		OldTokenDetails<T::AccountId, BalanceOf<T, I>>,
		OptionQuery,
	>;

	generate_storage_instance!(NFT, Attribute, AttributeInstance);
	#[allow(type_alias_bounds)]
	pub type Attribute<T: Config<I>, I: 'static = ()> = StorageNMap<
		AttributeInstance<T, I>,
		(
			NMapKey<Blake2_128Concat, T::ClassId>,
			NMapKey<Blake2_128Concat, Option<T::TokenId>>,
			NMapKey<Blake2_128Concat, BoundedVec<u8, KeyLimit>>,
		),
		(BoundedVec<u8, ValueLimit>, BalanceOf<T, I>),
		OptionQuery,
	>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(PalletStorageVersion::<T, I>::get() == Releases::V1);
		log::debug!(
			target: "runtime::nft",
			"migration: nft storage version v2 PRE migration checks succesful!",
		);
		Ok(())
	}

	pub fn migrate<T: Config<I>, I: 'static>() -> Weight {
		log::info!(
			target: "runtime::nft",
			"Migrating nft to Releases::V2",
		);
		let pallet_name = <Pallet<T, I>>::name().as_bytes();

		let mut class_count: u32 = 0;
		let mut token_count: u32 = 0;
		let mut attribute_count: u32 = 0;

		let permission = ClassPermission(
			Permission::Burnable | Permission::Transferable | Permission::DelegateMintable,
		);
		let mut max_class_id: T::ClassId = Zero::zero();
		for (class_id, p) in Class::<T, I>::drain() {
			let (metadata, count) = attributes_to_metadata::<T, I>(class_id, None);
			attribute_count += count;
			let new_class_details = ClassDetails {
				owner: p.owner,
				deposit: p.deposit,
				permission,
				metadata,
				total_tokens: p.instances.saturated_into(),
				total_issuance: p.instances.saturated_into(),
				royalty_rate: p.royalty_rate,
			};
			Classes::<T, I>::insert(class_id, new_class_details);
			if class_id > max_class_id {
				max_class_id = class_id;
			}
			class_count += 1;
		}

		let mut max_token_id_map: BTreeMap<T::ClassId, T::TokenId> = BTreeMap::new();
		let zero = Zero::zero();

		for (class_id, token_id, p) in Asset::<T, I>::drain() {
			let (metadata, count) = attributes_to_metadata::<T, I>(class_id, Some(token_id));
			attribute_count += count;
			let new_token_details = TokenDetails {
				creator: p.owner.clone(),
				metadata,
				deposit: p.deposit,
				quantity: One::one(),
				consumers: 0,
				royalty_rate: p.royalty_rate,
				royalty_beneficiary: p.royalty_beneficiary,
			};
			let mut token_amount: TokenAmount<T::Quantity> = Default::default();
			if p.reserved {
				token_amount.reserved = One::one();
			} else {
				token_amount.free = One::one();
			}
			let max_token_id = max_token_id_map.get(&class_id).unwrap_or(&zero);
			if token_id > *max_token_id {
				max_token_id_map.insert(class_id, token_id);
			}

			Tokens::<T, I>::insert(class_id, token_id, new_token_details);
			TokensByOwner::<T, I>::insert(p.owner.clone(), (class_id, token_id), token_amount);
			OwnersByToken::<T, I>::insert((class_id, token_id), p.owner, ());
			token_count += 1;
		}

		for (class_id, max_token_id) in max_token_id_map.iter() {
			NextTokenId::<T, I>::insert(*class_id, max_token_id.saturating_add(One::one()));
		}

		migration::remove_storage_prefix(pallet_name, b"AssetTransfer", b"");
		migration::remove_storage_prefix(pallet_name, b"Account", b"");
		migration::remove_storage_prefix(pallet_name, b"Attribute", b"");
		migration::remove_storage_prefix(pallet_name, b"MaxClassId", b"");
		NextClassId::<T, I>::put(max_class_id.saturating_add(One::one()));

		PalletStorageVersion::<T, I>::put(Releases::V2);

		log::info!(
			target: "runtime::nft",
			"Migrate {:?} classes, {:?} tokens",
			class_count,
			token_count,
		);

		T::DbWeight::get().reads_writes(
			(class_count + token_count + attribute_count) as Weight,
			(class_count * 2 + token_count * 3 + 5) as Weight,
		)
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(PalletStorageVersion::<T, I>::get() == Releases::V2);
		log::debug!(
			target: "runtime::nft",
			"migration: nft storage version v2 POST migration checks succesful!",
		);
		for (owner, (class_id, token_id), _) in TokensByOwner::<T, I>::iter() {
			assert!(
				OwnersByToken::<T, I>::get((class_id, token_id), owner.clone()).is_some() &&
					Tokens::<T, I>::get(class_id, token_id).is_some(),
				"invalid token ({:?} {:?})",
				class_id,
				token_id
			);
		}
		assert_eq!(Class::<T, I>::iter().count(), 0);
		assert_eq!(Asset::<T, I>::iter().count(), 0);
		Ok(())
	}

	fn attributes_to_metadata<T: Config<I>, I: 'static>(
		class_id: T::ClassId,
		token_id: Option<T::TokenId>,
	) -> (Vec<u8>, u32) {
		let mut count = 0;
		let mut pairs: Vec<Vec<u8>> = vec![];
		for key in Attribute::<T, I>::iter_key_prefix((class_id, token_id)) {
			if let Some((value, _)) = Attribute::<T, I>::get((class_id, token_id, &key)) {
				pairs.push([b"\"", &key[..], b"\":\"", &value[..], b"\""].concat());
			}
			count += 1;
		}
		let content = pairs.join(&b',');
		([b"{", &content[..], b"}"].concat(), count)
	}
}
