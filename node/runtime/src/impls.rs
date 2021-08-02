use frame_support::traits::{OnUnbalanced, Currency};
use pallet_storage::Payout;
use crate::constants::currency::*;
use crate::constants::time::YEARS;
use crate::{Balances, BlockNumber, Authorship, Balance, NegativeImbalance};

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		Balances::resolve_creating(&Authorship::author(), amount);
	}
}


pub struct FileStoragePayout;
impl Payout<Balance, BlockNumber> for FileStoragePayout {
	fn payout(now: BlockNumber) -> Balance {
		let years = now / YEARS;
		match years {
			0 => 570385 * MILLICENTS,
			1 => 427789 * MILLICENTS,
			2 => 320841 * MILLICENTS,
			3 => 240631 * MILLICENTS,
			4 => 180473 * MILLICENTS,
			5 => 135355 * MILLICENTS,
			6 => 101516 * MILLICENTS,
			7 =>  76137 * MILLICENTS,
			8 =>  57102 * MILLICENTS,
			_ =>  42827 * MILLICENTS,
		}
	}
}
