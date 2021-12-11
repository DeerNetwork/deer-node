use super::*;

pub mod v2 {
	use super::*;
	use frame_support::{storage::migration, traits::PalletInfoAccess, weights::Weight};

	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T, I>::get() == Releases::V1);
		log!(debug, "migration: nft storage version v2 PRE migration checks succesful!");
		Ok(())
	}

	pub fn migrate<T: Config<I>, I: 'static>() -> Weight {
		log!(info, "Migrating nft to Releases::V2");

		let mut class_count: u32 = 0;
		let mut token_count: u32 = 0;
		let mut attribute_count: u32 = 0;

		for (class_id, p) in Class::<T, I>::drain() {
			let (metadata, count) = attributes_to_metadata::<T, I>(class_id, None);
			attribute_count += count;
			let new_class_details = ClassDetails {
				owner: p.owner,
				deposit: p.deposit,
				metadata,
				total_tokens: p.instances.saturated_into(),
				total_issuance: p.instances.saturated_into(),
				royalty_rate: p.royalty_rate,
			};
			Classes::<T, I>::insert(class_id, new_class_details);
			class_count += 1;
		}

		for (class_id, token_id, p) in Asset::<T, I>::drain() {
			let (metadata, count) = attributes_to_metadata::<T, I>(class_id, Some(token_id));
			attribute_count += count;
			let new_token_details = TokenDetails {
				metadata,
				deposit: p.deposit,
				quantity: One::one(),
				royalty_rate: p.royalty_rate,
				royalty_beneficiary: p.royalty_beneficiary,
			};
			let mut token_amount: TokenAmount<T::TokenId> = Default::default();
			if p.reserved {
				token_amount.reserved = One::one();
			} else {
				token_amount.free = One::one();
			}
			Tokens::<T, I>::insert(class_id, token_id, new_token_details);
			TokensByOwner::<T, I>::insert(p.owner.clone(), (class_id, token_id), token_amount);
			OwnersByToken::<T, I>::insert((class_id, token_id), p.owner, ());
			token_count += 1;
		}

		migration::remove_storage_prefix(<Pallet<T, I>>::name().as_bytes(), b"AssetTransfer", b"");
		migration::remove_storage_prefix(<Pallet<T, I>>::name().as_bytes(), b"Account", b"");
		migration::remove_storage_prefix(<Pallet<T, I>>::name().as_bytes(), b"Attribute", b"");

		StorageVersion::<T, I>::put(Releases::V2);

		T::DbWeight::get().reads_writes(
			(class_count + token_count + attribute_count) as Weight,
			(class_count + token_count * 3 + 4) as Weight,
		)
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config<I>, I: 'static>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T, I>::get() == Releases::V2);

		log!(info, "Attribute.exits()? {:?}", Attribute::exists());
		log!(info, "Class.exits()? {:?}", Class::exists());
		log!(info, "Asset.exits()? {:?}", Asset::exists());

		for (class_id, p) in Classes::<T, I>::iter() {
			log!(info, "Class {:?} {:?}", class_Id, p);
		}

		for (class_id, token_id, p) in Tokens::<T, I>::iter() {
			log!(info, "Token {:?} {:?} {:?}", class_Id, token_id, p);
		}
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
