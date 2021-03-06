use std::{convert::TryInto, sync::Arc};

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use pallet_storage_rpc_runtime_api::FileStorageApi as FileStorageRuntimeApi;
use pallet_storage_rpc_runtime_api::{NodeDepositInfo, StoreFeeInfo};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};

#[rpc]
pub trait FileStorageApi<AccountId, Balance, BlockNumber, ResponseFeeType, ResponseDepsoitType> {
	#[rpc(name = "fileStorage_storeFee")]
	fn store_fee(&self, file_size: u64, time: BlockNumber) -> Result<ResponseFeeType>;
	#[rpc(name = "fileStorage_nodeDeposit")]
	fn node_deposit(&self, controller: AccountId) -> Result<ResponseDepsoitType>;
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

impl<C, Block, AccountId, Balance, BlockNumber>
	FileStorageApi<AccountId, Balance, BlockNumber, StoreFeeInfo<Balance>, NodeDepositInfo<Balance>>
	for FileStorage<C, Block>
where
	Block: BlockT,
	C: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: FileStorageRuntimeApi<Block, AccountId, Balance, BlockNumber>,
	Balance: Codec + MaybeDisplay + MaybeFromStr + Copy + TryInto<NumberOrHex>,
	AccountId: Codec,
	BlockNumber: Codec,
{
	fn store_fee(&self, file_size: u64, time: BlockNumber) -> Result<StoreFeeInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.store_fee(&at, file_size, time).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to query dispatch info.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
	fn node_deposit(&self, controller: AccountId) -> Result<NodeDepositInfo<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.node_deposit(&at, &controller).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to query dispatch info.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
