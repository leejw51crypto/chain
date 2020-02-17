use super::ChainNodeApp;
use crate::enclave_bridge::EnclaveProxy;
use abci::*;
use chain_core::common::MerkleTree;
use chain_core::compute_app_hash;
use chain_core::tx::data::input::{TxoIndex, TxoPointer};
use chain_core::tx::data::TxId;
use chain_core::tx::{TxAux, TxEnclaveAux};
use chain_core::ChainInfo;
use chain_storage::Storage;
use enclave_protocol::{EnclaveRequest, EnclaveResponse};
use log::debug;
use parity_scale_codec::Encode;

/// Given a db and a DB transaction, it will go through TX inputs and mark them as spent
/// in the TX_META storage and it will create a new entry for TX in TX_META with all outputs marked as unspent.
pub fn update_utxos_commit(
    inputs: &[TxoPointer],
    no_of_outputs: TxoIndex,
    txid: TxId,
    storage: &mut Storage,
) {
    storage.spend_utxos(&inputs);
    storage.create_utxo(no_of_outputs, &txid);
}

impl<T: EnclaveProxy> ChainNodeApp<T> {
    pub fn process_txs(&mut self) {
        for txaux in self.delivered_txs.iter() {
            let txid: TxId = txaux.tx_id();
            match &txaux {
                TxAux::EnclaveTx(TxEnclaveAux::TransferTx {
                    inputs,
                    no_of_outputs,
                    ..
                }) => {
                    update_utxos_commit(&inputs, *no_of_outputs, txid, &mut self.storage);
                }
                TxAux::EnclaveTx(TxEnclaveAux::DepositStakeTx { tx, .. }) => {
                    self.storage.store_tx_body(&txid, &tx.encode());
                    // witness is obfuscated -- TODO: could be stored on the enclave side or thrown away?
                    // this is not necessary (as they are spent in deliver_tx) and more of a sanity check (as update_utxos_commit does it)
                    self.storage.spend_utxos(&tx.inputs);
                    // account should be already updated in deliver_tx
                }
                TxAux::UnbondStakeTx(tx, witness) => {
                    self.storage.store_tx_body(&txid, &tx.encode());
                    self.storage.store_tx_witness(&txid, &witness.encode());
                    // account should be already updated in deliver_tx
                }
                TxAux::EnclaveTx(TxEnclaveAux::WithdrawUnbondedStakeTx {
                    witness,
                    no_of_outputs,
                    ..
                }) => {
                    self.storage.store_tx_witness(&txid, &witness.encode());
                    // account should be already updated in deliver_tx
                    self.storage.create_utxo(*no_of_outputs, &txid);
                }
                TxAux::UnjailTx(tx, witness) => {
                    self.storage.store_tx_body(&txid, &tx.encode());
                    self.storage.store_tx_witness(&txid, &witness.encode());
                    // account should be already unjailed in deliver_tx
                }
                TxAux::NodeJoinTx(tx, witness) => {
                    self.storage.store_tx_body(&txid, &tx.encode());
                    self.storage.store_tx_witness(&txid, &witness.encode());
                    // staked state updated in deliver_tx
                    // validator state updated in end_block
                }
            }
        }
    }
    /// Commits delivered TX: flushes updates to the underlying storage
    pub fn commit_handler(&mut self, _req: &RequestCommit) -> ResponseCommit {
        let orig_state = self.last_state.clone();
        let mut new_state = orig_state.expect("executing block commit, but no app state stored (i.e. no initchain or recovery was executed)");
        let mut top_level = &mut new_state.top_level;
        let mut resp = ResponseCommit::new();

        let ids: Vec<TxId> = self
            .delivered_txs
            .iter()
            .map(chain_core::tx::TxAux::tx_id)
            .collect();
        let tree = MerkleTree::new(ids);

        if !self.delivered_txs.is_empty() {
            self.process_txs();
        }
        if self.rewards_pool_updated {
            top_level.rewards_pool.last_block_height = new_state.last_block_height;
            self.rewards_pool_updated = false;
        }
        top_level.account_root = self.uncommitted_account_root_hash;
        let app_hash = compute_app_hash(
            &tree,
            &top_level.account_root,
            &top_level.rewards_pool,
            &top_level.network_params,
        );
        self.storage
            .store_txs_merkle_tree(&app_hash, &tree.encode());
        new_state.last_apphash = app_hash;
        match self
            .tx_validator
            .process_request(EnclaveRequest::CommitBlock {
                app_hash,
                info: ChainInfo {
                    // TODO: fee computation in enclave?
                    min_fee_computed: top_level.network_params.calculate_fee(0).expect("base fee"),
                    chain_hex_id: self.chain_hex_id,
                    previous_block_time: new_state.block_time,
                    unbonding_period: top_level.network_params.get_unbonding_period(),
                },
            }) {
            EnclaveResponse::CommitBlock(Ok(_)) => {
                debug!("enclave storage persisted");
            }
            _ => {
                panic!("persisting enclave storage failed");
            }
        }
        self.storage.store_chain_state(
            &new_state,
            new_state.last_block_height,
            self.tx_query_address.is_some(),
        );

        let wr = self.storage.persist_write();
        if let Err(e) = wr {
            panic!("db write error: {}", e);
        } else {
            resp.data = new_state.last_apphash.to_vec();
            self.last_state = Some(new_state);
            self.delivered_txs.clear();
        }

        resp
    }
}
