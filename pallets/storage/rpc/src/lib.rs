use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
pub use pallet_storage_rpc_runtime_api::FileStorageApi as FileStorageRuntimeApi;
use pallet_storage_rpc_runtime_api::{NodeDepositInfo, StoreFeeInfo};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT},
};

const RUNTIME_ERROR: i32 = 1;

#[rpc(client, server)]
pub trait FileStorageApi<AccountId, Balance, BlockNumber, ResponseFeeType, ResponseDepsoitType> {
	#[method(name = "fileStorage_storeFee")]
	fn store_fee(&self, file_size: u64, time: BlockNumber) -> RpcResult<ResponseFeeType>;
	#[method(name = "fileStorage_nodeDeposit")]
	fn node_deposit(&self, controller: AccountId) -> RpcResult<ResponseDepsoitType>;
}

/// A struct that implements the [`FileStorageApi`].
pub struct FileStorage<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> FileStorage<C, B> {
	/// Create new `FileStorage` with the given reference to the client.
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

impl<Client, Block, AccountId, Balance, BlockNumber>
	FileStorageApiServer<AccountId, Balance, BlockNumber, StoreFeeInfo<Balance>, NodeDepositInfo<Balance>>
	for FileStorage<Client, Block>
where
	Block: BlockT,
	Client: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: FileStorageRuntimeApi<Block, AccountId, Balance, BlockNumber>,
	Balance: Codec + Copy + TryFrom<NumberOrHex> + Into<NumberOrHex>,
	AccountId: Codec,
	BlockNumber: Codec,
{
	fn store_fee(&self, file_size: u64, time: BlockNumber) -> RpcResult<StoreFeeInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.store_fee(&at, file_size, time).map_err(runtime_error_into_rpc_err)
	}

	fn node_deposit(&self, controller: AccountId) -> RpcResult<NodeDepositInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.node_deposit(&at, &controller).map_err(runtime_error_into_rpc_err)
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