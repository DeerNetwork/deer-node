#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok};

fn create_dutch_auction() -> u32 {
	Balances::make_free_balance_be(&1, 100);
	assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
	assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
	let auction_id: u32 = CurrentAuctionId::<Test>::get();
	assert_ok!(NFTAuction::create_dutch(Origin::signed(1), 0, 42, 20, 80, 1200));
	return auction_id
}

fn create_english_auction() -> u32 {
	Balances::make_free_balance_be(&1, 100);
	assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
	assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));
	let auction_id: u32 = CurrentAuctionId::<Test>::get();
	assert_ok!(NFTAuction::create_english(Origin::signed(1), 0, 42, 20, 1, 1200));
	return auction_id
}

#[test]
fn create_dutch_auction_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));

		// should work and reserve balance
		assert_eq!(Balances::reserved_balance(&1), 3);
		let auction_id: u32 = CurrentAuctionId::<Test>::get();
		assert_ok!(NFTAuction::create_dutch(Origin::signed(1), 0, 42, 20, 80, 1200));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_eq!(NFT::info(&0, &42), Some((1, true)));
		assert_eq!(Auctions::<Test>::get(0, 42).unwrap(), auction_id);

		// Failed when nft not found
		assert_err!(
			NFTAuction::create_dutch(Origin::signed(1), 1, 42, 20, 80, 1200),
			Error::<Test>::InvalidNFT
		);

		// Failed when deadline lt T::MinDeadline
		assert_ok!(NFT::mint(Origin::signed(1), 0, 43, None, None));
		assert_err!(
			NFTAuction::create_dutch(Origin::signed(1), 0, 43, 20, 80, 100),
			Error::<Test>::InvalidDeadline
		);

		// Failed when price is invalid
		assert_err!(
			NFTAuction::create_dutch(Origin::signed(1), 0, 43, 80, 80, 1200),
			Error::<Test>::InvalidPrice
		);
	});
}

#[test]
fn bid_dutch_auction_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, None));
		let bid = DutchAuctionBids::<Test>::get(auction_id);
		assert_eq!(bid, Some(AuctionBid { account: 2, bid_at: 601, price: 50 }));
		assert_eq!(Balances::reserved_balance(&2), 50);
	});
}

#[test]
fn bid_dutch_auction_again_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, None));
		assert_eq!(Balances::reserved_balance(&2), 50);

		Balances::make_free_balance_be(&3, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(3), auction_id, Some(60)));
		assert_eq!(Balances::reserved_balance(&2), 0);
	});
}

#[test]
fn bid_dutch_auction_with_max_price_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, Some(80)));
		assert_eq!(NFT::info(&0, &42), Some((2, false)));
		assert_eq!(Balances::free_balance(&2), 19);
	});
}

#[test]
fn bid_dutch_auction_again_with_max_price_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, None));

		Balances::make_free_balance_be(&3, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(3), auction_id, Some(80)));
		assert_eq!(NFT::info(&0, &42), Some((3, false)));
	});
}

#[test]
fn bid_dutch_auction_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(2), auction_id + 1, None),
			Error::<Test>::AuctionNotFound
		);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(1), auction_id, None),
			Error::<Test>::SelfBid
		);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(2), auction_id, Some(10)),
			Error::<Test>::InvalidBidPrice
		);

		Balances::make_free_balance_be(&3, 10);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(3), auction_id, Some(80)),
			Error::<Test>::InsufficientFunds
		);

		run_to_block(1201);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(2), auction_id, None),
			Error::<Test>::AuctionClosed
		);
	});
}

