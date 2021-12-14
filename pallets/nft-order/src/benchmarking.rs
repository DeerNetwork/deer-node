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
) -> (T::ClassId, T::TokenId, T::TokenId) {
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
	sell_order {
		let caller: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&caller);
		let order_id = NextOrderId::<T, I>::get();
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, quantity, 10u32.into(), Some(3u32.into()))
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Selling(order_id, class_id, token_id, quantity, caller).into());
	}

	deal_order {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let caller: T::AccountId = account("target", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let order_id = NextOrderId::<T, I>::get();
		assert!(NFTOrder::<T, I>::sell_order(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, 10u32.into(), Some(3u32.into())).is_ok());
		let order_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
	}: _(SystemOrigin::Signed(caller.clone()), order_owner, order_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Dealed(order_id, class_id, token_id, quantity, owner, caller).into());
	}

	remove_order {
		let caller: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let (class_id, token_id, quantity) = create_nft::<T, I>(&caller);
		let order_id = NextOrderId::<T, I>::get();
		assert!(NFTOrder::<T, I>::sell_order(SystemOrigin::Signed(caller.clone()).into(), class_id, token_id, quantity, 10u32.into(), Some(3u32.into())).is_ok());
	}: _(SystemOrigin::Signed(caller.clone()), order_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Removed(order_id, class_id, token_id, quantity, caller).into());
	}
}

impl_benchmark_test_suite!(NFTOrder, crate::mock::new_test_ext(), crate::mock::Test);
