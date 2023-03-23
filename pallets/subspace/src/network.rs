use super::*;
use frame_support::{sp_std::vec};
use sp_std::vec::Vec;
use frame_system::ensure_root;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;


#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct Network {
    netuid: Compact<u16>,
    immunity_period: Compact<u16>,
    max_allowed_validators: Compact<u16>,
    min_allowed_weights: Compact<u16>,
    max_weights_limit: Compact<u16>,
    network_n: Compact<u16>,
    max_allowed_uids: Compact<u16>,
    blocks_since_last_step: Compact<u64>,
    tempo: Compact<u16>,
    network_connect: Vec<[u16; 2]>,
    emission_values: Compact<u64>,
}
impl<T: Config> Pallet<T> { 


    // ---- The implementation for the extrinsic add_network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- Must be sudo.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'tempo' ( u16 ):
    // 		- Number of blocks between epoch step.
    //

    // # Event:
    // 	* NetworkAdded;
    // 		- On successfully creation of a network.
    //
    // # Raises:
    // 	* 'NetworkExist':
    // 		- Attempting to register an already existing.
    //

    // 	* 'InvalidTempo':
    // 		- Attempting to register a network with an invalid tempo.
    //
    pub fn do_add_network( 
        origin: T::RuntimeOrigin, 
        netuid: u16, 
        tempo: u16, 
    ) -> dispatch::DispatchResultWithPostInfo {

        // --- 1. Ensure this is a sudo caller.
        ensure_root( origin )?;

        // --- 2. Ensure this network does not already exist.
        ensure!( !Self::if_network_exist( netuid ), Error::<T>::NetworkExist );


        // --- 4. Ensure the tempo is valid.
        ensure!( Self::if_tempo_is_valid( tempo ), Error::<T>::InvalidTempo );

        // --- 5. Initialize the network and all its parameters.
        Self::init_new_network( netuid, tempo );
        
        // --- 6. Emit the new network event.
        log::info!("NetworkAdded( netuid:{:?} )", netuid);
        Self::deposit_event( Event::NetworkAdded( netuid ) );

        // --- 7. Ok and return.
        Ok(().into())
    }

    // ---- The implementation for the extrinsic remove_network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- Must be sudo.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // # Event:
    // 	* NetworkRemoved;
    // 		- On the successfull removing of this network.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to remove a non existent network.
    //
    pub fn do_remove_network( origin: T::RuntimeOrigin, netuid: u16 ) -> dispatch::DispatchResult {

        // --- 1. Ensure the function caller it Sudo.
        ensure_root( origin )?;

        // --- 2. Ensure the network to be removed exists.
        ensure!( Self::if_network_exist( netuid ), Error::<T>::NetworkDoesNotExist );

        // --- 3. Explicitly erase the network and all its parameters.
        Self::remove_network( netuid );
    
        // --- 4. Emit the event.
        log::info!("NetworkRemoved( netuid:{:?} )", netuid);
        Self::deposit_event( Event::NetworkRemoved( netuid ) );

        // --- 5. Ok and return.
        Ok(())
    }

    // ---- The implementation for the extrinsic set_emission_values.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- Must be sudo.
    //
   	// 	* `netuids` (Vec<u16>):
	// 		- A vector of network uids values. This must include all netuids.
	//
	// 	* `emission` (Vec<u64>):
	// 		- The emission values associated with passed netuids in order.
    //
    // # Event:
    // 	* NetworkRemoved;
    // 		- On the successfull removing of this network.
    //
    // # Raises:
    // 	* 'EmissionValuesDoesNotMatchNetworks':
    // 		- Attempting to remove a non existent network.
    //
    pub fn do_set_emission_values( 
        origin: T::RuntimeOrigin, 
        netuids: Vec<u16>,
        emission: Vec<u64>
    ) -> dispatch::DispatchResult {

        // --- 1. Ensure caller is sudo.
        ensure_root( origin )?;

        // --- 2. Ensure emission values match up to network uids.
        ensure!( netuids.len() == emission.len(), Error::<T>::WeightVecNotEqualSize );

        // --- 3. Ensure we are setting emission for all networks. 
        ensure!( netuids.len() as u16 == TotalNetworks::<T>::get(), Error::<T>::NotSettingEnoughWeights );

        // --- 4. Ensure the passed uids contain no duplicates.
        ensure!( !Self::has_duplicate_netuids( &netuids ), Error::<T>::DuplicateUids );

        // --- 5. Ensure that the passed uids are valid for the network.
        ensure!( !Self::contains_invalid_netuids( &netuids ), Error::<T>::InvalidUid );

        // --- 6. check if sum of emission rates is equal to 1.
        ensure!( emission.iter().sum::<u64>() as u64 == Self::get_block_emission(), Error::<T>::InvalidEmissionValues);

        // --- 7. Add emission values for each network
        Self::set_emission_values( &netuids, &emission );

        // --- 8. Add emission values for each network
        log::info!("EmissionValuesSet()");
        Self::deposit_event( Event::EmissionValuesSet() );

        // --- 9. Ok and return.
        Ok(())
    }

