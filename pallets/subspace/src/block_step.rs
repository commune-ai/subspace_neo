use super::*;
use substrate_fixed::types::I110F18;
use substrate_fixed::types::I64F64;
use frame_support::inherent::Vec;
use frame_support::storage::IterableStorageMap;
use frame_support::storage::IterableStorageDoubleMap;

impl<T: Config> Pallet<T> { 

    pub fn block_step() {
        let block_number: u64 = Self::get_current_block_as_u64();
        log::debug!("block_step for block: {:?} ", block_number );
        // --- 1. Adjust difficulties.
		Self::adjust_registration_terms_for_networks( );
        // --- 2. Drains emission tuples ( key, amount ).
        Self::drain_emission( block_number );
        // --- 3. Generates emission tuples from epoch functions.
		Self::generate_emission( block_number );
    }

    // Helper function which returns the number of blocks remaining before we will run the epoch on this
    // network. Networks run their epoch when (block_number + netuid + 1 ) % (tempo + 1) = 0
    //
    pub fn blocks_until_next_epoch( netuid: u16, tempo: u16, block_number: u64 ) -> u64 { 
        if tempo == 0 { return 1000 } // Special case: tempo = 0, the network never runs.
        // tempo | netuid | # first epoch block
        //   1        0               0
        //   1        1               1
        //   2        0               1
        //   2        1               0
        //   100      0              99
        //   100      1              98
        return tempo as u64 - ( block_number + netuid as u64 + 1 ) % ( tempo as u64 + 1 )
    }

 
    // Helper function returns the number of tuples to drain on a particular step based on
    // the remaining tuples to sink and the block number
    //
    pub fn tuples_to_drain_this_block( netuid: u16, tempo: u16, block_number: u64, n_remaining: usize ) -> usize {
        let blocks_until_epoch: u64 = Self::blocks_until_next_epoch( netuid, tempo, block_number );  
        if blocks_until_epoch / 2 == 0 { return n_remaining } // drain all.
        if tempo / 2 == 0 { return n_remaining } // drain all
        if n_remaining == 0 { return 0 } // nothing to drain at all.
        // Else return enough tuples to drain all within half the epoch length.
        let to_sink_via_tempo: usize = n_remaining / (tempo as usize / 2);
        let to_sink_via_blocks_until_epoch: usize = n_remaining / (blocks_until_epoch as usize / 2);
        if to_sink_via_tempo > to_sink_via_blocks_until_epoch {
            return to_sink_via_tempo;   
        } else {
            return to_sink_via_blocks_until_epoch;
        }
    }

    pub fn has_loaded_emission_tuples( netuid: u16 ) -> bool { LoadedEmission::<T>::contains_key( netuid ) }
    pub fn get_loaded_emission_tuples( netuid: u16 ) -> Vec<(T::AccountId, u64)> { LoadedEmission::<T>::get( netuid ).unwrap() }

    // Reads from the loaded emission storage which contains lists of pending emission tuples ( key, amount )
    // and distributes small chunks of them at a time.
    //
    pub fn drain_emission( _: u64 ) {
        // --- 1. We iterate across each network.
        for ( netuid, _ ) in <Tempo<T> as IterableStorageMap<u16, u16>>::iter() {
            if !Self::has_loaded_emission_tuples( netuid ) { continue } // There are no tuples to emit.
            let tuples_to_drain: Vec<(T::AccountId, u64)> = Self::get_loaded_emission_tuples( netuid );
            for (key, amount) in tuples_to_drain.iter() {                 
                Self::emit_inflation_through_account( &key, *amount );
            }            
            LoadedEmission::<T>::remove( netuid );
        }
    }

