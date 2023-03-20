use super::*;

impl<T: Config> Pallet<T> { 

    // ---- The implementation for the extrinsic become_delegate: signals that this key allows delegated stake.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's coldkey.
    //
    // 	* 'key' (T::AccountId):
    // 		- The key we are delegating (must be owned by the coldkey.)
    //
    // 	* 'take' (u16):
    // 		- The stake proportion that this key takes from delegations.
    //
    // # Event:
    // 	* DelegateAdded;
    // 		- On successfully setting a key as a delegate.
    //
    // # Raises:
    // 	* 'NotRegistered':
    // 		- The key we are delegating is not registered on the network.
    //
    // 	* 'NonAssociatedColdKey':
    // 		- The key we are delegating is not owned by the calling coldket.
    //
	// 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
	pub fn do_become_delegate(
        origin: T::RuntimeOrigin, 
        key: T::AccountId, 
        take: u16
    ) -> dispatch::DispatchResult {
        // --- 1. We check the coldkey signuture.
        let coldkey = ensure_signed( origin )?;
        log::info!("do_become_delegate( origin:{:?} key:{:?}, take:{:?} )", coldkey, key, take );

        // --- 2. Ensure we are delegating an known key.
        ensure!( Self::key_account_exists( &key ), Error::<T>::NotRegistered );    
  
        // --- 3. Ensure that the coldkey is the owner.
        ensure!( Self::coldkey_owns_key( &coldkey, &key ), Error::<T>::NonAssociatedColdKey );

        // --- 4. Ensure we are not already a delegate (dont allow changing delegate take.)
        ensure!( !Self::key_is_delegate( &key ), Error::<T>::AlreadyDelegate );

		// --- 5. Ensure we don't exceed tx rate limit
		ensure!( !Self::exceeds_tx_rate_limit( Self::get_last_tx_block(&coldkey), Self::get_current_block_as_u64() ), Error::<T>::TxRateLimitExceeded );

        // --- 6. Delegate the key.
        Self::delegate_key( &key, take );
      
        // --- 7. Emit the staking event.
        log::info!("DelegateAdded( coldkey:{:?}, key:{:?}, take:{:?} )", coldkey, key, take );
        Self::deposit_event( Event::DelegateAdded( coldkey, key, take ) );

        // --- 8. Ok and return.
        Ok(())
    }

    // ---- The implementation for the extrinsic add_stake: Adds stake to a key account.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's coldkey.
    //
    // 	* 'key' (T::AccountId):
    // 		- The associated key account.
    //
    // 	* 'amount' (u64):
    // 		- The amount of stake to be added to the key staking account.
    //
    // # Event:
    // 	* StakeAdded;
    // 		- On the successfully adding stake to a global account.
    //
    // # Raises:
    // 	* 'CouldNotConvertToBalance':
    // 		- Unable to convert the passed stake value to a balance.
    //
    // 	* 'NotEnoughBalanceToStake':
    // 		- Not enough balance on the coldkey to add onto the global account.
    //
    // 	* 'NonAssociatedColdKey':
    // 		- The calling coldkey is not associated with this key.
    //
    // 	* 'BalanceWithdrawalError':
    // 		- Errors stemming from transaction pallet.
    //
	// 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
	pub fn do_add_stake(
        origin: T::RuntimeOrigin, 
        amount: u64
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the T::AccountId coldkey information.
        let key = ensure_signed( origin )?;
        log::info!("do_add_stake( origin:{:?}, amount:{:?} )", key, amount );

        // --- 2. We convert the stake u64 into a balancer.
        let stake_as_balance = Self::u64_to_balance( amount );
        ensure!( stake_as_balance.is_some(), Error::<T>::CouldNotConvertToBalance );
 
        // --- 3. Ensure the callers coldkey has enough stake to perform the transaction.
        ensure!( Self::can_remove_balance_from_account( &key, stake_as_balance.unwrap() ), Error::<T>::NotEnoughBalanceToStake );

        // --- 4. Ensure that the key account exists this is only possible through registration.
        ensure!( Self::key_account_exists( &key ), Error::<T>::NotRegistered );    

        // --- 6. Ensure the remove operation from the coldkey is a success.
        ensure!( Self::remove_balance_from_account( &key, stake_as_balance.unwrap() ) == true, Error::<T>::BalanceWithdrawalError );

		// --- 7. Ensure we don't exceed tx rate limit
		ensure!( !Self::exceeds_tx_rate_limit( Self::get_last_tx_block(&key), Self::get_current_block_as_u64() ), Error::<T>::TxRateLimitExceeded );

        // --- 8. If we reach here, add the balance to the key.
        Self::increase_stake_on_account( &key, amount );
 
        // --- 9. Emit the staking event.
        log::info!("StakeAdded( key:{:?}, amount:{:?} )", key, amount );
        Self::deposit_event( Event::StakeAdded( key, amount ) );

        // --- 10. Ok and return.
        Ok(())
    }

