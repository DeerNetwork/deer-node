#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
};
use frame_system::RawOrigin as SystemOrigin;
use sp_runtime::{traits::Bounded, Perbill};
use sp_std::prelude::*;

use crate::Pallet as NFTOrder;
use pallet_nft::Pallet as NFT;

const SEED: u32 = 0;

fn rate(v: u32) -> Perbill {
	Perbill::from_percent(v)
}

fn create_nft<T: Config<I>, I: 'static>(owner: &T::AccountId) -> (T::ClassId, T::InstanceId) {
	let class = Default::default();
	let instance = Default::default();
	assert!(
		NFT::<T, I>::create(SystemOrigin::Signed(owner.clone()).into(), class, rate(10)).is_ok()
	);
	assert!(NFT::<T, I>::mint(
		SystemOrigin::Signed(owner.clone()).into(),
		class,
		instance,
		None,
		None
	)
	.is_ok());
	(class, instance)
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
		let caller: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let (class, instance) = create_nft::<T, I>(&caller);
	}: _(SystemOrigin::Signed(caller.clone()), class, instance, 10u32.into(), Some(3u32.into()))
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Selling(class, instance, caller).into());
	}

	deal {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class, instance) = create_nft::<T, I>(&owner);
		let caller: T::AccountId = account("target", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		assert!(NFTOrder::<T, I>::sell(SystemOrigin::Signed(owner.clone()).into(), class, instance, 10u32.into(), Some(3u32.into())).is_ok());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Dealed(class, instance, owner, caller).into());
	}

	remove {
		let caller: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let (class, instance) = create_nft::<T, I>(&caller);
		assert!(NFTOrder::<T, I>::sell(SystemOrigin::Signed(caller.clone()).into(), class, instance, 10u32.into(), Some(3u32.into())).is_ok());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Removed(class, instance, caller).into());
	}
}

impl_benchmark_test_suite!(NFTOrder, crate::mock::new_test_ext(), crate::mock::Test);