    // Initializes a new network under netuid with parameters.
    //
    pub fn init_new_network( netuid:u16, tempo:u16 ){

        // --- 1. Set network to 0 size.
        NetworkworkN::<T>::insert( netuid, 0 );

        // --- 2. Set this network uid to alive.
        NetworksAdded::<T>::insert( netuid, true );
        
        // --- 3. Fill tempo memory item.
        Tempo::<T>::insert( netuid, tempo );

        // --- 5. Increase total network count.
        TotalNetworks::<T>::mutate( |n| *n += 1 );

        // --- 6. Set all default values **explicitly**.
        Self::set_default_values_for_all_parameters( netuid );
    }

    // Removes the network (netuid) and all of its parameters.
    //
    pub fn remove_network( netuid:u16 ) {

        // --- 1. Remove network count.
        NetworkworkN::<T>::remove( netuid );


        // --- 3. Remove netuid from added networks.
        NetworksAdded::<T>::remove( netuid );

        // --- 4. Erase all memory associated with the network.
        Self::erase_all_network_data( netuid );

        // --- 5. Decrement the network counter.
        TotalNetworks::<T>::mutate(|val| *val -= 1);
    }


    // Explicitly sets all network parameters to their default values.
    // Note: this is required because, although there are defaults, they are not explicitly set until this call.
    //
    pub fn set_default_values_for_all_parameters(netuid: u16){
        // Make network parameters explicit.
        if !Tempo::<T>::contains_key( netuid ) { Tempo::<T>::insert( netuid, Tempo::<T>::get( netuid ));}
        if !MaxAllowedUids::<T>::contains_key( netuid ) { MaxAllowedUids::<T>::insert( netuid, MaxAllowedUids::<T>::get( netuid ));}
        if !ImmunityPeriod::<T>::contains_key( netuid ) { ImmunityPeriod::<T>::insert( netuid, ImmunityPeriod::<T>::get( netuid ));}
        if !ActivityCutoff::<T>::contains_key( netuid ) { ActivityCutoff::<T>::insert( netuid, ActivityCutoff::<T>::get( netuid ));}
        if !EmissionValues::<T>::contains_key( netuid ) { EmissionValues::<T>::insert( netuid, EmissionValues::<T>::get( netuid ));}   
        if !MaxWeightsLimit::<T>::contains_key( netuid ) { MaxWeightsLimit::<T>::insert( netuid, MaxWeightsLimit::<T>::get( netuid ));}
        if !MinAllowedWeights::<T>::contains_key( netuid ) { MinAllowedWeights::<T>::insert( netuid, MinAllowedWeights::<T>::get( netuid )); }
        if !RegistrationsThisInterval::<T>::contains_key( netuid ) { RegistrationsThisInterval::<T>::insert( netuid, RegistrationsThisInterval::<T>::get( netuid ));}
    }

    // Explicitly erases all data associated with this network.
    //
    pub fn erase_all_network_data(netuid: u16){

        // --- 1. Remove incentive mechanism memory.
        let _ = Uids::<T>::clear_prefix( netuid, u32::max_value(), None );
        let _ = Keys::<T>::clear_prefix( netuid, u32::max_value(), None );
        let _ = Bonds::<T>::clear_prefix( netuid, u32::max_value(), None );
        let _ = Weights::<T>::clear_prefix( netuid, u32::max_value(), None );

        Rank::<T>::remove( netuid );
        Active::<T>::remove( netuid );
        Emission::<T>::remove( netuid );
        Incentive::<T>::remove( netuid );
        Dividends::<T>::remove( netuid );
        PruningScores::<T>::remove( netuid );
        LastUpdate::<T>::remove( netuid );


        // --- 2. Erase network parameters.
        MaxAllowedUids::<T>::remove( netuid );
        ImmunityPeriod::<T>::remove( netuid );
        ActivityCutoff::<T>::remove( netuid );
        EmissionValues::<T>::remove( netuid );
        MaxWeightsLimit::<T>::remove( netuid );
        MinAllowedWeights::<T>::remove( netuid );
        RegistrationsThisInterval::<T>::remove( netuid );
    }



