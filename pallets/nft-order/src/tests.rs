#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok};
use pallet_nft::Error as NFTError;

fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
	}
}

fn token_info(owner: u64, class_id: u32, token_id: u32) -> (u32, u32) {
	let am = NFT::tokens_by_owner(&owner, (class_id, token_id)).unwrap_or_default();
	(am.free, am.reserved)
}

#[test]
fn sell_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		add_class(1);
		add_token(1, 0);

		// should work and reserve balance
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 0, 1, 10, None));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_eq!(token_info(1, 0, 0), (0, 1));

		// should not sell twice
		assert_err!(
			NFTOrder::sell(Origin::signed(1), 0, 0, 1, 10, None),
			NFTError::<Test>::InvalidQuantity
		);

		// should not sell nft which is not found
		assert_err!(
			NFTOrder::sell(Origin::signed(1), 0, 1, 1, 10, None),
			NFTError::<Test>::TokenNotFound,
		);

		// should not sell asset you do not owned
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFT::mint(Origin::signed(1), 2, 0, 1, vec![], None, None));
		assert_err!(
			NFTOrder::sell(Origin::signed(1), 0, 1, 1, 10, None),
			NFTError::<Test>::TokenNotFound
		);

		// should work with deadline
		add_token(1, 0);
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 2, 1, 10, Some(2)));

		// should not sell asset with outdated dealine
		run_to_block(3);
		add_token(1, 0);
		assert_err!(
			NFTOrder::sell(Origin::signed(1), 0, 3, 1, 10, Some(2)),
			Error::<Test>::InvalidDeadline
		);
	});
}

#[test]
fn deal_order_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		add_class(1);
		add_token(1, 0);
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 0, 1, 10, None));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_eq!(Balances::free_balance(&1), 87);
		Balances::make_free_balance_be(&2, 100);
		let total = Balances::total_issuance();
		assert_ok!(NFTOrder::deal_order(Origin::signed(2), 1, 0));
		assert!(NFTOrder::orders(1, 0).is_none());
		assert_eq!(Balances::total_issuance(), total.saturating_sub(1));
		assert_eq!(Balances::free_balance(&1), 106);
		assert_eq!(Balances::free_balance(&2), 90);
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_eq!(Balances::reserved_balance(&2), 0);
		assert_eq!(token_info(2, 0, 0), (1, 0));

		// should fail when token is not selling
		assert_err!(NFTOrder::deal_order(Origin::signed(2), 1, 1), Error::<Test>::OrderNotFound);

		// should fail when dealine is exceed
		add_token(1, 0);
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 1, 1, 10, Some(2)));
		run_to_block(3);
		assert_err!(NFTOrder::deal_order(Origin::signed(2), 1, 1), Error::<Test>::OrderExpired);
	});
}

#[test]
fn deal_order_with_royalty_beneficiary() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&3, 100);
		add_class(1);
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 1, vec![], None, Some(3)));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 0, 1, 10, None));
		assert_eq!(Balances::free_balance(&1), 87);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTOrder::deal_order(Origin::signed(2), 1, 0));
		assert_eq!(Balances::free_balance(&1), 105);
		assert_eq!(Balances::free_balance(&2), 90);
		assert_eq!(Balances::free_balance(&3), 101);
	})
}

#[test]
fn deal_order_with_royalty_beneficiary_no_account() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		add_class(1);
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 1, vec![], None, Some(3)));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 0, 1, 10, None));
		assert_eq!(Balances::free_balance(&1), 87);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTOrder::deal_order(Origin::signed(2), 1, 0));
		assert_eq!(Balances::free_balance(&1), 106);
		assert_eq!(Balances::free_balance(&2), 90);
		assert_eq!(Balances::free_balance(&3), 0);
	})
}

#[test]
fn deal_order_with_insufficient_funds() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		add_class(1);
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 1, vec![], None, Some(3)));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 0, 1, 10, None));
		Balances::make_free_balance_be(&2, 9);
		assert_err!(
			NFTOrder::deal_order(Origin::signed(2), 1, 0),
			Error::<Test>::InsufficientFunds
		);
	})
}

#[test]
fn remove_order_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		add_class(1);
		assert_ok!(NFT::mint(Origin::signed(1), 1, 0, 1, vec![], None, Some(3)));
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 0, 1, 10, None));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_ok!(NFTOrder::remove_order(Origin::signed(1), 0));
		assert!(NFTOrder::orders(1, 0).is_none());
		assert_eq!(Balances::reserved_balance(&1), 3);
		assert_eq!(token_info(1, 0, 0), (1, 0));
	});
}

#[test]
fn buy_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		add_class(1);
		add_token(1, 0);

		Balances::make_free_balance_be(&2, 100);
		assert_eq!(Balances::reserved_balance(&2), 0);
		assert_ok!(NFTOrder::buy(Origin::signed(2), 0, 0, 1, 10, None));
		assert_eq!(Balances::reserved_balance(&2), 10);

		// should not sell nft which is not exists
		assert_err!(
			NFTOrder::sell(Origin::signed(2), 0, 1, 1, 10, None),
			NFTError::<Test>::TokenNotFound,
		);

		// should work with deadline
		add_token(1, 0);
		assert_ok!(NFTOrder::buy(Origin::signed(1), 0, 1, 1, 10, Some(2)));

		run_to_block(3);
		add_token(1, 0);
		assert_err!(
			NFTOrder::buy(Origin::signed(1), 0, 2, 1, 10, Some(2)),
			Error::<Test>::InvalidDeadline
		);
	})
}

#[test]
fn deal_offer_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&2, 100);
		add_class(1);
		add_token(1, 0);

		assert_ok!(NFTOrder::buy(Origin::signed(2), 0, 0, 1, 10, None));
		assert_ok!(NFTOrder::deal_offer(Origin::signed(1), 2, 0));
		assert_eq!(Balances::free_balance(&2), 90);
		assert_eq!(Balances::reserved_balance(&2), 0);
		assert!(NFTOrder::offers(2, 0).is_none());
		assert_eq!(token_info(2, 0, 0), (1, 0));

		// should fail when offer is not found
		assert_err!(NFTOrder::deal_offer(Origin::signed(1), 2, 1), Error::<Test>::OfferNotFound);

		// should fail when dealine is exceed
		add_token(1, 0);
		assert_ok!(NFTOrder::buy(Origin::signed(2), 0, 1, 1, 10, Some(2)));
		run_to_block(3);
		assert_err!(NFTOrder::deal_offer(Origin::signed(1), 2, 1), Error::<Test>::OfferExpired);
	})
}

#[test]
fn remove_offer_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		Balances::make_free_balance_be(&2, 100);
		add_class(1);
		add_token(1, 0);

		assert_ok!(NFTOrder::buy(Origin::signed(2), 0, 0, 1, 10, None));
		assert_eq!(Balances::free_balance(&2), 90);
		assert_eq!(Balances::reserved_balance(&2), 10);
		assert_ok!(NFTOrder::remove_offer(Origin::signed(2), 0));
		assert_eq!(Balances::free_balance(&2), 100);
		assert_eq!(Balances::reserved_balance(&2), 0);
		assert!(NFTOrder::offers(2, 0).is_none());
	});
}
