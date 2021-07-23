use frame_support::traits::{OnUnbalanced, Currency};
use pallet_storage::RoundPayout;
use crate::constants::currency::*;
use crate::{Balances, Authorship, Balance, NegativeImbalance};

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		Balances::resolve_creating(&Authorship::author(), amount);
	}
}

pub struct SimpleRoundPayout;
impl RoundPayout<Balance> for SimpleRoundPayout {
	fn round_payout(_total_size: u128) -> Balance {
		1000 * DOLLARS
	}
}
