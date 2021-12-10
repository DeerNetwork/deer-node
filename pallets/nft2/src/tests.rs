//! Tests for NFT pallet.

use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::Currency};

#[test]
fn create_class_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_eq!(Balances::reserved_balance(&1), 5);
		let c = Classes::<Test>::get(0).unwrap();
		assert_eq!(c.owner, 1);
		assert_eq!(c.deposit, 5);
		assert_eq!(c.metadata, vec![0, 0, 0]);
		assert_eq!(c.total_tokens, 0);
		assert_eq!(c.total_issuance, 0);
		assert_eq!(c.royalty_rate, rate(5));
		assert_err!(
			NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)),
			Error::<Test>::ConflictClassId
		);
		assert_err!(
			NFT::create_class(Origin::signed(1), 1, vec![0, 0, 0], rate(21)),
			Error::<Test>::RoyaltyRateTooHigh
		);
	});
}

#[test]
fn create_class_with_limit() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_err!(
			NFT::create_class(
				Origin::signed(1),
				ClassIdIncLimit::get() + 1,
				vec![0, 0, 0],
				rate(5)
			),
			Error::<Test>::InvalidClassId
		);
		assert_ok!(NFT::create_class(Origin::signed(1), 3, vec![0, 0, 0], rate(5)));
		assert_eq!(MaxClassId::<Test>::get(), 3);
		assert_ok!(NFT::create_class(Origin::signed(1), 1, vec![0, 0, 0], rate(5)));
		assert_eq!(MaxClassId::<Test>::get(), 3);
	})
}

#[test]
fn mint_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 42, vec![0, 0, 1], 2, None, None));
		assert_eq!(Balances::reserved_balance(&1), 9);
		let t = Tokens::<Test>::get(0, 42).unwrap();
		assert_eq!(t.metadata, vec![0, 0, 1]);
		assert_eq!(t.deposit, 4);
		assert_eq!(t.quantity, 2);
		assert_eq!(t.royalty_rate, rate(5));
		assert_eq!(t.royalty_beneficiary, 1);
		let c = Classes::<Test>::get(0).unwrap();
		assert_eq!(c.total_tokens, 1);
		assert_eq!(c.total_issuance, 2);
		let tm = TokensByOwner::<Test>::get(1, (0, 42)).unwrap();
		assert_eq!(tm.free, 2);
		assert_eq!(tm.reserved, 0);
		assert_eq!(OwnersByToken::<Test>::get((0, 42), 1), Some(()));
		assert_err!(
			NFT::mint(Origin::signed(1), 1, 0, 42, vec![0, 0, 1], 1, None, None),
			Error::<Test>::ConflictTokenId
		);
		assert_err!(
			NFT::mint(Origin::signed(1), 1, 1, 43, vec![0, 0, 1], 1, None, None),
			Error::<Test>::ClassNotFound
		);
		assert_err!(
			NFT::mint(Origin::signed(1), 1, 0, 43, vec![0, 0, 1], 0, None, None),
			Error::<Test>::InvalidQuantity
		);
		assert_err!(
			NFT::mint(Origin::signed(1), 1, 0, 43, vec![0, 0, 1], 1, Some(rate(21)), None),
			Error::<Test>::RoyaltyRateTooHigh
		);
		assert_err!(
			NFT::mint(Origin::signed(2), 2, 0, 43, vec![0, 0, 1], 2, None, None),
			Error::<Test>::NoPermission
		);

		assert_ok!(NFT::mint(Origin::signed(1), 2, 0, 52, vec![0, 0, 1], 2, None, None));
		let t = Tokens::<Test>::get(0, 52).unwrap();
		assert_eq!(t.royalty_beneficiary, 2);
		let tm = TokensByOwner::<Test>::get(2, (0, 52)).unwrap();
		assert_eq!(tm.free, 2);
		assert_eq!(tm.reserved, 0);
		assert_eq!(OwnersByToken::<Test>::get((0, 52), 2), Some(()));

		assert_ok!(NFT::mint(Origin::signed(1), 2, 0, 53, vec![0, 0, 1], 2, None, Some(1)));
		let t = Tokens::<Test>::get(0, 53).unwrap();
		assert_eq!(t.royalty_beneficiary, 1);
	});
}

#[test]
fn burn_should_works() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 42, vec![0, 0, 1], 2, None, None));
		assert_eq!(Balances::reserved_balance(&1), 9);
		assert_ok!(NFT::burn(Origin::signed(1), 0, 42, 2));
		assert_eq!(Balances::reserved_balance(&1), 5);
		assert_eq!(TokensByOwner::<Test>::get(1, (0, 42)), None);
		assert_eq!(OwnersByToken::<Test>::get((0, 42), 1), None);
		assert_eq!(Classes::<Test>::get(0).unwrap().total_tokens, 0);
		assert_eq!(Classes::<Test>::get(0).unwrap().total_issuance, 0);

		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 43, vec![0, 0, 1], 2, None, None));
		assert_err!(NFT::burn(Origin::signed(1), 0, 43, 0), Error::<Test>::InvalidQuantity);
		assert_err!(NFT::burn(Origin::signed(1), 1, 43, 2), Error::<Test>::ClassNotFound);
		assert_err!(NFT::burn(Origin::signed(1), 0, 44, 2), Error::<Test>::TokenNotFound);
		assert_err!(NFT::burn(Origin::signed(1), 0, 43, 3), Error::<Test>::NumOverflow);
	});
}

