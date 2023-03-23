#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
use alloc::vec::Vec;

// Here we declare the runtime API. It is implemented it the `impl` block in
// src/module.rs, src/network.rs 
sp_api::decl_runtime_apis! {
	pub trait ModuleRuntimeApi {
		fn get_module(netuid: u16, uid: u16) -> Vec<u8>;
		fn get_modules(netuid: u16) -> Vec<u8>;
	}
	pub trait NetworkRuntimeApi {
		fn get_network(netuid: u16) -> Vec<u8>;
		fn get_networks() -> Vec<u8>;
	}
}