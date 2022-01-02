use std::{convert::TryInto, sync::Arc};

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use pallet_nft_rpc_runtime_api::BalanceInfo;
pub use pallet_nft_rpc_runtime_api::NFTApi as NFTRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};

#[rpc]
pub trait NFTApi<Balance, ResponseType> {
	#[rpc(name = "nft_createClassDeposit")]
	fn create_class_deposit(&self, bytes_len: u32) -> Result<ResponseType>;

	#[rpc(name = "nft_mintTokenDeposit")]
	fn mint_token_deposit(&self, bytes_len: u32) -> Result<ResponseType>;
}

/// A struct that implements the [`NFTApi`].
pub struct NFT<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> NFT<C, B> {
	/// Create new `NFT` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The transaction was not decodable.
	DecodeError,
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
			Error::DecodeError => 2,
		}
	}
}

impl<C, Block, Balance> NFTApi<Balance, BalanceInfo<Balance>> for NFT<C, Block>
where
	Block: BlockT,
	C: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: NFTRuntimeApi<Block, Balance>,
	Balance: Codec + MaybeDisplay + MaybeFromStr + Copy + TryInto<NumberOrHex>,
{
	fn create_class_deposit(&self, bytes_len: u32) -> Result<BalanceInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.create_class_deposit(&at, bytes_len).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to query dispatch info.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn mint_token_deposit(&self, bytes_len: u32) -> Result<BalanceInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.mint_token_deposit(&at, bytes_len).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to query dispatch info.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
