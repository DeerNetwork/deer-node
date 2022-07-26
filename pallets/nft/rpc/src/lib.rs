use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use pallet_nft_rpc_runtime_api::BalanceInfo;
pub use pallet_nft_rpc_runtime_api::NFTApi as NFTRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT},
};

const RUNTIME_ERROR: i32 = 1;

#[rpc(client, server)]
pub trait NFTApi<Balance, ResponseType> {
	#[method(name = "nft_createClassDeposit")]
	fn create_class_deposit(&self, bytes_len: u32) -> RpcResult<ResponseType>;

	#[method(name = "nft_mintTokenDeposit")]
	fn mint_token_deposit(&self, bytes_len: u32) -> RpcResult<ResponseType>;
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

impl<Client, Block, Balance> NFTApiServer<Balance, BalanceInfo<Balance>> for NFT<Client, Block>
where
	Block: BlockT,
	Client: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: NFTRuntimeApi<Block, Balance>,
	Balance: Codec + Copy + TryFrom<NumberOrHex> + Into<NumberOrHex>,
{
	fn create_class_deposit(&self, bytes_len: u32) -> RpcResult<BalanceInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.create_class_deposit(&at, bytes_len).map_err(runtime_error_into_rpc_err)
	}

	fn mint_token_deposit(&self, bytes_len: u32) -> RpcResult<BalanceInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.mint_token_deposit(&at, bytes_len).map_err(runtime_error_into_rpc_err)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(
		RUNTIME_ERROR,
		"Runtime error",
		Some(format!("{:?}", err)),
	))
	.into()
}