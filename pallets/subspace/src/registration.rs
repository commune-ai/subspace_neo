use super::*;
use frame_support::{ pallet_prelude::DispatchResult};
use sp_std::convert::TryInto;
use sp_core::{H256, U256};
use crate::system::ensure_root;
use sp_io::hashing::sha2_256;
use sp_io::hashing::keccak_256;
use frame_system::{ensure_signed};
use sp_std::vec::Vec;
use substrate_fixed::types::I32F32;

const LOG_TARGET: &'static str = "runtime::subspace::registration";

impl<T: Config> Pallet<T> {



    // ---- The implementation for the extrinsic do_registration.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling key.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'block_number' ( u64 ):
    // 		- Block hash used to prove work done.
    //
    // 	* 'nonce' ( u64 ):
    // 		- Positive integer nonce used in POW.
    //
    // 	* 'work' ( Vec<u8> ):
    // 		- Vector encoded bytes representing work done.
    //
    //
    // # Event:
    // 	* ModuleRegistered;
    // 		- On successfully registereing a uid to a module slot on a network.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to registed to a non existent network.
    //
    // 	* 'TooManyRegistrationsThisBlock':
    // 		- This registration exceeds the total allowed on this network this block.
    //
    // 	* 'AlreadyRegistered':
    // 		- The key is already registered on this network.
    //
    pub fn do_registration( 
        origin: T::RuntimeOrigin,
        netuid: u16, 
    ) -> DispatchResult {


        // --- 1. Check that the caller has signed the transaction. 
        // TODO( const ): This not be the key signature or else an exterior actor can register the key and potentially control it?
        let key = ensure_signed( origin )?;        
        log::info!("do_registration( origin:{:?} netuid:{:?} key:{:?} )", signing_origin, netuid, key );

        let current_block_number: u64 = Self::get_current_block_as_u64();

        // --- 2. Ensure the passed network is valid.
        ensure!( Self::if_network_exist( netuid ), Error::<T>::NetworkDoesNotExist ); 

        // --- 3. Ensure we are not exceeding the max allowed registrations per block.
        ensure!( Self::get_registrations_this_block( netuid ) < Self::get_max_registrations_per_block( netuid ), Error::<T>::TooManyRegistrationsThisBlock );

        // --- 4. Ensure that the key is not already registered.
        ensure!( !Uids::<T>::contains_key( netuid, &key ), Error::<T>::AlreadyRegistered );

        // --- 11. Ensure that the pairing is correct.

        // --- 12. Append module or prune it.
        let network_uid: u16;
        let current_network_n: u16 = Self::get_network_n( netuid );

        // Possibly there is no module slots at all.
        ensure!( Self::get_max_allowed_uids( netuid ) != 0, Error::<T>::NetworkDoesNotExist );
        
        if current_network_n < Self::get_max_allowed_uids( netuid ) {

            // --- 12.1.1 No replacement required, the uid appends the network.
            // We increment the network count here but not below.
            network_uid = current_network_n;

            // --- 12.1.2 Expand network with new account.
            Self::append_module( netuid, &key, current_block_number );
            log::info!("add new module account");
        } else {
            // --- 12.1.1 Replacement required.
            // We take the module with the lowest pruning score here.
            network_uid = Self::get_module_to_prune( netuid );

            // --- 12.1.1 Replace the module account with the new info.
            Self::replace_module( netuid, network_uid, &key, current_block_number );
            log::info!("prune module");
        }

        // --- 14. Record the registration and increment block and interval counters.
        RegistrationsThisInterval::<T>::mutate( netuid, |val| *val += 1 );
        RegistrationsThisBlock::<T>::mutate( netuid, |val| *val += 1 );
    
        // --- 15. Deposit successful event.
        log::info!("ModuleRegistered( netuid:{:?} uid:{:?} key:{:?}  ) ", netuid, network_uid, key );
        Self::deposit_event( Event::ModuleRegistered( netuid, network_uid, key ) );

        // --- 16. Ok and done.
        Ok(())
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning score out of
    // immunity period. If all modules are in immunity period, return node with lowest prunning score.
    // This function will always return an element to prune.
    pub fn get_module_to_prune(netuid: u16) -> u16 {
        let mut min_score : u16 = u16::MAX;
        let mut min_score_in_immunity_period = u16::MAX;
        let mut uid_with_min_score = 0;
        let mut uid_with_min_score_in_immunity_period: u16 =  0;
        if Self::get_network_n( netuid ) == 0 { return 0 } // If there are no modules in this network.
        for module_uid_i in 0..Self::get_network_n( netuid ) {
            let pruning_score:u16 = Self::get_pruning_score_for_uid( netuid, module_uid_i );
            let block_at_registration: u64 = Self::get_module_block_at_registration( netuid, module_uid_i );
            let current_block :u64 = Self::get_current_block_as_u64();
            let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
            // Find min pruning score.
            if min_score >= pruning_score { 
                if current_block - block_at_registration <  immunity_period;  { //module is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                        min_score_in_immunity_period = pruning_score; 
                        uid_with_min_score_in_immunity_period = module_uid_i;
                    }
                }
                else {
                    min_score = pruning_score; 
                    uid_with_min_score = module_uid_i;
                }
            }
        }
        if min_score == u16::MAX { //all neuorns are in immunity period
            Self::set_pruning_score_for_uid( netuid, uid_with_min_score_in_immunity_period, u16::MAX );
            return uid_with_min_score_in_immunity_period;
        }
        else {
            // We replace the pruning score here with u16 max to ensure that all peers always have a 
            // pruning score. In the event that every peer has been pruned this function will prune
            // the last element in the network continually.
            Self::set_pruning_score_for_uid( netuid, uid_with_min_score, u16::MAX );
            return uid_with_min_score;
        }
    } 



}