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
		// First year mine 50_000_000 DOLLARS, 75% decay factor every year
		let years = now / YEARS;
		match years {
			0 => 950642 * MILLICENTS,
			1 => 712981 * MILLICENTS,
			2 => 534736 * MILLICENTS,
			3 => 401052 * MILLICENTS,
			4 => 300789 * MILLICENTS,
			5 => 225591 * MILLICENTS,
			6 => 169193 * MILLICENTS,
			7 => 126895 * MILLICENTS,
			8 =>  95171 * MILLICENTS,
			_ =>  71378 * MILLICENTS,
		}
	}
}