#[test]
fn bid_dutch_auction_again_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, None));

		Balances::make_free_balance_be(&3, 100);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(3), auction_id, None),
			Error::<Test>::MissDutchBidPrice
		);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(3), auction_id, Some(10)),
			Error::<Test>::InvalidBidPrice
		);

		Balances::make_free_balance_be(&4, 10);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(4), auction_id, Some(80)),
			Error::<Test>::InsufficientFunds
		);

		run_to_block(662);
		assert_err!(
			NFTAuction::bid_dutch(Origin::signed(3), auction_id, Some(60)),
			Error::<Test>::AuctionClosed
		);
	});
}

#[test]
fn redeem_dutch_auction_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, None));
		run_to_block(662);
		assert_ok!(NFTAuction::redeem_dutch(Origin::signed(2), auction_id));
		assert_eq!(Balances::reserved_balance(&2), 1);
		assert_eq!(NFT::info(&0, &42), Some((2, false)));
		assert_eq!(Balances::free_balance(&2), 49);
	});
}

#[test]
fn redeem_dutch_auction_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		assert_err!(
			NFTAuction::redeem_dutch(Origin::signed(2), auction_id + 1),
			Error::<Test>::AuctionNotFound
		);
		assert_err!(
			NFTAuction::redeem_dutch(Origin::signed(2), auction_id),
			Error::<Test>::AuctionBidNotFound
		);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, None));

		run_to_block(632);
		assert_err!(
			NFTAuction::redeem_dutch(Origin::signed(2), auction_id),
			Error::<Test>::CannotRedeemNow
		);

		run_to_block(662);
		Balances::make_free_balance_be(&3, 100);
		assert_err!(
			NFTAuction::redeem_dutch(Origin::signed(3), auction_id),
			Error::<Test>::NotBidAccount
		);
	});
}

#[test]
fn cancel_dutch_auction_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		assert_ok!(NFTAuction::cancel_dutch(Origin::signed(1), auction_id));
	});
}

#[test]
fn cancel_dutch_auction_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_dutch_auction();
		run_to_block(601);

		assert_err!(
			NFTAuction::redeem_dutch(Origin::signed(1), auction_id + 1),
			Error::<Test>::AuctionNotFound
		);
		Balances::make_free_balance_be(&2, 100);
		assert_err!(
			NFTAuction::cancel_dutch(Origin::signed(2), auction_id),
			Error::<Test>::NotOwnerAccount
		);

		assert_ok!(NFTAuction::bid_dutch(Origin::signed(2), auction_id, None));
		assert_err!(
			NFTAuction::cancel_dutch(Origin::signed(1), auction_id),
			Error::<Test>::CannotRemoveAuction
		);
	});
}

#[test]
fn create_english_auction_should_work() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 100);
		assert_ok!(NFT::create(Origin::signed(1), 0, rate(10)));
		assert_ok!(NFT::mint(Origin::signed(1), 0, 42, None, None));

		// should work and reserve balance
		assert_eq!(Balances::reserved_balance(&1), 3);
		let auction_id: u32 = CurrentAuctionId::<Test>::get();
		assert_ok!(NFTAuction::create_english(Origin::signed(1), 0, 42, 20, 1, 1200));
		assert_eq!(Balances::reserved_balance(&1), 13);
		assert_eq!(NFT::info(&0, &42), Some((1, true)));
		assert_eq!(Auctions::<Test>::get(0, 42).unwrap(), auction_id);

		// Failed when nft not found
		assert_err!(
			NFTAuction::create_english(Origin::signed(1), 1, 42, 20, 1, 1200),
			Error::<Test>::InvalidNFT
		);

		// Failed when deadline lt T::MinDeadline
		assert_ok!(NFT::mint(Origin::signed(1), 0, 43, None, None));
		assert_err!(
			NFTAuction::create_english(Origin::signed(1), 0, 43, 20, 1, 100),
			Error::<Test>::InvalidDeadline
		);
	});
}

#[test]
fn bid_english_auction_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		run_to_block(601);
		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_english(Origin::signed(2), auction_id, 20));
		let bid = EnglishAuctionBids::<Test>::get(auction_id);
		assert_eq!(bid, Some(AuctionBid { account: 2, bid_at: 601, price: 20 }));
		assert_eq!(Balances::reserved_balance(&2), 20);
	});
}

