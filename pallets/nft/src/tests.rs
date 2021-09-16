//! Tests for NFT pallet.

use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::Currency};
use sp_std::convert::TryInto;

fn assets() -> Vec<(u64, u32, u32)> {
	let mut r: Vec<_> = Account::<Test>::iter().map(|x| x.0).collect();
	r.sort();
	let mut s: Vec<_> = Asset::<Test>::iter().map(|x| (x.2.owner, x.0, x.1)).collect();
	s.sort();
	assert_eq!(r, s);
	for class in Asset::<Test>::iter()
		.map(|x| x.0)
		.scan(None, |s, item| {
			if s.map_or(false, |last| last == item) {
				*s = Some(item);
				Some(None)
			} else {
				Some(Some(item))
			}
		})
		.filter_map(|item| item)
	{
		let details = Class::<Test>::get(class).unwrap();
		let instances = Asset::<Test>::iter_prefix(class).count() as u32;
		assert_eq!(details.instances, instances);
	}
	r
}

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn attributes(class: u32) -> Vec<(Option<u32>, Vec<u8>, Vec<u8>)> {
	let mut s: Vec<_> = Attribute::<Test>::iter_prefix((class,))
		.map(|(k, v)| (k.0, k.1.into(), v.0.into()))
		.collect();
	s.sort();
	s
}

#[test]
fn basic_setup_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(assets(), vec![]);
	});
}

#[test]
fn create_class_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(5)));
		assert_eq!(Balances::reserved_balance(&1), 2);
		let c = Class::<Test>::get(0).unwrap();
		assert_eq!(c.instances, 0);
		assert_eq!(c.deposit, 2);
		assert_eq!(c.owner, 1);
		assert_eq!(c.royalty_rate, rate(5));
		assert_err!(NFT::create(Origin::signed(1), 0, rate(5)), Error::<Test>::AlreadyExists);
	});
}

#[test]
fn create_class_with_limit() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_err!(
			NFT::create(Origin::signed(1), ClassIdIncLimit::get() + 1, rate(5)),
			Error::<Test>::ClassIdTooLarge
		);
		assert_ok!(NFT::create(Origin::signed(1), 3, rate(5)));
		assert_eq!(MaxClassId::<Test>::get(), 3);
		assert_ok!(NFT::create(Origin::signed(1), 1, rate(5)));
		assert_eq!(MaxClassId::<Test>::get(), 3);
	})
}

#[test]
fn mint_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
		assert_eq!(Balances::reserved_balance(&1), 3);
		let a = Asset::<Test>::get(0, 42).unwrap();
		assert_eq!(a.owner, 1);
		assert_eq!(a.deposit, 1);
		assert_eq!(a.reserved, false);
		assert_eq!(a.royalty_beneficiary, 1);
		assert_eq!(a.royalty_rate, rate(5));
		let c = Class::<Test>::get(0).unwrap();
		assert_eq!(c.instances, 1);
		assert_eq!(c.deposit, 2);
		assert_eq!(assets(), vec![(1, 0, 42)]);
		assert_err!(NFT::mint(Origin::signed(1), 0, 42, None, None), Error::<Test>::AlreadyExists);
		assert_err!(
			NFT::mint(Origin::signed(2), 0, 43, None, None),
			Error::<Test>::WrongClassOwner
		);
	});
}

#[test]
fn transfer_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_ok!(NFT::ready_transfer(Origin::signed(1), 0, 42, 2));
		assert_ok!(NFT::accept_transfer(Origin::signed(2), 0, 42));
		assert_eq!(Balances::reserved_balance(&1), 2);
		assert_eq!(Balances::reserved_balance(&2), 1);
		assert_eq!(assets(), vec![(2, 0, 42)]);
	});
}

#[test]
fn attribute_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(5))); // reserve 2
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, None, bvec![0], bvec![0])); // reserve (1 + 1) * 1 + 1
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None)); // reserve 1
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0]));
		assert_eq!(Balances::reserved_balance(&1), 9);
		assert_eq!(
			attributes(0),
			vec![(None, bvec![0], bvec![0]), (Some(42), bvec![0], bvec![0]),]
		);
		assert_eq!(Class::<Test>::get(0).unwrap().deposit, 5);
		assert_eq!(Asset::<Test>::get(0, 42).unwrap().deposit, 4);

		// update attribute
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, None, bvec![0], bvec![0; 2]));
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0; 2]));
		assert_eq!(
			attributes(0),
			vec![(None, bvec![0], bvec![0; 2]), (Some(42), bvec![0], bvec![0; 2]),]
		);
		assert_eq!(Balances::reserved_balance(&1), 11);

		// multiple attirbutes
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, None, bvec![1], bvec![0]));
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, Some(42), bvec![1], bvec![0]));
		assert_eq!(Balances::reserved_balance(&1), 17);
		assert_eq!(Class::<Test>::get(0).unwrap().deposit, 9);
		assert_eq!(Asset::<Test>::get(0, 42).unwrap().deposit, 8);

		// clear attributes
		assert_ok!(NFT::clear_attribute(Origin::signed(1), 0, None, bvec![0]));
		assert_ok!(NFT::clear_attribute(Origin::signed(1), 0, Some(42), bvec![0]));
		assert_eq!(Class::<Test>::get(0).unwrap().deposit, 5);
		assert_eq!(Asset::<Test>::get(0, 42).unwrap().deposit, 4);
		assert_eq!(Balances::reserved_balance(&1), 9);
	});
}

#[test]
fn burn_works() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_eq!(assets(), vec![(1, 0, 42)]);
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0]));
		assert_ok!(NFT::burn(Origin::signed(1), 0, 42));
		assert_eq!(Balances::reserved_balance(&1), 2);
		assert_eq!(Class::<Test>::get(0).unwrap().instances, 0);
		assert_eq!(assets(), vec![]);
		assert_eq!(Attribute::<Test>::iter_prefix((0, Some(42),)).fold(0, |acc, _| acc + 1), 0);
	});
}

#[test]
fn reserve_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
		assert_ok!(NFT::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0]));
		assert_ok!(NFT::reserve(&0, &42, &1));
		assert_err!(
			NFT::ready_transfer(Origin::signed(1), 0, 42, 2),
			Error::<Test>::AlreadyReserved
		);
		assert_err!(
			NFT::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0]),
			Error::<Test>::AlreadyReserved
		);
		assert_err!(
			NFT::clear_attribute(Origin::signed(1), 0, Some(42), bvec![0]),
			Error::<Test>::AlreadyReserved
		);
		assert_err!(NFT::burn(Origin::signed(1), 0, 42), Error::<Test>::AlreadyReserved);
		assert_ok!(NFT::unreserve(&0, &42));
		assert_ok!(NFT::clear_attribute(Origin::signed(1), 0, Some(42), bvec![0]));
	});
}
