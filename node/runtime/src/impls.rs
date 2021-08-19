use crate::{
	constants::{currency::*, time::YEARS},
	Authorship, Balance, Balances, BlockNumber, NegativeImbalance,
};
use frame_support::traits::{Currency, OnUnbalanced};
use pallet_storage::Payout;

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
			7 => 76137 * MILLICENTS,
			8 => 57102 * MILLICENTS,
			_ => 42827 * MILLICENTS,
		}
	}
}