#[test]
fn bid_english_auction_again_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_english(Origin::signed(2), auction_id, 20));
		assert_eq!(Balances::reserved_balance(&2), 20);

		Balances::make_free_balance_be(&3, 100);
		assert_ok!(NFTAuction::bid_english(Origin::signed(3), auction_id, 21));
		assert_eq!(Balances::reserved_balance(&2), 0);
	});
}

#[test]
fn bid_english_auction_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_err!(
			NFTAuction::bid_english(Origin::signed(2), auction_id + 1, 20),
			Error::<Test>::AuctionNotFound
		);
		assert_err!(
			NFTAuction::bid_english(Origin::signed(1), auction_id, 20),
			Error::<Test>::SelfBid
		);

		run_to_block(1201);
		assert_err!(
			NFTAuction::bid_english(Origin::signed(2), auction_id, 20),
			Error::<Test>::AuctionClosed
		);
	});
}

#[test]
fn bid_english_auction_again_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_english(Origin::signed(2), auction_id, 20));

		Balances::make_free_balance_be(&3, 100);
		assert_err!(
			NFTAuction::bid_english(Origin::signed(3), auction_id, 20),
			Error::<Test>::InvalidBidPrice
		);

		run_to_block(1201);
		assert_err!(
			NFTAuction::bid_english(Origin::signed(3), auction_id, 21),
			Error::<Test>::AuctionClosed
		);
	});
}

#[test]
fn redeem_english_auction_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		run_to_block(601);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_english(Origin::signed(2), auction_id, 20));

		run_to_block(1201);
		assert_ok!(NFTAuction::redeem_english(Origin::signed(2), auction_id));
		assert_eq!(Balances::reserved_balance(&2), 1);
		assert_eq!(NFT::info(&0, &42), Some((2, false)));
		assert_eq!(Balances::free_balance(&2), 79);
	});
}

#[test]
fn redeem_english_auction_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		run_to_block(601);

		assert_err!(
			NFTAuction::redeem_english(Origin::signed(2), auction_id + 1),
			Error::<Test>::AuctionNotFound
		);
		assert_err!(
			NFTAuction::redeem_english(Origin::signed(2), auction_id),
			Error::<Test>::AuctionBidNotFound
		);

		Balances::make_free_balance_be(&2, 100);
		assert_ok!(NFTAuction::bid_english(Origin::signed(2), auction_id, 20));

		run_to_block(632);
		assert_err!(
			NFTAuction::redeem_english(Origin::signed(2), auction_id),
			Error::<Test>::CannotRedeemNow
		);

		run_to_block(1201);
		Balances::make_free_balance_be(&3, 100);
		assert_err!(
			NFTAuction::redeem_english(Origin::signed(3), auction_id),
			Error::<Test>::NotBidAccount
		);
	});
}

#[test]
fn cancel_english_auction_should_work() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		assert_ok!(NFTAuction::cancel_english(Origin::signed(1), auction_id));
	});
}

#[test]
fn cancel_english_auction_should_fail() {
	new_test_ext().execute_with(|| {
		let auction_id = create_english_auction();
		run_to_block(601);

		assert_err!(
			NFTAuction::redeem_english(Origin::signed(1), auction_id + 1),
			Error::<Test>::AuctionNotFound
		);
		Balances::make_free_balance_be(&2, 100);
		assert_err!(
			NFTAuction::cancel_english(Origin::signed(2), auction_id),
			Error::<Test>::NotOwnerAccount
		);

		assert_ok!(NFTAuction::bid_english(Origin::signed(2), auction_id, 20));
		assert_err!(
			NFTAuction::cancel_english(Origin::signed(1), auction_id),
			Error::<Test>::CannotRemoveAuction
		);
	});
}