    // Iterates through networks queues more emission onto their pending storage.
    // If a network has no blocks left until tempo, we run the epoch function and generate
    // more token emission tuples for later draining onto accounts.
    //
    pub fn generate_emission( block_number: u64 ) {

        // --- 1. Iterate through network ids.
        for ( netuid, tempo )  in <Tempo<T> as IterableStorageMap<u16, u16>>::iter() {

            // --- 2. Queue the emission due to this network.
            let new_queued_emission = EmissionValues::<T>::get( netuid );
            PendingEmission::<T>::mutate( netuid, | queued | *queued += new_queued_emission );
            log::debug!("netuid_i: {:?} queued_emission: +{:?} ", netuid, new_queued_emission );  
            // --- 3. Check to see if this network has reached tempo.
            if Self::blocks_until_next_epoch( netuid, tempo, block_number ) != 0 {
                // --- 3.1 No epoch, increase blocks since last step and continue,
                Self::set_blocks_since_last_step( netuid, Self::get_blocks_since_last_step( netuid ) + 1 );
                continue;
            }

            // --- 4 This network is at tempo and we are running its epoch.
            // First frain the queued emission.
            let emission_to_drain:u64 = PendingEmission::<T>::get( netuid ); 
            PendingEmission::<T>::insert( netuid, 0 );

            // --- 5. Run the epoch mechanism and return emission tuples for keys in the network.
            let emission_tuples_this_block: Vec<(T::AccountId, u64)> = Self::epoch( netuid, emission_to_drain );
                
            // --- 6. Check that the emission does not exceed the allowed total.
            let emission_sum: u128 = emission_tuples_this_block.iter().map( |(_account_id, e)| *e as u128 ).sum();
            if emission_sum > emission_to_drain as u128 { continue } // Saftey check.

            // --- 7. Sink the emission tuples onto the already loaded.
            let mut concat_emission_tuples: Vec<(T::AccountId, u64)> = emission_tuples_this_block.clone();
            if Self::has_loaded_emission_tuples( netuid ) {
                // 7.a We already have loaded emission tuples, so we concat the new ones.
                let mut current_emission_tuples: Vec<(T::AccountId, u64)> = Self::get_loaded_emission_tuples( netuid );
                concat_emission_tuples.append( &mut current_emission_tuples );
            } 
            LoadedEmission::<T>::insert( netuid, concat_emission_tuples );

            // --- 8 Set counters.
            Self::set_blocks_since_last_step( netuid, 0 );
            Self::set_last_mechanism_step_block( netuid, block_number );        
        }
    }
    // Distributes token inflation through the key based on emission. The call ensures that the inflation
    // is distributed onto the accounts in proportion of the stake  minus the take. This function
    //
    pub fn emit_inflation_through_account( key: &T::AccountId, emission: u64) {
        // --- 1.We simply increase the key's stake.
        Self::increase_stake_on_account( &key, emission ); 
            // --- 4. The emission proportion is remaining_emission * ( stake / total_stake ).
        log::debug!(" key: {:?} emission: +{:?} ", key, stake_proportion );

        }

    }


    // Adjusts the network difficulties of every active network. Reseting state parameters.
    //
    pub fn adjust_registration_terms_for_networks( ) {
        
        // --- 1. Iterate through each network.
        for ( netuid, _ )  in <NetworksAdded<T> as IterableStorageMap<u16, bool>>::iter(){

            // --- 2. Pull counters for network difficulty.
            let last_adjustment_block: u64 = Self::get_last_adjustment_block( netuid );
            let adjustment_interval: u16 = Self::get_adjustment_interval( netuid );
            let current_block: u64 = Self::get_current_block_as_u64( ); 
            log::debug!("netuid: {:?} last_adjustment_block: {:?} adjustment_interval: {:?} current_block: {:?}", 
                netuid,
                last_adjustment_block,
                adjustment_interval,
                current_block
            );

            // --- 3. Check if we are at the adjustment interval for this network.
            // If so, we need to adjust the registration difficulty based on target and actual registrations.
            if ( current_block - last_adjustment_block ) >= adjustment_interval as u64 {

                // --- 4. Get the current counters for this network and difficulty values.
                let registrations_this_interval: u16 = Self::get_registrations_this_interval( netuid );
                let target_registrations_this_interval: u16 = Self::get_target_registrations_per_interval( netuid );
   

                // --- 5. Drain all counters for this network for this interval.
                Self::set_last_adjustment_block( netuid, current_block );
                Self::set_registrations_this_interval( netuid, 0 );
            }

            // --- 6. Drain block registrations for each network. Needed for registration rate limits.
            Self::set_registrations_this_block( netuid, 0 );
        }
    }


}
