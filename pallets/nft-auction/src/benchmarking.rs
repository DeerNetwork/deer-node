#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
};
use frame_support::assert_ok;
use frame_system::{Pallet as System, RawOrigin as SystemOrigin};
use sp_runtime::Perbill;
use sp_std::prelude::*;

use crate::Pallet as NFTAuction;
use pallet_nft::Pallet as NFT;

const SEED: u32 = 0;

fn rate(v: u32) -> Perbill {
	Perbill::from_percent(v)
}

fn create_nft<T: Config<I>, I: 'static>(owner: &T::AccountId) -> (T::ClassId, T::InstanceId) {
	let class = Default::default();
	let instance = Default::default();
	assert_ok!(NFT::<T, I>::create(SystemOrigin::Signed(owner.clone()).into(), class, rate(10)));
	assert_ok!(NFT::<T, I>::mint(
		SystemOrigin::Signed(owner.clone()).into(),
		class,
		instance,
		None,
		None
	));
	(class, instance)
}

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn get_dollars<T: Config<I>, I: 'static>(mul: u32) -> BalanceOf<T, I> {
	T::Currency::minimum_balance().saturating_mul(mul.into())
}

benchmarks_instance_pallet! {
	create_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let caller = owner.clone();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CreatedDutchAuction(caller, auction_id).into());
	}

	bid_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_dutch(SystemOrigin::Signed(owner.clone()).into(), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
	}: _(SystemOrigin::Signed(caller.clone()), auction_id, None)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::BidDutchAuction(caller, auction_id).into());
	}

	redeem_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_dutch(SystemOrigin::Signed(owner.clone()).into(), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
		assert_ok!(NFTAuction::<T, I>::bid_dutch(SystemOrigin::Signed(caller.clone()).into(), auction_id, None));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(T::DelayOfAuction::get()).saturating_add(2u32.into()));

	}: _(SystemOrigin::Signed(caller.clone()), auction_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::RedeemedDutchAuction(caller, auction_id).into());
	}

	cancel_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_dutch(SystemOrigin::Signed(owner.clone()).into(), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller = owner.clone();
	}: _(SystemOrigin::Signed(caller.clone()), auction_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CanceledDutchAuction(caller, auction_id).into());
	}


	create_english {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let caller = owner.clone();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
	}: _(SystemOrigin::Signed(caller.clone()), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CreatedEnglishAuction(caller, auction_id).into());
	}

	bid_english {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_english(SystemOrigin::Signed(owner.clone()).into(), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
	}: _(SystemOrigin::Signed(caller.clone()), auction_id, get_dollars::<T, I>(20))
	verify {
		assert_last_event::<T, I>(Event::<T, I>::BidEnglishAuction(caller, auction_id).into());
	}

	redeem_english {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_english(SystemOrigin::Signed(owner.clone()).into(), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
		assert_ok!(NFTAuction::<T, I>::bid_english(SystemOrigin::Signed(caller.clone()).into(), auction_id, get_dollars::<T, I>(20)));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(T::DelayOfAuction::get()).saturating_add(2u32.into()));

	}: _(SystemOrigin::Signed(caller.clone()), auction_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::RedeemedEnglishAuction(caller, auction_id).into());
	}

	cancel_english {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class, instance) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::current_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_english(SystemOrigin::Signed(owner.clone()).into(), class, instance, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller = owner.clone();
	}: _(SystemOrigin::Signed(caller.clone()), auction_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CanceledEnglishAuction(caller, auction_id).into());
	}
}

impl_benchmark_test_suite!(NFTAuction, crate::mock::new_test_ext(), crate::mock::Test);
