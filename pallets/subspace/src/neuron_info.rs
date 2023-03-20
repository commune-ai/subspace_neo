use super::*;
use frame_support::storage::IterableStorageDoubleMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct ModuleInfo<T: Config> {
    key: T::AccountId,
    coldkey: T::AccountId,
    uid: Compact<u16>,
    netuid: Compact<u16>,
    active: bool,
    module_info: ModuleInfo,
    prometheus_info: PrometheusInfo,
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of coldkey to stake on this module/key (includes delegations)
    rank: Compact<u16>,
    emission: Compact<u64>,
    incentive: Compact<u16>,
    consensus: Compact<u16>,
    trust: Compact<u16>,
    dividends: Compact<u16>,
    last_update: Compact<u64>,
    weights: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, weight)
    bonds: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, bond)
    pruning_score: Compact<u16>,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct ModuleInfoLite<T: Config> {
    key: T::AccountId,
    coldkey: T::AccountId,
    uid: Compact<u16>,
    netuid: Compact<u16>,
    active: bool,
    module_info: ModuleInfo,
    prometheus_info: PrometheusInfo,
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of coldkey to stake on this module/key (includes delegations)
    rank: Compact<u16>,
    emission: Compact<u64>,
    incentive: Compact<u16>,
    consensus: Compact<u16>,
    trust: Compact<u16>,
    dividends: Compact<u16>,
    last_update: Compact<u64>,
    // has no weights or bonds
    pruning_score: Compact<u16>,
}

impl<T: Config> Pallet<T> {
	pub fn get_modules(netuid: u16) -> Vec<ModuleInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return Vec::new();
        }

        let mut modules = Vec::new();
        let n = Self::get_subnetwork_n(netuid);
        for uid in 0..n {
            let uid = uid;
            let netuid = netuid;

            let _module = Self::get_module_subnet_exists(netuid, uid);
            let module;
            if _module.is_none() {
                break; // No more modules
            } else {
                // No error, key was registered
                module = _module.expect("Module should exist");
            }

            modules.push( module );
        }
        modules
	}

    fn get_module_subnet_exists(netuid: u16, uid: u16) -> Option<ModuleInfo<T>> {
        let _key = Self::get_key_for_net_and_uid(netuid, uid);
        let key;
        if _key.is_err() {
            return None;
        } else {
            // No error, key was registered
            key = _key.expect("Hotkey should exist");
        }

        let module_info = Self::get_module_info( netuid, &key.clone() );

        let prometheus_info = Self::get_prometheus_info( netuid, &key.clone() );

        
        let coldkey = Owner::<T>::get( key.clone() ).clone();
        
        let active = Self::get_active_for_uid( netuid, uid as u16 );
        let rank = Self::get_rank_for_uid( netuid, uid as u16 );
        let emission = Self::get_emission_for_uid( netuid, uid as u16 );
        let incentive = Self::get_incentive_for_uid( netuid, uid as u16 );
        let consensus = Self::get_consensus_for_uid( netuid, uid as u16 );
        let trust = Self::get_trust_for_uid( netuid, uid as u16 );
        let dividends = Self::get_dividends_for_uid( netuid, uid as u16 );
        let pruning_score = Self::get_pruning_score_for_uid( netuid, uid as u16 );
        let last_update = Self::get_last_update_for_uid( netuid, uid as u16 );

        let weights = <Weights<T>>::get(netuid, uid).iter()
            .filter_map(|(i, w)| if *w > 0 { Some((i.into(), w.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let bonds = <Bonds<T>>::get(netuid, uid).iter()
            .filter_map(|(i, b)| if *b > 0 { Some((i.into(), b.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let stake: Vec<(T::AccountId, Compact<u64>)> = < Stake<T> as IterableStorageDoubleMap<T::AccountId, T::AccountId, u64> >::iter_prefix( key.clone() )
            .map(|(coldkey, stake)| (coldkey, stake.into()))
            .collect();

        let module = ModuleInfo {
            key: key.clone(),
            uid: uid.into(),
            netuid: netuid.into(),
            active,
            module_info,
            prometheus_info,
            stake,
            rank: rank.into(),
            emission: emission.into(),
            incentive: incentive.into(),
            consensus: consensus.into(),
            trust: trust.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            weights,
            bonds,
            pruning_score: pruning_score.into()
        };
        
        return Some(module);
    }

    pub fn get_module(netuid: u16, uid: u16) -> Option<ModuleInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return None;
        }

        let module = Self::get_module_subnet_exists(netuid, uid);
        module
	}

    fn get_module_lite_subnet_exists(netuid: u16, uid: u16) -> Option<ModuleInfoLite<T>> {
        let _key = Self::get_key_for_net_and_uid(netuid, uid);
        let key;
        if _key.is_err() {
            return None;
        } else {
            // No error, key was registered
            key = _key.expect("Hotkey should exist");
        }

        let module_info = Self::get_module_info( netuid, &key.clone() );

        let prometheus_info = Self::get_prometheus_info( netuid, &key.clone() );

        
        let coldkey = Owner::<T>::get( key.clone() ).clone();
        
        let active = Self::get_active_for_uid( netuid, uid as u16 );
        let rank = Self::get_rank_for_uid( netuid, uid as u16 );
        let emission = Self::get_emission_for_uid( netuid, uid as u16 );
        let incentive = Self::get_incentive_for_uid( netuid, uid as u16 );
        let consensus = Self::get_consensus_for_uid( netuid, uid as u16 );
        let trust = Self::get_trust_for_uid( netuid, uid as u16 );
        let dividends = Self::get_dividends_for_uid( netuid, uid as u16 );
        let pruning_score = Self::get_pruning_score_for_uid( netuid, uid as u16 );
        let last_update = Self::get_last_update_for_uid( netuid, uid as u16 );

        let stake: Vec<(T::AccountId, Compact<u64>)> = < Stake<T> as IterableStorageDoubleMap<T::AccountId, T::AccountId, u64> >::iter_prefix( key.clone() )
            .map(|(coldkey, stake)| (coldkey, stake.into()))
            .collect();

        let module = ModuleInfoLite {
            key: key.clone(),
            coldkey: coldkey.clone(),
            uid: uid.into(),
            netuid: netuid.into(),
            active,
            module_info,
            prometheus_info,
            stake,
            rank: rank.into(),
            emission: emission.into(),
            incentive: incentive.into(),
            consensus: consensus.into(),
            trust: trust.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            pruning_score: pruning_score.into()
        };
        
        return Some(module);
    }    

    pub fn get_modules_lite(netuid: u16) -> Vec<ModuleInfoLite<T>> {
         if !Self::if_subnet_exist(netuid) {
            return Vec::new();
        }

        let mut modules: Vec<ModuleInfoLite<T>> = Vec::new();
        let n = Self::get_subnetwork_n(netuid);
        for uid in 0..n {
            let uid = uid;

            let _module = Self::get_module_lite_subnet_exists(netuid, uid);
            let module;
            if _module.is_none() {
                break; // No more modules
            } else {
                // No error, key was registered
                module = _module.expect("Module should exist");
            }

            modules.push( module );
        }
        modules 
    }

    pub fn get_module_lite(netuid: u16, uid: u16) -> Option<ModuleInfoLite<T>> {
        if !Self::if_subnet_exist(netuid) {
            return None;
        }

        let module = Self::get_module_lite_subnet_exists(netuid, uid);
        module
   }
}

