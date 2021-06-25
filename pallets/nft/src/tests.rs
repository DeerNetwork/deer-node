//! Tests for Uniques pallet.

use super::*;
use crate::mock::*;
use sp_std::convert::TryInto;
use frame_support::{assert_ok, traits::Currency};

fn assets() -> Vec<(u64, u32, u32)> {
	let mut r: Vec<_> = Account::<Test>::iter().map(|x| x.0).collect();
	r.sort();
	let mut s: Vec<_> = Asset::<Test>::iter().map(|x| (x.2.owner, x.0, x.1)).collect();
	s.sort();
	assert_eq!(r, s);
	for class in Asset::<Test>::iter()
		.map(|x| x.0)
		.scan(None, |s, item| if s.map_or(false, |last| last == item) {
				*s = Some(item);
				Some(None)
			} else {
				Some(Some(item))
			}
		).filter_map(|item| item)
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
fn mint_minting_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42));
		assert_eq!(assets(), vec![(1, 0, 42)]);
	});
}

#[test]
fn transfer_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42));
		assert_ok!(Uniques::transfer(Origin::signed(1), 0, 42, 2));
		assert_eq!(assets(), vec![(2, 0, 42)]);
	});
}

#[test]
fn set_attribute_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, None, bvec![0], bvec![0]));
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0]));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, None, bvec![0], bvec![0; 10]));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0; 10]));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, None, bvec![1], bvec![0; 10]));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, Some(42), bvec![1], bvec![0; 10]));
		assert_ok!(Uniques::clear_attribute(Origin::signed(1), 0, Some(42), bvec![0]));
		assert_ok!(Uniques::clear_attribute(Origin::signed(1), 0, None, bvec![0]));
	});
}

#[test]
fn burn_works() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42));
		assert_eq!(assets(), vec![(1, 0, 42)]);
		assert_ok!(Uniques::burn(Origin::signed(1), 0, 42));
		assert_eq!(assets(), vec![]);
	});
}
