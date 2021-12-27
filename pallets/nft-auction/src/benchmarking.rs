#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
};
use frame_support::assert_ok;
use frame_system::{Pallet as System, RawOrigin as SystemOrigin};
use sp_runtime::{
	traits::{One, Saturating, StaticLookup},
	Perbill,
};
use sp_std::prelude::*;

use crate::Pallet as NFTAuction;
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

fn get_dollars<T: Config<I>, I: 'static>(mul: u32) -> BalanceOf<T, I> {
	T::Currency::minimum_balance().saturating_mul(mul.into())
}

benchmarks_instance_pallet! {
	create_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let caller = owner.clone();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire, None)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CreatedDutchAuction(class_id, token_id, quantity, caller, auction_id).into());
	}

	bid_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		let auction_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_dutch(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire, None));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
	}: _(SystemOrigin::Signed(caller.clone()), auction_owner, auction_id, None)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::BidDutchAuction(caller, auction_id).into());
	}

	redeem_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		let auction_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_dutch(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire, None));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
		assert_ok!(NFTAuction::<T, I>::bid_dutch(SystemOrigin::Signed(caller.clone()).into(), auction_owner.clone(), auction_id, None));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(T::DelayOfAuction::get()).saturating_add(2u32.into()));

	}: _(SystemOrigin::Signed(caller.clone()), auction_owner, auction_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::RedeemedDutchAuction(caller, auction_id).into());
	}

	cancel_dutch {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		let auction_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_dutch(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(80), expire, None));

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
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let caller = owner.clone();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
	}: _(SystemOrigin::Signed(caller.clone()), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire, None)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CreatedEnglishAuction(class_id, token_id, quantity, caller, auction_id).into());
	}

	bid_english {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		let auction_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_english(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire, None));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
	}: _(SystemOrigin::Signed(caller.clone()), auction_owner, auction_id, get_dollars::<T, I>(20))
	verify {
		assert_last_event::<T, I>(Event::<T, I>::BidEnglishAuction(caller, auction_id).into());
	}

	redeem_english {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		let auction_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_english(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire, None));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller: T::AccountId = account("bid", 0, SEED);
		whitelist_account!(caller);
		T::Currency::make_free_balance_be(&caller, value);
		assert_ok!(NFTAuction::<T, I>::bid_english(SystemOrigin::Signed(caller.clone()).into(), auction_owner.clone(), auction_id, get_dollars::<T, I>(20)));

		System::<T>::set_block_number(expire.saturating_add(T::DelayOfAuction::get()).saturating_add(2u32.into()));

	}: _(SystemOrigin::Signed(caller.clone()), auction_owner, auction_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::RedeemedEnglishAuction(caller, auction_id).into());
	}

	cancel_english {
		let owner: T::AccountId = account("anonymous", 0, SEED);
		let auction_owner: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(owner.clone());
		whitelist_account!(owner);
		let value = get_dollars::<T, I>(1_000_000);
		T::Currency::make_free_balance_be(&owner, value);
		let (class_id, token_id, quantity) = create_nft::<T, I>(&owner);
		let auction_id = NFTAuction::<T, I>::next_auction_id();
		let expire = T::MinDeadline::get().saturating_mul(2u32.into());
		assert_ok!(NFTAuction::<T, I>::create_english(SystemOrigin::Signed(owner.clone()).into(), class_id, token_id, quantity, get_dollars::<T, I>(20), get_dollars::<T, I>(1), expire, None));

		System::<T>::set_block_number(T::MinDeadline::get().saturating_add(1u32.into()));

		let caller = owner.clone();
	}: _(SystemOrigin::Signed(caller.clone()), auction_id)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::CanceledEnglishAuction(caller, auction_id).into());
	}
}

impl_benchmark_test_suite!(NFTAuction, crate::mock::new_test_ext(), crate::mock::Test);