    // ---- The implementation for the extrinsic remove_stake: Removes stake from a key account and adds it onto a coldkey.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's coldkey.
    //
    // 	* 'key' (T::AccountId):
    // 		- The associated key account.
    //
    // 	* 'amount' (u64):
    // 		- The amount of stake to be added to the key staking account.
    //
    // # Event:
    // 	* StakeRemoved;
    // 		- On the successfully removing stake from the key account.
    //
    // # Raises:
    // 	* 'NotRegistered':
    // 		- Thrown if the account we are attempting to unstake from is non existent.
    //
    // 	* 'NonAssociatedColdKey':
    // 		- Thrown if the coldkey does not own the key we are unstaking from.
    //
    // 	* 'NotEnoughStaketoWithdraw':
    // 		- Thrown if there is not enough stake on the key to withdwraw this amount. 
    //
    // 	* 'CouldNotConvertToBalance':
    // 		- Thrown if we could not convert this amount to a balance.
	//
    // 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
    //
    pub fn do_remove_stake(
        origin: T::RuntimeOrigin, 
        amount: u64
    ) -> dispatch::DispatchResult {

        // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId coldkey information.
        let key = ensure_signed( origin )?;
        log::info!("do_remove_stake( key:{:?}, amount:{:?} )", key, amount );

        // --- 2. Ensure that the key account exists this is only possible through registration.
        ensure!( Self::key_account_exists( &key ), Error::<T>::NotRegistered );    

        // --- 3. Ensure that the key allows delegation or that the key is owned by the calling coldkey.
        ensure!( Self::key_is_delegate( &key ) || Self::coldkey_owns_key( &coldkey, &key ), Error::<T>::NonAssociatedColdKey );

        // --- 4. Ensure that the key has enough stake to withdraw.
        ensure!( Self::has_enough_stake( &coldkey, &key, amount ), Error::<T>::NotEnoughStaketoWithdraw );

        // --- 5. Ensure that we can conver this u64 to a balance.
        let amount_as_currency = Self::u64_to_balance( amount );
        ensure!( amount_as_currency.is_some(), Error::<T>::CouldNotConvertToBalance );

		// --- 6. Ensure we don't exceed tx rate limit
		ensure!( !Self::exceeds_tx_rate_limit( Self::get_last_tx_block(&coldkey), Self::get_current_block_as_u64() ), Error::<T>::TxRateLimitExceeded );

        // --- 7. We remove the balance from the key.
        Self::decrease_stake_on_key_account( &coldkey, &key, amount );

        // --- 8. We add the balancer to the coldkey.  If the above fails we will not credit this coldkey.
        Self::add_balance_to_account( &coldkey, amount_as_currency.unwrap() );

        // --- 9. Emit the unstaking event.
        log::info!("StakeRemoved( key:{:?}, amount:{:?} )", key, amount );
        Self::deposit_event( Event::StakeRemoved( key, amount ) );

        // --- 10. Done and ok.
        Ok(())
    }

    // Returns true if the passed key allow delegative staking. 
    //
    pub fn key_is_delegate( key: &T::AccountId ) -> bool {
		return Delegates::<T>::contains_key( key );
    }

    // Sets the key as a delegate with take.
    //
    pub fn delegate_key( key: &T::AccountId, take: u16 ) {
        Delegates::<T>::insert( key, take );
    }

    // Returns the total amount of stake in the staking table.
    //
    pub fn get_total_stake() -> u64 { 
        return TotalStake::<T>::get();
    }

    // Increases the total amount of stake by the passed amount.
    //
    pub fn increase_total_stake( increment: u64 ) { 
        TotalStake::<T>::put( Self::get_total_stake().saturating_add( increment ) );
    }

