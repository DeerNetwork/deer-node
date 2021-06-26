//! Tests for Uniques pallet.

use super::*;
use crate::mock::*;
use sp_std::convert::TryInto;
use frame_support::{assert_ok, assert_noop, traits::Currency};

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
fn create_class_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_eq!(Balances::reserved_balance(&1), 2);
        let c = Class::<Test>::get(0).unwrap();
		assert_eq!(c.instances, 0);
		assert_eq!(c.deposit, 2);
		assert_eq!(c.owner, 1);
		assert_noop!(Uniques::create(Origin::signed(1), 0), Error::<Test>::AlreadyExists);
    });
}

#[test]
fn mint_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42));
		assert_eq!(Balances::reserved_balance(&1), 3);
        let a = Asset::<Test>::get(0, 42).unwrap();
		assert_eq!(a.owner, 1);
		assert_eq!(a.deposit, 1);
        let c = Class::<Test>::get(0).unwrap();
		assert_eq!(c.instances, 1);
		assert_eq!(c.deposit, 2);
		assert_eq!(assets(), vec![(1, 0, 42)]);
		assert_noop!(Uniques::mint(Origin::signed(1), 0, 42), Error::<Test>::AlreadyExists);
		assert_noop!(Uniques::mint(Origin::signed(2), 0, 43), Error::<Test>::WrongClassOwner);
	});
}

#[test]
fn transfer_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42));
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_ok!(Uniques::transfer(Origin::signed(1), 0, 42, 2));
		assert_eq!(Balances::reserved_balance(&1), 2);
		assert_eq!(Balances::reserved_balance(&2), 1);
		assert_eq!(assets(), vec![(2, 0, 42)]);
	});
}

#[test]
fn attribute_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0)); // reserve 2
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, None, bvec![0], bvec![0])); // reserve (1 + 1) * 1 + 1
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42)); // reserve 1
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0]));
		assert_eq!(Balances::reserved_balance(&1), 9);
		assert_eq!(attributes(0), vec![
			(None, bvec![0], bvec![0]),
			(Some(42), bvec![0], bvec![0]),
		]);
		assert_eq!(Class::<Test>::get(0).unwrap().deposit, 5);
		assert_eq!(Asset::<Test>::get(0, 42).unwrap().deposit, 4);

        // update attribute
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, None, bvec![0], bvec![0; 2]));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0; 2]));
		assert_eq!(attributes(0), vec![
			(None, bvec![0], bvec![0; 2]),
			(Some(42), bvec![0], bvec![0; 2]),
		]);
		assert_eq!(Balances::reserved_balance(&1), 11);

        // multiple attirbutes
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, None, bvec![1], bvec![0]));
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, Some(42), bvec![1], bvec![0]));
		assert_eq!(Balances::reserved_balance(&1), 17);
		assert_eq!(Class::<Test>::get(0).unwrap().deposit, 9);
		assert_eq!(Asset::<Test>::get(0, 42).unwrap().deposit, 8);

        // clear attributes
		assert_ok!(Uniques::clear_attribute(Origin::signed(1), 0, None, bvec![0]));
		assert_ok!(Uniques::clear_attribute(Origin::signed(1), 0, Some(42), bvec![0]));
		assert_eq!(Class::<Test>::get(0).unwrap().deposit, 5);
		assert_eq!(Asset::<Test>::get(0, 42).unwrap().deposit, 4);
		assert_eq!(Balances::reserved_balance(&1), 9);
	});
}

#[test]
fn burn_works() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(Uniques::create(Origin::signed(1), 0));
		assert_ok!(Uniques::mint(Origin::signed(1), 0, 42));
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_eq!(assets(), vec![(1, 0, 42)]);
		assert_ok!(Uniques::set_attribute(Origin::signed(1), 0, Some(42), bvec![0], bvec![0]));
		assert_ok!(Uniques::burn(Origin::signed(1), 0, 42));
		assert_eq!(Balances::reserved_balance(&1), 2);
		assert_eq!(Class::<Test>::get(0).unwrap().instances, 0);
		assert_eq!(assets(), vec![]);
        assert_eq!(Attribute::<Test>::iter_prefix((0, Some(42),)).fold(0, |acc, _| acc + 1), 0);
	});
}
