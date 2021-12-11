use super::*;

pub mod v2 {
    use super::*;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub struct OldOrderDetails<AccountId, Balance, BlockNumber> {
        /// Who create the order.
        pub owner: AccountId,
        /// Price of this order.
        pub price: Balance,
        /// The balances to create an order
        pub deposit: Balance,
        /// This order will be invalidated after `deadline` block number.
        pub deadline: Option<BlockNumber>,
    }
}