    // Decreases the total amount of stake by the passed amount.
    //
    pub fn decrease_total_stake( decrement: u64 ) { 
        TotalStake::<T>::put( Self::get_total_stake().saturating_sub( decrement ) );
    }

    // Returns the total amount of stake under a key (delegative or otherwise)
    //
    pub fn get_total_stake_for_key( key: &T::AccountId ) -> u64 { 
        return TotalHotkeyStake::<T>::get( key ); 
    }

    // Returns the total amount of stake held by the coldkey (delegative or otherwise)
    //
     pub fn get_total_stake_for_coldkey( coldkey: &T::AccountId ) -> u64 { 
         return TotalColdkeyStake::<T>::get( coldkey ); 
     }

    // Returns the stake under the cold - hot pairing in the staking table.
    //
    pub fn get_stake_for_and_key( coldkey: &T::AccountId, key: &T::AccountId ) -> u64 { 
        return Stake::<T>::get( key, coldkey );
    }

    // Creates a cold - hot pairing account if the key is not already an active account.
    //
    pub fn create_account_if_non_existent( coldkey: &T::AccountId, key: &T::AccountId ) {
        if !Self::key_account_exists( key ) {
            Stake::<T>::insert( key, coldkey, 0 ); 
            Owner::<T>::insert( key, coldkey );
        }
    }

    // Returns the coldkey owning this key. This function should only be called for active accounts.
    //
    pub fn get_owning_for_key( key: &T::AccountId ) ->  T::AccountId { 
        return Owner::<T>::get( key );
    }

    // Returns true if the key account has been created.
    //
    pub fn key_account_exists( key: &T::AccountId ) -> bool {
		return Owner::<T>::contains_key( key );
    }

    // Return true if the passed coldkey owns the key. 
    //
    pub fn coldkey_owns_key( coldkey: &T::AccountId, key: &T::AccountId ) -> bool {
        if Self::key_account_exists( key ){
		    return Owner::<T>::get( key ) == *coldkey;
        } else {
            return false;
        }
    }

    // Returns true if the cold-hot staking account has enough balance to fufil the decrement.
    //
    pub fn has_enough_stake( key: &T::AccountId, decrement: u64 ) -> bool {
        return Self::get_stake_key( key ) >= decrement;
    }



    // Decreases the stake on the key account under its owning coldkey.
    //
    pub fn decrease_stake_on_key_account( key: &T::AccountId, decrement: u64 ){
        Self::decrease_stake_on_coldkey_key_account( &Self::get_owning_coldkey_for_key( key ), key, decrement );
    }

    // Increases the stake on the cold - hot pairing by increment while also incrementing other counters.
    // This function should be called rather than set_stake under account.
    // 
    pub fn increase_stake_on_account(  key: &T::AccountId, increment: u64 ){
        Stake::<T>::insert( key, coldkey, Stake::<T>::get( key, coldkey).saturating_add( increment ) );
        TotalStake::<T>::put( TotalStake::<T>::get().saturating_add( increment ) );
        TotalIssuance::<T>::put( TotalIssuance::<T>::get().saturating_add( increment ) );

    }

    // Decreases the stake on the cold - hot pairing by the decrement while decreasing other counters.
    //
    pub fn decrease_stake_on_account( coldkey: &T::AccountId, key: &T::AccountId, decrement: u64 ){
        Stake::<T>::insert( key, coldkey, Stake::<T>::get( key, coldkey).saturating_sub( decrement ) );
        TotalStake::<T>::put( TotalStake::<T>::get().saturating_sub( decrement ) );
        TotalIssuance::<T>::put( TotalIssuance::<T>::get().saturating_sub( decrement ) );
    }

	pub fn u64_to_balance( input: u64 ) -> Option<<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance> { input.try_into().ok() }

    pub fn add_balance_to_key_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::deposit_creating(&key, amount); // Infallibe
    }

    pub fn set_balance_on_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::make_free_balance_be(&key, amount); 
    }

    pub fn can_remove_balance_from_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        let current_balance = Self::get_key_balance(key);
        if amount > current_balance {
            return false;
        }

        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw = T::Currency::ensure_can_withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), new_potential_balance).is_ok();
        can_withdraw
    }

    pub fn get_balance(key: &T::AccountId) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&key);
    }


    pub fn remove_balance_from_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        return match T::Currency::withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), ExistenceRequirement::KeepAlive) {
            Ok(_result) => {
                true
            }
            Err(_error) => {
                false
            }
        };
    }

}