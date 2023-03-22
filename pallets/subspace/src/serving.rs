use super::*;
use frame_support::inherent::Vec;
use frame_support::sp_std::vec;


impl<T: Config> Pallet<T> {

    // ---- The implementation for the extrinsic serve_module which sets the ip endpoint information for a uid on a network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'version' (u64):
    // 		- The commune version identifier.
    //
    // 	* 'ip' (u64):
    // 		- The endpoint ip information as a u128 encoded integer.
    //
    // 	* 'port' (u16):
    // 		- The endpoint port information as a u16 encoded integer.
    // 
    // 	* 'ip_type' (u8):
    // 		- The endpoint ip version as a u8, 4 or 6.
    //
    // 	* 'protocol' (u8):
    // 		- UDP:1 or TCP:0 
    //
    // 	* 'placeholder1' (u8):
    // 		- Placeholder for further extra params.
    //
    // 	* 'placeholder2' (u8):
    // 		- Placeholder for further extra params.
    //
    // # Event:
    // 	* ModuleServed;
    // 		- On successfully serving the module info.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'InvalidIpType':
    // 		- The ip type is not 4 or 6.
    //
    // 	* 'InvalidIpAddress':
    // 		- The numerically encoded ip address does not resolve to a proper ip.
    //
    // 	* 'ServingRateLimitExceeded':
    // 		- Attempting to set prometheus information withing the rate limit min.
    //
    pub fn do_serve_module( 
        origin: T::RuntimeOrigin, 
		netuid: u16,
        ip: u128, 
        port: u16, 
        uri: Vec<u8>, // contains a uri string
    ) -> dispatch::DispatchResult {
        // --- 1. We check the callers (key) signature.
        let key_id = ensure_signed(origin)?;

        // --- 2. Ensure the key is registered somewhere.
        ensure!( Self::is_key_registered_on_any_network( &key_id ), Error::<T>::NotRegistered );  
        
        // --- 3. Check the ip signature validity.
        ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpAddress );
  

        // --- 4. Get the previous module information.
        let mut prev_module = Self::get_module( netuid, &key_id );
        let current_block:u64 = Self::get_current_block_as_u64();
        ensure!( Self::module_passes_rate_limit( netuid, &prev_module, current_block ), Error::<T>::ServingRateLimitExceeded );  

        // --- 6. We insert the module meta.
        prev_module.block = Self::get_current_block_as_u64();
        prev_module.ip = ip;
        prev_module.port = port;
        prev_module.uri = uri;
        Modules::<T>::insert( netuid, key_id.clone(), prev_module );

        // --- 7. We deposit module served event.
        log::info!("ModuleServed( key:{:?} ) ", key_id.clone() );
        Self::deposit_event(Event::ModuleServed( netuid, key_id ));

        // --- 8. Return is successful dispatch. 
        Ok(())
    }

    // ---- The implementation for the extrinsic serve_prometheus.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'version' (u64):
    // 		- The commune version identifier.
    //
    // 	* 'ip' (u64):
    // 		- The prometheus ip information as a u128 encoded integer.
    //
    // 	* 'port' (u16):
    // 		- The prometheus port information as a u16 encoded integer.
    // 
    // 	* 'ip_type' (u8):
    // 		- The prometheus ip version as a u8, 4 or 6.
    //
    // # Event:
    // 	* PrometheusServed;
    // 		- On successfully serving the module info.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'InvalidIpType':
    // 		- The ip type is not 4 or 6.
    //
    // 	* 'InvalidIpAddress':
    // 		- The numerically encoded ip address does not resolve to a proper ip.
    //
    // 	* 'ServingRateLimitExceeded':
    // 		- Attempting to set prometheus information withing the rate limit min.
    //

    /********************************
     --==[[  Helper functions   ]]==--
    *********************************/

    pub fn module_passes_rate_limit( netuid: u16, prev_module: &Module, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_serving_rate_limit(netuid);
        let last_serve = prev_module.block;
        return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
    }


    pub fn has_module( netuid: u16, key: &T::AccountId ) -> bool {
        return Modules::<T>::contains_key( netuid, key );
    }


    pub fn get_module( netuid: u16, key: &T::AccountId ) -> Module {
        if Self::has_module( netuid, key ) {
            return Modules::<T>::get( netuid, key ).unwrap();
        } else{
            return Module { 
                block: 0,
                version: 0,
                ip: 0,
                port: 0,
            }

        }
    }

    pub fn is_valid_ip_type(ip_type: u8) -> bool {
        let allowed_values: Vec<u8> = vec![4, 6];
        return allowed_values.contains(&ip_type);
    }


    // @todo (Parallax 2-1-2021) : Implement exclusion of private IP ranges
    pub fn is_valid_ip_address(addr: u128) -> bool {
        let ip_type = Self::get_ip_type(addr);
        if addr == 0 {
            return false;
        }
        if ip_type == 4 {
            if addr == 0 { return false; }
            if addr >= u32::MAX as u128 { return false; }
            if addr == 0x7f000001 { return false; } // Localhost
        }
        if ip_type == 6 {
            if addr == 0x0 { return false; }
            if addr == u128::MAX { return false; }
            if addr == 1 { return false; } // IPv6 localhost
        }
        return true;
    }


    fn get_ip_type(ip: u128) -> u8 {
        if ip <= u32::MAX as u128 {
            return 4;
        } else if ip <= u128::MAX {
            return 6;
        } 

        // If the IP address is not IPv4 or IPv6 and not private, raise an error
        return 0;
    } 


}