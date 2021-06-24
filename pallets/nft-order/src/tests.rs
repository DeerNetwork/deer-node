#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};

fn prepare1() {
	Balances::make_free_balance_be(&1, 100);
	assert_ok!(NFT::create(Origin::signed(1), 0, 1));
	assert_ok!(NFT::mint(Origin::signed(1), 0, 42, 1));
}

#[test]
fn sell_should_work() {
	new_test_ext().execute_with(|| {
		prepare1();
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
	});
}

#[test]
fn deal_should_work() {
	new_test_ext().execute_with(|| {
		prepare1();
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTOrder::deal(Origin::signed(2), 0, 42, 2));
	});
}

#[test]
fn update_price_should_work() {
	new_test_ext().execute_with(|| {
		prepare1();
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		assert_ok!(NFTOrder::update_price(Origin::signed(1), 0, 42, 20));
	});
}

#[test]
fn remove_should_work() {
	new_test_ext().execute_with(|| {
		prepare1();
		assert_ok!(NFTOrder::sell(Origin::signed(1), 0, 42, 10, None));
		assert_ok!(NFTOrder::remove(Origin::signed(1), 0, 42));
	});
}
