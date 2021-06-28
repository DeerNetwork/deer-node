#![cfg(feature = "runtime-benchmarks")]

use sp_std::prelude::*;
use super::*;
use sp_runtime::traits::Bounded;
use frame_system::RawOrigin as SystemOrigin;
use frame_benchmarking::{
	benchmarks_instance_pallet, account, whitelist_account, impl_benchmark_test_suite
};
use frame_support::traits::Get;

use crate::Pallet as NFTOrder;
use pallet_nft::Pallet as NFT;

const SEED: u32 = 0;


fn create_nft<T: Config<I>, I: 'static>(owner: &T::AccountId, n: u32) -> (T::ClassId, T::InstanceId) {
    let class = n.into();
    let instance = n.into();
	assert!(NFT::<T, I>::create(
		SystemOrigin::Signed(owner.clone()).into(),
		class,
	).is_ok());
	assert!(NFT::<T, I>::mint(
		SystemOrigin::Signed(owner.clone()).into(),
		class,
		instance,
	).is_ok());
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
        let n in 0 .. T::MaxOrders::get() - 1;
		let caller: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let (class, instance) = create_nft::<T, I>(&caller, n);
	}: _(SystemOrigin::Signed(caller.clone()), class, instance, 10u32.into(), Some(3u32.into()))
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Selling(class, instance, caller).into());
	}

	deal {
        let n in 0 .. T::MaxOrders::get() - 1;
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		T::Currency::make_free_balance_be(&owner, BalanceOf::<T, I>::max_value());
		let (class, instance) = create_nft::<T, I>(&owner, n);
		let caller: T::AccountId = account("target", 0, SEED);
        whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
        assert!(NFTOrder::<T, I>::sell(SystemOrigin::Signed(owner.clone()).into(), class, instance, 10u32.into(), Some(3u32.into())).is_ok());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Dealed(class, instance, owner, caller).into());
	}

	remove {
        let n in 0 .. T::MaxOrders::get() - 1;
		let caller: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let (class, instance) = create_nft::<T, I>(&caller, n);
        assert!(NFTOrder::<T, I>::sell(SystemOrigin::Signed(caller.clone()).into(), class, instance, 10u32.into(), Some(3u32.into())).is_ok());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::Removed(class, instance, caller).into());
	}
}

impl_benchmark_test_suite!(NFTOrder, crate::mock::new_test_ext(), crate::mock::Test);
