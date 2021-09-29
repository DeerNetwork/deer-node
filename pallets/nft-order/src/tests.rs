#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok};

fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
	}
}

#[test]
fn sell_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));

		// should work and reserve balance
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_eq!(NFT::info(&0, &42), Some((1, true)));

		// should not sell twice
		assert_err!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None), Error::<Test>::InvalidNFT);

		// should not sell asset which is not found
		assert_err!(NFTOrder::sell(Origin::signed(1), 0, 1, 10, None), Error::<Test>::InvalidNFT);

		// should not sell asset you do not owned
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFT::mint(Origin::signed(1), 0, 43, None, None));
		assert_ok!(NFT::ready_transfer(Origin::signed(1), 0, 43, 2));
		assert_ok!(NFT::accept_transfer(Origin::signed(2), 0, 43));
		assert_err!(NFTOrder::sell(Origin::signed(1), 0, 43, 10, None), Error::<Test>::InvalidNFT);

		// should work with deadline
		assert_ok!(NFT::mint(Origin::signed(1), 0, 44, None, None));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 44, 10, Some(2)));

		// should not sell asset with outdated dealine
		run_to_block(3);
		assert_ok!(NFT::mint(Origin::signed(1), 0, 45, None, None));
		assert_err!(
			NFTOrder::sell(Origin::signed(1), 0, 45, 10, Some(2)),
			Error::<Test>::InvalidDeadline
		);
	});
}

#[test]
fn deal_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_eq!(Balances::free_balance(&1), 87);
		Balances::make_free_balance_be(&2, 100);
		let total = Balances::total_issuance();
		assert_ok!(NFTOrder::deal(Origin::signed(2), 0, 42));
		assert_eq!(Balances::total_issuance(), total.saturating_sub(1));
		assert_eq!(Balances::free_balance(&1), 107);
		assert_eq!(Balances::free_balance(&2), 89);
		assert_eq!(Balances::reserved_balance(&1), 2);
		assert_eq!(Balances::reserved_balance(&2), 1);
		assert_eq!(NFT::info(&0, &42), Some((2, false)));

		// should fail when asset is not sell
		assert_ok!(NFT::mint(Origin::signed(1), 0, 43, None, None));
		assert_err!(NFTOrder::deal(Origin::signed(2), 0, 42), Error::<Test>::OrderNotFound);

		// should fail when dealine is exceed
		assert_ok!(NFT::mint(Origin::signed(1), 0, 44, None, None));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 44, 10, Some(2)));
		run_to_block(3);
		assert_err!(NFTOrder::deal(Origin::signed(2), 0, 44), Error::<Test>::OrderExpired);
	});
}

#[test]
fn deal_with_royalty_beneficiary() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&3, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, Some(3)));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		assert_eq!(Balances::free_balance(&1), 87);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTOrder::deal(Origin::signed(2), 0, 42));
		assert_eq!(Balances::free_balance(&1), 106);
		assert_eq!(Balances::free_balance(&2), 89);
		assert_eq!(Balances::free_balance(&3), 101);
	})
}

#[test]
fn deal_with_royalty_beneficiary_no_account() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, Some(3)));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		assert_eq!(Balances::free_balance(&1), 87);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTOrder::deal(Origin::signed(2), 0, 42));
		assert_eq!(Balances::free_balance(&1), 107);
		assert_eq!(Balances::free_balance(&2), 89);
		assert_eq!(Balances::free_balance(&3), 0);
	})
}

#[test]
fn deal_with_insufficient_funds() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, Some(3)));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		Balances::make_free_balance_be(&2, 9);
		assert_err!(NFTOrder::deal(Origin::signed(2), 0, 42), Error::<Test>::InsufficientFunds);
	})
}

#[test]
fn remove_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_ok!(NFTOrder::remove(Origin::signed(1), 0, 42));
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_eq!(NFT::info(&0, &42), Some((1, false)));
	});
}
