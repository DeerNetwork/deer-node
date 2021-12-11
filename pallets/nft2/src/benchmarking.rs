#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_token_id_pallet, impl_benchmark_test_suite, whitelist_account,
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
	let class_id = Default::default();
	T::Currency::make_free_balance_be(&caller, DepositBalanceOf::<T, I>::max_value());
	assert!(
		NFT::<T, I>::create_class(SystemOrigin::Signed(caller.clone()).into(), class_id, vec![0, 0, 0], rate(10)).is_ok()
	);
	(class_id, caller)
}

fn mint_token<T: Config<I>, I: 'static>(class_id: T::ClassId, token_Id: u32, quantity: u32) -> (T::TokenId, T::TokenId, T::AccountId) {
	let caller = Class::<T, I>::get(T::ClassId::default()).unwrap().owner;
	if caller != whitelisted_caller() {
		whitelist_account!(caller);
	}
	let class_id = class_id.into();
	let token_id = token_id.into();
    let quantity = quantity.into();
	assert!(NFT::<T, I>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
        caller.clone(),
        class_id,
		token_id,
        vec![0, 0, 0],
        quantity,
		Some(rate(10)),
		None,
	)
	.is_ok());
	(token_id, quantity, caller)
}

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

benchmarks_token_id_pallet! {
	create {
		let caller: T::AccountId = whitelisted_caller();
		let class_id = 1u32.into();
		T::Currency::make_free_balance_be(&caller, DepositBalanceOf::<T, I>::max_value());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, vec![0, 0, 0], rate(10))
	verify {
		assert_last_event::<T, I>(Event::CreatedClass(class_id, caller).into());
	}

	mint {
		let (class_id, caller) = create_class::<T, I>();
		let token_id = 1u32.into();
        let quantity = 1u32.into();
		let beneficiary: T::AccountId = account("beneficiary", 0, SEED);
		whitelist_account!(beneficiary);
	}: _(SystemOrigin::Signed(caller.clone()), caller.clone(), class_id, token_id, vec![0, 0, 0], quantity, Some(rate(10)), Some(beneficiary))
	verify {
		assert_last_event::<T, I>(Event::MintedToken(class_id, token_id, quantity, caller.clone(), caller).into());
	}

	burn {
		let (class_id, caller) = create_class::<T, I>();
		let (token_id, quantity, ..) = mint_token::<T, I>(class_id, 1, 1);
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id)
	verify {
		assert_last_event::<T, I>(Event::BurnedToken(class_id, token_id, quantity, caller).into());
	}


	transfer {
		let (class_id, caller) = create_class::<T, I>();
		let (token_id, quantity, ..) = mint_instance::<T, I>(class_id, 1, 1);
		let target: T::AccountId = account("target", 0, SEED);
		T::Currency::make_free_balance_be(&target, DepositBalanceOf::<T, I>::max_value());
		let target_lookup = T::Lookup::unlookup(target.clone());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, quantity, target_lookup)
	verify {
		assert_last_event::<T, I>(Event::TransferredToken(class_id, token_id, quantity, caller, target).into());
	}
}

impl_benchmark_test_suite!(NFT, crate::mock::new_test_ext(), crate::mock::Test);
