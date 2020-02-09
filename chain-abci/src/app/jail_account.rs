use chain_core::state::account::{PunishmentKind, StakedStateAddress};
use chain_tx_validation::Error;

use crate::app::ChainNodeApp;
use crate::enclave_bridge::EnclaveProxy;
use crate::storage::get_account;
use chain_storage::account::update_staked_state;

impl<T: EnclaveProxy> ChainNodeApp<T> {
    /// Jails staking account with given address
    /// TODO: isolate from ChainNodeApp
    pub fn jail_account(
        &mut self,
        staking_address: StakedStateAddress,
        punishment_kind: PunishmentKind,
    ) -> Result<(), Error> {
        let mut account = get_account(
            &staking_address,
            &self.uncommitted_account_root_hash,
            &self.accounts,
        )?;

        if account.is_jailed() {
            // Return early if account is already jailed
            return Ok(());
        }

        let last_state = self
            .last_state
            .as_mut()
            .expect("Last state not found. Init chain was not called.");

        let block_time = last_state.block_time;
        let jail_duration = last_state.top_level.network_params.get_jail_duration();

        account.jail_until(block_time + jail_duration, punishment_kind);

        let (new_root, _) = update_staked_state(
            account,
            &self.uncommitted_account_root_hash,
            &mut self.accounts,
        );
        self.uncommitted_account_root_hash = new_root;
        last_state
            .validators
            .validator_state_helper
            .punish_update(staking_address);

        Ok(())
    }
}
