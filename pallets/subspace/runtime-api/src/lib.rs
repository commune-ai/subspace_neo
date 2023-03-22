#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
use alloc::vec::Vec;

// Here we declare the runtime API. It is implemented it the `impl` block in
// src/module.rs, src/subnet_info.rs 
sp_api::decl_runtime_apis! {

	pub trait ModuleRuntimeApi {
		fn get_modules(netuid: u16) -> Vec<u8>;
		fn get_module(netuid: u16, uid: u16) -> Vec<u8>;
	}

	pub trait SubnetInfoRuntimeApi {
		fn get_subnet_info(netuid: u16) -> Vec<u8>;
		fn get_subnets_info() -> Vec<u8>;
	}
}