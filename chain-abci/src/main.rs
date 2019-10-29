mod app;
mod enclave_bridge;
mod liveness;
mod punishment;
mod slashing;
mod storage;

use abci::Application;
use abci::*;
use bit_vec::BitVec;
use chain_abci::app::*;
use chain_abci::enclave_bridge::mock::MockClient;
use chain_abci::storage::account::AccountStorage;
use chain_abci::storage::account::AccountWrapper;
use chain_abci::storage::tx::StarlingFixedKey;
use chain_abci::storage::*;
use chain_core::common::{MerkleTree, Proof, H256, HASH_SIZE_256};
use chain_core::compute_app_hash;
use chain_core::init::address::RedeemAddress;
use chain_core::init::coin::Coin;
use chain_core::init::config::InitConfig;
use chain_core::init::config::InitNetworkParameters;
use chain_core::init::config::StakedStateDestination;
use chain_core::init::config::{
    JailingParameters, SlashRatio, SlashingParameters, ValidatorKeyType, ValidatorPubkey,
};
use chain_core::state::account::{
    to_stake_key, DepositBondTx, StakedState, StakedStateAddress, StakedStateOpAttributes,
    StakedStateOpWitness, UnbondTx, WithdrawUnbondedTx,
};
use chain_core::state::tendermint::TendermintVotePower;
use chain_core::state::RewardsPoolState;
use chain_core::tx::fee::{LinearFee, Milli};
use chain_core::tx::witness::tree::RawPubkey;
use chain_core::tx::witness::EcdsaSignature;
use chain_core::tx::PlainTxAux;
use chain_core::tx::TransactionId;
use chain_core::tx::TxObfuscated;
use chain_core::tx::{
    data::{
        access::{TxAccess, TxAccessPolicy},
        address::ExtendedAddr,
        attribute::TxAttributes,
        input::{TxoIndex, TxoPointer},
        output::TxOut,
        txid_hash, Tx, TxId,
    },
    witness::{TxInWitness, TxWitness},
    TxAux, TxEnclaveAux,
};
use chain_tx_filter::BlockFilter;
use chain_tx_validation::TxWithOutputs;
use hex::decode;
use kvdb::KeyValueDB;
use kvdb_memorydb::create;
use parity_scale_codec::{Decode, Encode};
use secp256k1::schnorrsig::schnorr_sign;
use secp256k1::{key::PublicKey, key::SecretKey, Message, Secp256k1, Signing};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::Arc;

fn main() {
    println!("ok");

        let init_network_params = InitNetworkParameters {
        initial_fee_policy: LinearFee::new(Milli::new(0, 0), Milli::new(0, 0)),
        required_council_node_stake: Coin::max(),
        unbonding_period: 1,
        jailing_config: JailingParameters {
            jail_duration: 60,
            block_signing_window: 5,
            missed_block_threshold: 1,
        },
        slashing_config: SlashingParameters {
            liveness_slash_percent: SlashRatio::from_str("0.1").unwrap(),
            byzantine_slash_percent: SlashRatio::from_str("0.2").unwrap(),
            slash_wait_period: 5,
        },
    };
    let mut nodes = BTreeMap::new();
    let node_pubkey = ValidatorPubkey {
        consensus_pubkey_type: ValidatorKeyType::Ed25519,
        consensus_pubkey_b64: "EIosObgfONUsnWCBGRpFlRFq5lSxjGIChRlVrVWVkcE=".to_string(),
    };

      let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&[0xcd; 32]).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let address = RedeemAddress::from(&public_key);
    let staking_account_address = StakedStateAddress::BasicRedeem(address);
    nodes.insert(address, node_pubkey);


 let rewards_pool = Coin::zero();
    let mut distribution = BTreeMap::new();
    distribution.insert(address, (StakedStateDestination::Bonded, Coin::max()));
    let init_config = InitConfig::new(rewards_pool, distribution, init_network_params, nodes);
    let m= serde_json::to_string_pretty(&init_config).unwrap();
    println!("{}", m);
}