    // Returns true if the items contain duplicates.
    //
    fn has_duplicate_netuids( netuids: &Vec<u16> ) -> bool {
        let mut parsed: Vec<u16> = Vec::new();
        for item in netuids {
            if parsed.contains(&item) { return true; }
            parsed.push(item.clone());
        }
        return false;
    }

    // Checks for any invalid netuids on this network.
    //
    pub fn contains_invalid_netuids( netuids: &Vec<u16> ) -> bool {
        for netuid in netuids {
            if !Self::if_network_exist( *netuid ) {
                return true;
            }
        }
        return false;
    }

    // Set emission values for the passed networks. 
    //
    pub fn set_emission_values( netuids: &Vec<u16>, emission: &Vec<u64> ){
        for (i, netuid_i) in netuids.iter().enumerate() {
            Self::set_emission_for_network( *netuid_i, emission[i] ); 
        }
    }

    // Set the emission on a single network.
    //
    pub fn set_emission_for_network( netuid: u16, emission: u64 ){
        EmissionValues::<T>::insert( netuid, emission );
    }

    // Returns true if the network exists.
    //
    pub fn if_network_exist( netuid: u16 ) -> bool{
        return NetworksAdded::<T>::get( netuid );
    }

    // Returns true if the passed tempo is allowed.
    //
    pub fn if_tempo_is_valid(tempo: u16) -> bool {
        tempo < u16::MAX
    }

    pub fn get_network(netuid: u16) -> Option<Network> {
        if !Self::if_network_exist(netuid) {
            return None;
        }

        let difficulty = Self::get_difficulty_as_u64(netuid);
        let immunity_period = Self::get_immunity_period(netuid);
        let min_allowed_weights = Self::get_min_allowed_weights(netuid);
        let max_weights_limit = Self::get_max_weight_limit(netuid);
        let network_n = Self::get_network_n(netuid);
        let max_allowed_uids = Self::get_max_allowed_uids(netuid);
        let blocks_since_last_step = Self::get_blocks_since_last_step(netuid);
        let tempo = Self::get_tempo(netuid);
        let emission_values = Self::get_emission_value(netuid);


        let mut network_connect: Vec<[u16; 2]> = Vec::<[u16; 2]>::new();

        for ( _netuid_, con_req) in < NetworkConnect<T> as IterableStorageDoubleMap<u16, u16, u16> >::iter_prefix(netuid) {
            network_connect.push([_netuid_, con_req]);
        }

        return Some(Network {
            immunity_period: immunity_period.into(),
            netuid: netuid.into(),
            min_allowed_weights: min_allowed_weights.into(),
            max_weights_limit: max_weights_limit.into(),
            network_n: network_n.into(),
            max_allowed_uids: max_allowed_uids.into(),
            blocks_since_last_step: blocks_since_last_step.into(),
            tempo: tempo.into(),
            network_connect: network_connect,
            emission_values: emission_values.into(),
        })
    }

    pub fn get_networks() -> Vec<Option<Network>> {
        let mut network_netuids = Vec::<u16>::new();
        let mut max_netuid: u16 = 0;
        for ( netuid, added ) in < NetworksAdded<T> as IterableStorageMap<u16, bool> >::iter() {
            if added {
                network_netuids.push(netuid);
                if netuid > max_netuid {
                    max_netuid = netuid;
                }
            }
        }

        let mut networks = Vec::<Option<Network>>::new();
        for netuid_ in 0..(max_netuid + 1) {
            if network_netuids.contains(&netuid_) {
                networks.push(Self::get_network(netuid_));
            }
        }

        return networks;
    }



    // Return the total number of networks available on the chain.
    //
    pub fn get_number_of_networks()-> u16 {
        let mut number_of_networks : u16 = 0;
        for (_, _)  in <NetworkworkN<T> as IterableStorageMap<u16, u16>>::iter(){
            number_of_networks = number_of_networks + 1;
        }
        return number_of_networks;
    }

    }


    
    
}