#[test]
fn burn_partial_should_works() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 2, 0, 52, vec![0, 0, 1], 2, None, None));
		Balances::make_free_balance_be(&2, 100);
		assert_eq!(Balances::reserved_balance(&1), 9);
		assert_ok!(NFT::burn(Origin::signed(2), 0, 52, 1));
		assert_eq!(Balances::reserved_balance(&1), 9);
		let tm = TokensByOwner::<Test>::get(2, (0, 52)).unwrap();
		assert_eq!(tm.free, 1);
		assert_eq!(tm.reserved, 0);
		assert_eq!(OwnersByToken::<Test>::get((0, 52), 2), Some(()));
		assert_eq!(Classes::<Test>::get(0).unwrap().total_tokens, 1);
		assert_eq!(Classes::<Test>::get(0).unwrap().total_issuance, 1);
	});
}

#[test]
fn update_token_royalty_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 42, vec![0, 0, 1], 2, None, None));
		assert_ok!(NFT::update_token_royalty(Origin::signed(1), 0, 42, rate(6)));
		let t = Tokens::<Test>::get(0, 42).unwrap();
		assert_eq!(t.royalty_rate, rate(6));

		assert_err!(
			NFT::update_token_royalty(Origin::signed(1), 0, 43, rate(6)),
			Error::<Test>::TokenNotFound
		);
		assert_err!(
			NFT::update_token_royalty(Origin::signed(1), 0, 42, rate(21)),
			Error::<Test>::RoyaltyRateTooHigh
		);
		assert_err!(
			NFT::update_token_royalty(Origin::signed(2), 0, 42, rate(5)),
			Error::<Test>::NoPermission
		);
	})
}

#[test]
fn update_token_royalty_beneficiary_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 42, vec![0, 0, 1], 2, None, None));
		assert_ok!(NFT::update_token_royalty_beneficiary(Origin::signed(1), 0, 42, 2));
		let a = Tokens::<Test>::get(0, 42).unwrap();
		assert_eq!(a.royalty_beneficiary, 2);

		assert_err!(
			NFT::update_token_royalty_beneficiary(Origin::signed(1), 0, 43, 2),
			Error::<Test>::TokenNotFound
		);
		assert_err!(
			NFT::update_token_royalty_beneficiary(Origin::signed(1), 0, 42, 2),
			Error::<Test>::NoPermission
		);
	})
}

#[test]
fn transfer_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 42, vec![0, 0, 1], 2, None, None));
		assert_eq!(Balances::reserved_balance(&1), 9);
		assert_ok!(NFT::transfer(Origin::signed(1), 0, 42, 1, 2));

		assert_eq!(OwnersByToken::<Test>::get((0, 42), 1), Some(()));
		assert_eq!(OwnersByToken::<Test>::get((0, 42), 2), Some(()));
		assert_eq!(TokensByOwner::<Test>::get(1, (0, 42)).unwrap().free, 1);
		assert_eq!(TokensByOwner::<Test>::get(2, (0, 42)).unwrap().free, 1);

		assert_ok!(NFT::transfer(Origin::signed(1), 0, 42, 1, 2));
		assert_eq!(OwnersByToken::<Test>::get((0, 42), 1), None);
		assert_eq!(OwnersByToken::<Test>::get((0, 42), 2), Some(()));
		assert_eq!(TokensByOwner::<Test>::get(1, (0, 42)), None);
		assert_eq!(TokensByOwner::<Test>::get(2, (0, 42)).unwrap().free, 2);

		assert_err!(NFT::transfer(Origin::signed(2), 0, 42, 0, 1), Error::<Test>::InvalidQuantity);
		assert_err!(NFT::transfer(Origin::signed(2), 0, 43, 1, 1), Error::<Test>::TokenNotFound);
		assert_err!(NFT::transfer(Origin::signed(2), 0, 42, 3, 1), Error::<Test>::NumOverflow);
	});
}

#[test]
fn reserve_unreserve_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 42, vec![0, 0, 1], 2, None, None));
		assert_ok!(NFT::reserve(0, 42, 1, &1));
		let tm = TokensByOwner::<Test>::get(1, (0, 42)).unwrap();
		assert_eq!(tm.free, 1);
		assert_eq!(tm.reserved, 1);

		assert_ok!(NFT::unreserve(0, 42, 1, &1));
		let tm = TokensByOwner::<Test>::get(1, (0, 42)).unwrap();
		assert_eq!(tm.free, 2);
		assert_eq!(tm.reserved, 0);
	});
}

#[test]
fn swap_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 1000);
		Balances::make_free_balance_be(&2, 1000);
		Balances::make_free_balance_be(&3, 1000);
		assert_ok!(NFT::create_class(Origin::signed(1), 0, vec![0, 0, 0], rate(5)));
		assert_ok!(NFT::mint(Origin::signed(1), 2, 0, 42, vec![0, 0, 1], 2, None, None));
		assert_ok!(NFT::update_token_royalty_beneficiary(Origin::signed(2), 0, 42, 1));

		let free1 = Balances::free_balance(&1);
		let free2 = Balances::free_balance(&2);
		let free3 = Balances::free_balance(&3);
		assert_ok!(NFT::reserve(0, 42, 2, &2));
		assert_ok!(NFT::swap(0, 42, 2, &2, &3, 100, rate(1)));
		assert_eq!(OwnersByToken::<Test>::get((0, 42), 2), None);
		assert_eq!(OwnersByToken::<Test>::get((0, 42), 3), Some(()));
		assert_eq!(TokensByOwner::<Test>::get(2, (0, 42)), None);
		assert_eq!(TokensByOwner::<Test>::get(3, (0, 42)).unwrap().free, 2);
		assert_eq!(Balances::free_balance(&1) - free1, 5);
		assert_eq!(Balances::free_balance(&2) - free2, 94);
		assert_eq!(free3 - Balances::free_balance(&3), 100);
	});
}
