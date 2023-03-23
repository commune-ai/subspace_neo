//! RPC interface for the custom Subspace rpc methods

use jsonrpsee::{
	core::RpcResult,
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sp_blockchain::HeaderBackend;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT}
};
use std::sync::Arc;


use sp_api::ProvideRuntimeApi;

pub use subspace_custom_rpc_runtime_api::ModuleRuntimeApi;
pub use subspace_custom_rpc_runtime_api::NetworkRuntimeApi;

#[rpc(client, server)]
pub trait SubspaceCustomApi<BlockHash> {

	#[method(name = "module_getModules")]
	fn get_modules(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "module_getModule")]
	fn get_module(&self, netuid: u16, uid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;

	#[method(name = "network_getNetwork")]
	fn get_network(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getNetworksInfo")]
	fn get_networks(&self, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
}

pub struct SubspaceCustom<C, P> {
	/// Shared reference to the client.
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> SubspaceCustom<C, P> {
	/// Creates a new instance of the TransactionPayment Rpc helper.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i32 {
	fn from(e: Error) -> i32 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

impl<C, Block> SubspaceCustomApiServer<<Block as BlockT>::Hash> for SubspaceCustom<C, Block>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: ModuleRuntimeApi<Block>,
	C::Api: NetworkRuntimeApi<Block>,
	{ 



	fn get_modules(
		&self,
		netuid: u16,
		at: Option<<Block as BlockT>::Hash>
	) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_modules(at, netuid).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get modules info.",
				Some(e.to_string()),
			)).into()
		})
	}

	fn get_module(
		&self,
		netuid: u16,
		uid: u16,
		at: Option<<Block as BlockT>::Hash>
	) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_module_from_uid(at, netuid, uid).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get module info.",
				Some(e.to_string()),
			)).into()
		})
	}
	
	fn get_network(
				&self, 
				netuid: u16, 
				at: Option<<Block as BlockT>::Hash>
			) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_network(at, netuid).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get network info.",
				Some(e.to_string()),
			)).into()
		})
	}

	fn get_networks(
		&self,
		at: Option<<Block as BlockT>::Hash>
	) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_networks(at).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
			Error::RuntimeError.into(),
			"Unable to get networks info.",
			Some(e.to_string()),
			)).into()
		})
	}
}
