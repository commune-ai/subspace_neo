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


    // ---- The implementation for the extrinsic do_burned_registration: registering by burning TAO.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling key. 
    //             Burned registers can only be created by the key.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    // 
    // 	* 'key' ( T::AccountId ):
    // 		- key to be registered to the network.
    //   
    // # Event:
    // 	* ModuleRegistered;
    // 		- On successfully registereing a uid to a module slot on a subnetwork.
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
    pub fn do_burned_registration( 
        origin: T::RuntimeOrigin,
        netuid: u16, 
    ) -> DispatchResult {

        // --- 1. Check that the caller has signed the transaction. (the key of the pairing)
        let key = ensure_signed( origin )?; 
        log::info!("do_registration( key:{:?} netuid:{:?} )", key, netuid );

        // --- 2. Ensure the passed network is valid.
        ensure!( Self::if_subnet_exist( netuid ), Error::<T>::NetworkDoesNotExist ); 

        // --- 3. Ensure we are not exceeding the max allowed registrations per block.
        ensure!( Self::get_registrations_this_block( netuid ) < Self::get_max_registrations_per_block( netuid ), Error::<T>::TooManyRegistrationsThisBlock );

        // --- 4. Ensure that the key is not already registered.
        ensure!( !Uids::<T>::contains_key( netuid, &key ), Error::<T>::AlreadyRegistered );

        // --- 6. Ensure the callers key has enough stake to perform the transaction.
        
        let registration_cost_as_balance = Self::u64_to_balance( Self::get_burn_as_u64( netuid ) ).unwrap();
        ensure!( Self::can_remove_balance_from_account( &key, registration_cost_as_balance ), Error::<T>::NotEnoughBalanceToStake );

        // --- 7. Ensure the remove operation from the key is a success.
        ensure!( Self::remove_balance_from_account( &key, registration_cost_as_balance ) == true, Error::<T>::BalanceWithdrawalError );
        
        // The burn occurs here.
        TotalIssuance::<T>::put( TotalIssuance::<T>::get().saturating_sub( Self::get_burn_as_u64( netuid ) ) );

 
        // --- 10. Append module or prune it.
        let subnetwork_uid: u16;
        let current_subnetwork_n: u16 = Self::get_subnetwork_n( netuid );

        // Possibly there is no module slots at all.
        ensure!( Self::get_max_allowed_uids( netuid ) != 0, Error::<T>::NetworkDoesNotExist );
        
        if current_subnetwork_n < Self::get_max_allowed_uids( netuid ) {
            // --- 11.1.1 No replacement required, the uid appends the subnetwork.
            // We increment the subnetwork count here but not below.
            subnetwork_uid = current_subnetwork_n;

            // --- 11.1.2 Expand subnetwork with new account.
            Self::append_module( netuid, &key, current_block_number );
            log::info!("add new module account");
        } else {
            // --- 12.1.1 Replacement required.
            // We take the module with the lowest pruning score here.
            subnetwork_uid = Self::get_module_to_prune( netuid );

            // --- 12.1.1 Replace the module account with the new info.
            Self::replace_module( netuid, subnetwork_uid, &key, current_block_number );
            log::info!("prune module");
        }

        // --- 13. Record the registration and increment block and interval counters.
        BurnRegistrationsThisInterval::<T>::mutate( netuid, |val| *val += 1 );
        RegistrationsThisInterval::<T>::mutate( netuid, |val| *val += 1 );
        RegistrationsThisBlock::<T>::mutate( netuid, |val| *val += 1 );
    
        // --- 14. Deposit successful event.
        log::info!("ModuleRegistered( netuid:{:?} uid:{:?} key:{:?}  ) ", netuid, subnetwork_uid, key );
        Self::deposit_event( Event::ModuleRegistered( netuid, subnetwork_uid, key ) );

        // --- 15. Ok and done.
        Ok(())
    }

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
    // 		- On successfully registereing a uid to a module slot on a subnetwork.
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
    // 	* 'InvalidWorkBlock':
    // 		- The work has been performed on a stale, future, or non existent block.
    //
    // 	* 'WorkRepeated':
    // 		- This work for block has already been used.
    //

    // 	* 'InvalidSeal':
    // 		- The seal is incorrect.
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
        ensure!( Self::if_subnet_exist( netuid ), Error::<T>::NetworkDoesNotExist ); 

        // --- 3. Ensure we are not exceeding the max allowed registrations per block.
        ensure!( Self::get_registrations_this_block( netuid ) < Self::get_max_registrations_per_block( netuid ), Error::<T>::TooManyRegistrationsThisBlock );

        // --- 4. Ensure that the key is not already registered.
        ensure!( !Uids::<T>::contains_key( netuid, &key ), Error::<T>::AlreadyRegistered );

        // --- 11. Ensure that the pairing is correct.

        // --- 12. Append module or prune it.
        let subnetwork_uid: u16;
        let current_subnetwork_n: u16 = Self::get_subnetwork_n( netuid );

        // Possibly there is no module slots at all.
        ensure!( Self::get_max_allowed_uids( netuid ) != 0, Error::<T>::NetworkDoesNotExist );
        
        if current_subnetwork_n < Self::get_max_allowed_uids( netuid ) {

            // --- 12.1.1 No replacement required, the uid appends the subnetwork.
            // We increment the subnetwork count here but not below.
            subnetwork_uid = current_subnetwork_n;

            // --- 12.1.2 Expand subnetwork with new account.
            Self::append_module( netuid, &key, current_block_number );
            log::info!("add new module account");
        } else {
            // --- 12.1.1 Replacement required.
            // We take the module with the lowest pruning score here.
            subnetwork_uid = Self::get_module_to_prune( netuid );

            // --- 12.1.1 Replace the module account with the new info.
            Self::replace_module( netuid, subnetwork_uid, &key, current_block_number );
            log::info!("prune module");
        }

        // --- 14. Record the registration and increment block and interval counters.
        RegistrationsThisInterval::<T>::mutate( netuid, |val| *val += 1 );
        RegistrationsThisBlock::<T>::mutate( netuid, |val| *val += 1 );
    
        // --- 15. Deposit successful event.
        log::info!("ModuleRegistered( netuid:{:?} uid:{:?} key:{:?}  ) ", netuid, subnetwork_uid, key );
        Self::deposit_event( Event::ModuleRegistered( netuid, subnetwork_uid, key ) );

        // --- 16. Ok and done.
        Ok(())
    }


    pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( de_de_ref_hash );
        return real_hash
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning score out of
    // immunity period. If all modules are in immunity period, return node with lowest prunning score.
    // This function will always return an element to prune.
    pub fn get_module_to_prune(netuid: u16) -> u16 {
        let mut min_score : u16 = u16::MAX;
        let mut min_score_in_immunity_period = u16::MAX;
        let mut uid_with_min_score = 0;
        let mut uid_with_min_score_in_immunity_period: u16 =  0;
        if Self::get_subnetwork_n( netuid ) == 0 { return 0 } // If there are no modules in this network.
        for module_uid_i in 0..Self::get_subnetwork_n( netuid ) {
            let pruning_score:u16 = Self::get_pruning_score_for_uid( netuid, module_uid_i );
            let block_at_registration: u64 = Self::get_module_block_at_registration( netuid, module_uid_i );
            let current_block :u64 = Self::get_current_block_as_u64();
            let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
            if min_score == pruning_score {
                if current_block - block_at_registration <  immunity_period { //module is in immunity period
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
            // Find min pruning score.
            else if min_score > pruning_score { 
                if current_block - block_at_registration <  immunity_period { //module is in immunity period
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



    pub fn get_block_hash_from_u64 ( block_number: u64 ) -> H256 {
        let block_number: T::BlockNumber = TryInto::<T::BlockNumber>::try_into( block_number ).ok().expect("convert u64 to block number.");
        let block_hash_at_number: <T as frame_system::Config>::Hash = system::Pallet::<T>::block_hash( block_number );
        let vec_hash: Vec<u8> = block_hash_at_number.as_ref().into_iter().cloned().collect();
        let deref_vec_hash: &[u8] = &vec_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( deref_vec_hash );

        log::trace!(
			target: LOG_TARGET,
			"block_number: {:?}, vec_hash: {:?}, real_hash: {:?}",
			block_number,
			vec_hash,
			real_hash
		);

        return real_hash;
    }

    pub fn hash_to_vec( hash: H256 ) -> Vec<u8> {
        let hash_as_bytes: &[u8] = hash.as_bytes();
        let hash_as_vec: Vec<u8> = hash_as_bytes.iter().cloned().collect();
        return hash_as_vec
    }

    pub fn create_seal_hash( block_number_u64: u64, nonce_u64: u64 ) -> H256 {
        let nonce = U256::from( nonce_u64 );
        let block_hash_at_number: H256 = Self::get_block_hash_from_u64( block_number_u64 );
        let block_hash_bytes: &[u8] = block_hash_at_number.as_bytes();
        let full_bytes: &[u8; 40] = &[
            nonce.byte(0),  nonce.byte(1),  nonce.byte(2),  nonce.byte(3),
            nonce.byte(4),  nonce.byte(5),  nonce.byte(6),  nonce.byte(7),

            block_hash_bytes[0], block_hash_bytes[1], block_hash_bytes[2], block_hash_bytes[3],
            block_hash_bytes[4], block_hash_bytes[5], block_hash_bytes[6], block_hash_bytes[7],
            block_hash_bytes[8], block_hash_bytes[9], block_hash_bytes[10], block_hash_bytes[11],
            block_hash_bytes[12], block_hash_bytes[13], block_hash_bytes[14], block_hash_bytes[15],

            block_hash_bytes[16], block_hash_bytes[17], block_hash_bytes[18], block_hash_bytes[19],
            block_hash_bytes[20], block_hash_bytes[21], block_hash_bytes[22], block_hash_bytes[23],
            block_hash_bytes[24], block_hash_bytes[25], block_hash_bytes[26], block_hash_bytes[27],
            block_hash_bytes[28], block_hash_bytes[29], block_hash_bytes[30], block_hash_bytes[31],
        ];
        let sha256_seal_hash_vec: [u8; 32] = sha2_256( full_bytes );
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256( &sha256_seal_hash_vec );
        let seal_hash: H256 = H256::from_slice( &keccak_256_seal_hash_vec );

		log::trace!(
			"\nblock_number: {:?}, \nnonce_u64: {:?}, \nblock_hash: {:?}, \nfull_bytes: {:?}, \nsha256_seal_hash_vec: {:?},  \nkeccak_256_seal_hash_vec: {:?}, \nseal_hash: {:?}",
			block_number_u64,
			nonce_u64,
			block_hash_at_number,
			full_bytes,
			sha256_seal_hash_vec,
            keccak_256_seal_hash_vec,
			seal_hash
		);

        return seal_hash;
    }

}