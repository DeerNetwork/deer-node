#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
};
use frame_support::assert_ok;
use frame_system::RawOrigin as SystemOrigin;
use sp_runtime::{
	traits::{Bounded, One, StaticLookup},
	Perbill,
};
use sp_std::prelude::*;

use crate::Pallet as NFTOrder;
use pallet_nft::{ClassPermission, NextClassId, NextTokenId, Pallet as NFT, Permission};

const SEED: u32 = 0;

fn rate(v: u32) -> Perbill {
	Perbill::from_percent(v)
}

fn create_nft<T: Config<I>, I: 'static>(
	owner: &T::AccountId,
) -> (T::ClassId, T::TokenId, T::Quantity) {
	let quantity = One::one();
	let permission = ClassPermission(
		Permission::Burnable | Permission::Transferable | Permission::DelegateMintable,
	);
	let class_id = NextClassId::<T, I>::get();
	assert_ok!(NFT::<T, I>::create_class(
		SystemOrigin::Signed(owner.clone()).into(),
		vec![0, 0, 0],
		rate(10),
		permission,
	));
	let to: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
	let token_id = NextTokenId::<T, I>::get(&class_id);
	assert_ok!(NFT::<T, I>::mint(
		SystemOrigin::Signed(owner.clone()).into(),
		to,
		class_id,
		quantity,
		vec![0, 0, 0],
		None,
		None
	));
	(class_id, token_id, quantity)
}

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

benchmarks_instance_pallet! {
	sell {
		let owner: T::AccountId = account("owner", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let order_id = NextOrderId::<T, I>::get();
	}: _(SystemOrigin::Signed(owner.clone()), class_id, token_id, quantity, 10u32.into(), Some(3u32.into()))
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CreatedOrder { order_id,  seller: owner }.into());
	}

	deal_order {
		let owner: T::AccountId = account("owner", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let buyer: T::AccountId = account("buyer", 0, SEED);
		whitelist_account!(buyer);
		T::Currency::make_free_balance_be(&buyer, BalanceOf::<T, I>::max_value());
		let order_id = NextOrderId::<T, I>::get();
		assert!(NFTOrder::<T, I>::sell(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, 10u32.into(), Some(3u32.into())).is_ok());
		let order_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
	}: _(SystemOrigin::Signed(buyer.clone()), order_owner, order_id, quantity)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::DealedOrder { order_id, seller: owner, buyer, quantity, fee: 10u32.into() }.into());
	}

	remove_order {
		let owner: T::AccountId = account("owner", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let order_id = NextOrderId::<T, I>::get();
		assert!(NFTOrder::<T, I>::sell(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, 10u32.into(), Some(3u32.into())).is_ok());
	}: _(SystemOrigin::Signed(owner.clone()), order_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::RemovedOrder { order_id, seller: owner }.into());
	}

	buy {
		let owner: T::AccountId = account("owner", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let buyer: T::AccountId = account("buyer", 0, SEED);
		whitelist_account!(buyer);
		T::Currency::make_free_balance_be(&buyer, BalanceOf::<T, I>::max_value());
		let offer_id = NextOfferId::<T, I>::get();
	}: _(SystemOrigin::Signed(buyer.clone()), class_id, token_id, quantity, 10u32.into(), Some(3u32.into()))
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CreatedOffer { offer_id, buyer }.into());
	}

	deal_offer {
		let owner: T::AccountId = account("owner", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let buyer: T::AccountId = account("buyer", 0, SEED);
		whitelist_account!(buyer);
		T::Currency::make_free_balance_be(&buyer, BalanceOf::<T, I>::max_value());
		let offer_id = NextOfferId::<T, I>::get();
		assert!(NFTOrder::<T, I>::buy(SystemOrigin::Signed(buyer.clone()).into(), class_id, token_id, quantity, 10u32.into(), Some(3u32.into())).is_ok());
		let offer_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(buyer.clone());
	}: _(SystemOrigin::Signed(owner.clone()), offer_owner, offer_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::DealedOffer { offer_id, buyer, seller: owner, quantity, fee: 10u32.into() }.into());
	}

	remove_offer {
		let owner: T::AccountId = account("owner", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let buyer: T::AccountId = account("buyer", 0, SEED);
		whitelist_account!(buyer);
		T::Currency::make_free_balance_be(&buyer, BalanceOf::<T, I>::max_value());
		let offer_id = NextOfferId::<T, I>::get();
		assert!(NFTOrder::<T, I>::buy(SystemOrigin::Signed(buyer.clone()).into(), class_id, token_id, quantity, 10u32.into(), Some(3u32.into())).is_ok());
	}: _(SystemOrigin::Signed(buyer.clone()), offer_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::RemovedOffer { offer_id, buyer }.into());
	}
}

impl_benchmark_test_suite!(NFTOrder, crate::mock::new_test_ext(), crate::mock::Test);
