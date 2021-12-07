use crate::{Authorship, Balances, NegativeImbalance};
use frame_support::traits::{Currency, OnUnbalanced};

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		Balances::resolve_creating(&Authorship::author(), amount);
	}
}
