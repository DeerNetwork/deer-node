#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
	whitelisted_caller,
};
use frame_support::{traits::Get, BoundedVec};
use frame_system::RawOrigin as SystemOrigin;
use sp_runtime::{traits::Bounded, Perbill};
use sp_std::{convert::TryInto, prelude::*};

use crate::Pallet as NFT;

const SEED: u32 = 0;

fn rate(v: u32) -> Perbill {
	Perbill::from_percent(v)
}

fn create_class<T: Config<I>, I: 'static>() -> (T::ClassId, T::AccountId) {
	let caller: T::AccountId = whitelisted_caller();
	let class = Default::default();
	T::Currency::make_free_balance_be(&caller, DepositBalanceOf::<T, I>::max_value());
	assert!(
		NFT::<T, I>::create(SystemOrigin::Signed(caller.clone()).into(), class, rate(10)).is_ok()
	);
	(class, caller)
}

fn mint_instance<T: Config<I>, I: 'static>(instance: u32) -> (T::InstanceId, T::AccountId) {
	let caller = Class::<T, I>::get(T::ClassId::default()).unwrap().owner;
	if caller != whitelisted_caller() {
		whitelist_account!(caller);
	}
	let instance = instance.into();
	assert!(NFT::<T, I>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
		Default::default(),
		instance,
		Some(rate(10)),
		None,
	)
	.is_ok());
	(instance, caller)
}

fn add_instance_attribute<T: Config<I>, I: 'static>(
	instance: T::InstanceId,
) -> (BoundedVec<u8, T::KeyLimit>, T::AccountId) {
	let caller = Class::<T, I>::get(T::ClassId::default()).unwrap().owner;
	if caller != whitelisted_caller() {
		whitelist_account!(caller);
	}
	let key: BoundedVec<_, _> = vec![0; T::KeyLimit::get() as usize].try_into().unwrap();
	assert!(NFT::<T, I>::set_attribute(
		SystemOrigin::Signed(caller.clone()).into(),
		Default::default(),
		Some(instance),
		key.clone(),
		vec![0; T::ValueLimit::get() as usize].try_into().unwrap(),
	)
	.is_ok());
	(key, caller)
}

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

benchmarks_instance_pallet! {
	create {
		let caller: T::AccountId = whitelisted_caller();
		let class = 1u32.into();
		T::Currency::make_free_balance_be(&caller, DepositBalanceOf::<T, I>::max_value());
	}: _(SystemOrigin::Signed(caller.clone()), class, rate(10))
	verify {
		assert_last_event::<T, I>(Event::Created(class, caller).into());
	}

	mint {
		let (class, caller) = create_class::<T, I>();
		let instance = Default::default();
		let beneficiary: T::AccountId = account("beneficiary", 0, SEED);
		whitelist_account!(beneficiary);
	}: _(SystemOrigin::Signed(caller.clone()), class, instance, Some(rate(10)), Some(beneficiary))
	verify {
		assert_last_event::<T, I>(Event::Issued(class, instance, caller).into());
	}

	burn {
		let (class, caller) = create_class::<T, I>();
		let (instance, ..) = mint_instance::<T, I>(0);
	}: _(SystemOrigin::Signed(caller.clone()), class, instance)
	verify {
		assert_last_event::<T, I>(Event::Burned(class, instance, caller).into());
	}

	ready_transfer {
		let (class, caller) = create_class::<T, I>();
		let (instance, ..) = mint_instance::<T, I>(0);
		let target: T::AccountId = account("target", 0, SEED);
		T::Currency::make_free_balance_be(&target, DepositBalanceOf::<T, I>::max_value());
		let target_lookup = T::Lookup::unlookup(target.clone());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance, target_lookup)
	verify {
		assert_last_event::<T, I>(Event::ReadyTransfer(class, instance, caller, target).into());
	}

	cancel_transfer {
		let (class, caller) = create_class::<T, I>();
		let (instance, ..) = mint_instance::<T, I>(0);
		let target: T::AccountId = account("target", 0, SEED);
		T::Currency::make_free_balance_be(&target, DepositBalanceOf::<T, I>::max_value());
		let target_lookup = T::Lookup::unlookup(target.clone());
		assert!(NFT::<T, I>::ready_transfer(SystemOrigin::Signed(caller.clone()).into(), class, instance, target_lookup).is_ok());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance)
	verify {
		assert_last_event::<T, I>(Event::CancelTransfer(class, instance, caller).into());
	}

	accept_transfer {
		let (class, caller) = create_class::<T, I>();
		let (instance, ..) = mint_instance::<T, I>(0);
		let target: T::AccountId = account("target", 0, SEED);
		whitelist_account!(target);
		T::Currency::make_free_balance_be(&target, DepositBalanceOf::<T, I>::max_value());
		let target_lookup = T::Lookup::unlookup(target.clone());
		assert!(NFT::<T, I>::ready_transfer(SystemOrigin::Signed(caller.clone()).into(), class, instance, target_lookup).is_ok());
	}: _(SystemOrigin::Signed(target.clone()), class, instance)
	verify {
		assert_last_event::<T, I>(Event::Transferred(class, instance, caller, target).into());
	}

	set_attribute {
		let key: BoundedVec<_, _> = vec![0u8; T::KeyLimit::get() as usize].try_into().unwrap();
		let value: BoundedVec<_, _> = vec![0u8; T::ValueLimit::get() as usize].try_into().unwrap();
		let (class, caller) = create_class::<T, I>();
		let (instance, ..) = mint_instance::<T, I>(0);
	}: _(SystemOrigin::Signed(caller), class, Some(instance), key.clone(), value.clone())
	verify {
		assert_last_event::<T, I>(Event::AttributeSet(class, Some(instance), key, value).into());
	}

	clear_attribute {
		let (class, caller) = create_class::<T, I>();
		let (instance, ..) = mint_instance::<T, I>(0);
		let (key, ..) = add_instance_attribute::<T, I>(instance);
	}: _(SystemOrigin::Signed(caller), class, Some(instance), key.clone())
	verify {
		assert_last_event::<T, I>(Event::AttributeCleared(class, Some(instance), key).into());
	}
}

impl_benchmark_test_suite!(NFT, crate::mock::new_test_ext(), crate::mock::Test);
