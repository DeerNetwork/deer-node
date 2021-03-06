#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
	whitelisted_caller,
};
use frame_support::assert_ok;
use frame_system::RawOrigin as SystemOrigin;
use sp_runtime::{traits::Bounded, Perbill};
use sp_std::prelude::*;

use crate::Pallet as NFT;

const SEED: u32 = 0;

fn rate(v: u32) -> Perbill {
	Perbill::from_percent(v)
}

fn new_class<T: Config<I>, I: 'static>() -> (T::ClassId, T::AccountId) {
	let caller: T::AccountId = whitelisted_caller();
	let permission = ClassPermission(
		Permission::Burnable | Permission::Transferable | Permission::DelegateMintable,
	);
	T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
	assert_ok!(NFT::<T, I>::create_class(
		SystemOrigin::Signed(caller.clone()).into(),
		vec![0, 0, 0],
		rate(10),
		permission
	));
	let class_id = NextClassId::<T, I>::get().saturating_sub(One::one());
	(class_id, caller)
}

fn mint_token<T: Config<I>, I: 'static>(
	class_id: T::ClassId,
	quantity: T::Quantity,
) -> (T::TokenId, T::Quantity, T::AccountId) {
	let caller = Classes::<T, I>::get(T::ClassId::default()).unwrap().owner;
	if caller != whitelisted_caller() {
		whitelist_account!(caller);
	}
	let to: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(caller.clone());
	assert_ok!(NFT::<T, I>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
		to,
		class_id,
		quantity,
		vec![0, 0, 0],
		Some(rate(10)),
		None,
	));
	let token_id = NextTokenId::<T, I>::get(&class_id).saturating_sub(One::one());
	(token_id, quantity, caller)
}

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

benchmarks_instance_pallet! {
	create_class {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let permission = ClassPermission(
			Permission::Burnable | Permission::Transferable | Permission::DelegateMintable,
		);
		let class_id = NextClassId::<T, I>::get();
	}: _(SystemOrigin::Signed(caller.clone()), vec![0, 0, 0], rate(10), permission)
	verify {
		assert_last_event::<T, I>(Event::CreatedClass { class_id, owner: caller }.into());
	}

	mint {
		let (class_id, caller) = new_class::<T, I>();
		let to: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(caller.clone());
		let quantity = 1u32.into();
		let beneficiary: T::AccountId = account("beneficiary", 0, SEED);
		let token_id = NextTokenId::<T, I>::get(&class_id);
		whitelist_account!(beneficiary);
	}: _(SystemOrigin::Signed(caller.clone()), to, class_id, quantity, vec![0, 0, 0], Some(rate(10)), Some(beneficiary))
	verify {
		assert_last_event::<T, I>(Event::MintedToken { class_id, token_id, quantity, owner: caller.clone(), caller }.into());
	}

	delegate_mint {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let (class_id, owner) = new_class::<T, I>();
		let to: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(caller.clone());
		let quantity = 1u32.into();
		let beneficiary: T::AccountId = account("beneficiary", 0, SEED);
		let token_id = NextTokenId::<T, I>::get(&class_id);
		whitelist_account!(beneficiary);
	}: _(SystemOrigin::Signed(caller.clone()), class_id, quantity, vec![0, 0, 0], Some(rate(10)), Some(beneficiary))
	verify {
		assert_last_event::<T, I>(Event::MintedToken { class_id, token_id, quantity, owner: caller.clone(), caller }.into());
	}

	burn {
		let (class_id, caller) = new_class::<T, I>();
		let (token_id, quantity, ..) = mint_token::<T, I>(class_id, 1u32.into());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, quantity)
	verify {
		assert_last_event::<T, I>(Event::BurnedToken{ class_id, token_id, quantity, owner: caller }.into());
	}

	update_token_royalty {
		let (class_id, caller) = new_class::<T, I>();
		let (token_id, quantity, ..) = mint_token::<T, I>(class_id, 1u32.into());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, rate(10))
	verify {
		assert_last_event::<T, I>(Event::UpdatedToken { class_id, token_id }.into());
	}

	update_token_royalty_beneficiary {
		let (class_id, caller) = new_class::<T, I>();
		let (token_id, quantity, ..) = mint_token::<T, I>(class_id, 1u32.into());
		let target: T::AccountId = account("target", 0, SEED);
		whitelist_account!(target);
		let target_lookup = T::Lookup::unlookup(target.clone());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, target_lookup)
	verify {
		assert_last_event::<T, I>(Event::UpdatedToken { class_id, token_id }.into());
	}

	transfer {
		let (class_id, caller) = new_class::<T, I>();
		let (token_id, quantity, ..) = mint_token::<T, I>(class_id, 1u32.into());
		let target: T::AccountId = account("target", 0, SEED);
		whitelist_account!(target);
		let target_lookup = T::Lookup::unlookup(target.clone());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, quantity, target_lookup)
	verify {
		assert_last_event::<T, I>(Event::TransferredToken { class_id, token_id, quantity, from: caller, to: target, reason: TransferReason::Direct, price: Zero::zero() }.into());
	}
}

impl_benchmark_test_suite!(NFT, crate::mock::new_test_ext(), crate::mock::Test);
