use crate::types::get_string;
use crate::types::CroStakedState;
use crate::types::{CroAddressPtr, CroResult, CroUtxo};
use chain_core::common::H256;
use chain_core::init::coin::Coin;
pub use chain_core::init::network::Network;
use chain_core::state::account::{
    DepositBondTx, StakedState, StakedStateAddress, StakedStateOpAttributes, StakedStateOpWitness,
    UnbondTx, WithdrawUnbondedTx,
};
use chain_core::tx::data::access::{TxAccess, TxAccessPolicy};
use chain_core::tx::data::address::ExtendedAddr;
use chain_core::tx::data::attribute::TxAttributes;
use chain_core::tx::data::Tx;
use chain_core::tx::data::{input::TxoPointer, output::TxOut, TxId};
use chain_core::tx::fee::{LinearFee, Milli};
use chain_core::tx::witness::{TxInWitness, TxWitness};
use chain_core::tx::TransactionId;
use chain_core::tx::TxAux;
use client_common::tendermint::types::AbciQueryExt;
use client_common::tendermint::{Client, WebsocketRpcClient};
use client_common::MultiSigAddress;
use client_common::{PrivateKey, PublicKey, SignedTransaction};
use client_core::cipher::DefaultTransactionObfuscation;

use client_core::transaction_builder::WitnessedUTxO;
use client_core::unspent_transactions::{Operation, Sorter};
use client_core::{TransactionObfuscation, UnspentTransactions};

use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
use std::collections::BTreeSet;
use std::os::raw::c_char;
use std::ptr;
use std::str::FromStr;
use std::string::ToString;

/// # Safety
pub unsafe fn get_string_from_array(src: &[u8]) -> String {
    let mut n = 0;
    for i in 0..src.len() {
        if 0 == src[i] {
            n = i;
            break;
        }
    }

    std::str::from_utf8(&src[0..n]).expect("utf8").to_string()
}

#[no_mangle]
// utxo -> utxo
pub unsafe extern "C" fn cro_trasfer(
    network: u8,
    from_ptr: CroAddressPtr,
    return_address_user: *const c_char,
    spend_utxo: *const CroUtxo, //  from  (will be spent utxo)
    spend_utxo_count: u32,
    utxo: *const CroUtxo, // to
    utxo_count: u32,
    viewkeys: *const *const c_char, // viewkeys
    viewkey_count: i32,
) -> CroResult {
    let from_address = from_ptr.as_mut().expect("get address");
    let from_private = &from_address.privatekey;
    let from_public = &from_address.publickey;

    let _return_address = ExtendedAddr::from_str(&get_string(return_address_user)).unwrap();
    let array: &[*const c_char] = std::slice::from_raw_parts(viewkeys, viewkey_count as usize);
    let _spend_utxo_array: &[CroUtxo] =
        std::slice::from_raw_parts(spend_utxo, spend_utxo_count as usize);
    let utxo_array: &[CroUtxo] = std::slice::from_raw_parts(utxo, utxo_count as usize);

    let mut access_policies = BTreeSet::new();
    for x in array {
        let a = get_string(*x);
        let view_key = a.trim();
        let publickey = PublicKey::from_str(view_key).unwrap();
        println!("get_string {}", a);
        access_policies.insert(TxAccessPolicy {
            view_key: publickey.into(),
            access: TxAccess::AllData,
        });
    }
    let _attributes = TxAttributes::new_with_access(network, access_policies.into_iter().collect());

    // build tx
    let _fee_algorithm = LinearFee::new(Milli::new(1, 1), Milli::new(1, 1));
    //let mut raw_tx_builder: RawTransferTransactionBuilder<LinearFee> =
    //RawTransferTransactionBuilder::new(attributes, fee_algorithm.clone());

    let mut unspent_transactions = UnspentTransactions::new(vec![(
        TxoPointer::new([0; 32], 0),
        TxOut::new(
            ExtendedAddr::from_str(
                "dcro1aj3tv4z40250v9v0aextlsq4pl9qzd7zezd3v6fc392ak00zhtds3d2wyl",
            )
            .unwrap(),
            Coin::new(500).unwrap(),
        ),
    )]);

    unspent_transactions.apply_all(&[Operation::Sort(Sorter::HighestValueFirst)]);
    let _fees = Coin::zero();
    let _tx_ins_witness: Vec<TxInWitness> = vec![];
    let mut tx = Tx::default();
    let tx_ins: &mut Vec<TxoPointer> = &mut tx.inputs;
    let tx_outs: &mut Vec<TxOut> = &mut tx.outputs;
    let _attributes: &mut TxAttributes = &mut tx.attributes;

    let mut spend_utxo: Vec<WitnessedUTxO> = vec![];
    for x in tx_ins {
        // search txout from TxoPointer
        let txout = TxOut {
            address: ExtendedAddr::from_str("").unwrap(),
            value: Coin::from_str("").unwrap(),
            valid_from: None,
        };
        let newone = WitnessedUTxO {
            prev_txo_pointer: x.clone(),
            prev_tx_out: txout.clone(),
            witness: None,
        };
        spend_utxo.push(newone);
    }
    assert!(spend_utxo.len() == tx.inputs.len());

    for x in utxo_array {
        let address = ExtendedAddr::OrTree(x.address.clone());
        let value = Coin::new(x.value).unwrap(); // carson unit
        let txout = TxOut {
            address,
            value,
            valid_from: None,
        };
        tx_outs.push(txout);
    }
    let txid: TxId = tx.id();
    let mut witness_vec: Vec<TxInWitness> = vec![];
    // sign
    for mut s in spend_utxo {
        let witness: TxInWitness = schnorr_sign(&txid, &s.prev_tx_out, &from_public, &from_private);
        s.witness = Some(witness.clone()); // signature
        witness_vec.push(witness);
    }

    let witness = TxWitness::from(witness_vec);
    let _signed_transaction = SignedTransaction::TransferTransaction(tx, witness);

    //let tx= raw_tx_builder.to_tx();

    //let witness = TxWitness::from(witness_vec);
    //let signed_transaction = SignedTransaction::TransferTransaction(tx, witness);

    /*
    if change_amount != Coin::zero() {
        raw_tx_builder.add_output(TxOut::new(return_address, change_amount));
    }*/

    CroResult::success()
}

fn schnorr_sign(
    message: &TxId,
    txout: &TxOut,
    public_key: &PublicKey,
    private_key: &PrivateKey,
) -> TxInWitness {
    let public_keys: Vec<PublicKey> = vec![public_key.clone()];
    let multi_sig_address =
        MultiSigAddress::new(public_keys.to_vec(), public_keys[0].clone(), 1).unwrap();
    let _root_hash: H256 = multi_sig_address.root_hash();

    let _signing_addr: &ExtendedAddr = &txout.address;
    let proof = multi_sig_address
        .generate_proof(public_keys.to_vec())
        .unwrap()
        .unwrap();
    let _multi_sig_address_extended: ExtendedAddr = multi_sig_address.into();

    TxInWitness::TreeSig(private_key.schnorr_sign(message).unwrap(), proof)
}

// utxo -> staked account
#[no_mangle]
pub unsafe extern "C" fn cro_deposit(
    network: u8,
    from_ptr: CroAddressPtr,
    to_address_user: *const std::os::raw::c_char,
    utxo: *const CroUtxo, // utxo address, and cro amount
    utxo_count: u32,
) -> CroResult {
    let from_address = from_ptr.as_mut().expect("get address");
    let from_private = &from_address.privatekey;
    let from_public = &from_address.publickey;
    let array: &[CroUtxo] = std::slice::from_raw_parts(utxo, utxo_count as usize);

    let inputs: Vec<TxoPointer> = vec![];
    //let to_address =  StakedStateAddress::BasicRedeem(RedeemAddress::from(&//from_address.publickey));
    let to_address_string = get_string(to_address_user);
    let to_address = StakedStateAddress::from_str(&to_address_string).unwrap();
    let attributes = StakedStateOpAttributes::new(network);
    let transaction = DepositBondTx::new(inputs, to_address, attributes);

    let _utxos: Vec<TxOut> = vec![];

    let message: TxId = transaction.id();
    println!("cro_deposit {}", array.len());
    let mut in_witness: Vec<TxInWitness> = vec![];
    for x in array {
        let address = ExtendedAddr::OrTree(x.address.clone());
        let value = Coin::new(x.value).unwrap();

        let txout = TxOut {
            address: address.clone(),
            value,
            valid_from: None,
        };
        println!("txid={} index={}", address, value);
        let tx_in_witness: TxInWitness =
            schnorr_sign(&message, &txout, &from_public, &from_private);
        in_witness.push(tx_in_witness);
    }
    let tx_witness: TxWitness = in_witness.into();

    // TxoPointer: txid, index  <- txout's location, which tx? which index?
    // TxOut: address, value
    //witness.0.push(value: T)

    let _signed_transaction: SignedTransaction =
        SignedTransaction::DepositStakeTransaction(transaction, tx_witness);
    // ready
    CroResult::success()
}

/// staked -> staked
#[no_mangle]
pub unsafe extern "C" fn cro_unbond(
    network: u8,
    nonce: u64,
    from_ptr: CroAddressPtr,
    to_address_user: *const std::os::raw::c_char,
    amount: u64,
    output: *mut u8,
    output_length: *mut u32,
) -> CroResult {
    let to_address = StakedStateAddress::from_str(&get_string(to_address_user)).unwrap();
    let value = Coin::new(amount).unwrap(); // carson unit
    println!("address {} {} cro  nonce={}", to_address, value, nonce);
    let attributes = StakedStateOpAttributes::new(network);
    let transaction: UnbondTx = UnbondTx::new(to_address, nonce, value, attributes);

    let from_address = from_ptr.as_mut().expect("get address");
    let from_private = &from_address.privatekey;
    let _from_public = &from_address.publickey;
    let signature: StakedStateOpWitness = from_private
        .sign(transaction.id())
        .map(StakedStateOpWitness::new)
        .unwrap();

    let result = TxAux::UnbondStakeTx(transaction, signature);
    let encoded = result.encode();
    ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
    (*output_length) = encoded.len() as u32;

    CroResult::success()
}

/// staked -> utxo
#[no_mangle]
pub unsafe extern "C" fn cro_withdraw(
    network: u8,
    from_ptr: CroAddressPtr,
    _to_user: *const c_char,
    viewkeys: *const *const c_char, // viewkeys
    viewkey_count: i32,
) -> CroResult {
    let from_address = from_ptr.as_mut().expect("get address");
    let from_private = &from_address.privatekey;
    let _from_public = &from_address.publickey;

    let array: &[*const c_char] = std::slice::from_raw_parts(viewkeys, viewkey_count as usize);

    let mut access_policies = BTreeSet::new();
    for x in array {
        let a = get_string(*x);
        let view_key = a.trim();
        let publickey = PublicKey::from_str(view_key).unwrap();
        println!("get_string {}", a);
        access_policies.insert(TxAccessPolicy {
            view_key: publickey.into(),
            access: TxAccess::AllData,
        });
    }
    let attributes = TxAttributes::new_with_access(network, access_policies.into_iter().collect());

    let nonce = 0;
    let staked_state = StakedState::default();
    let outputs: Vec<TxOut> = vec![];
    let transaction = WithdrawUnbondedTx::new(nonce, outputs, attributes);
    let signature = from_private
        .sign(transaction.id())
        .map(StakedStateOpWitness::new)
        .unwrap();

    let _signed_transaction = SignedTransaction::WithdrawUnbondedStakeTransaction(
        transaction,
        Box::new(staked_state),
        signature,
    );

    println!("viewcount {}", viewkey_count);
    CroResult::success()
}

/// staked -> utxo
/// tendermint_url: ws://localhost:26657/websocket
#[no_mangle]
pub unsafe extern "C" fn cro_get_staked_state(
    from_ptr: CroAddressPtr,
    tenermint_url_string: *const c_char,
    staked_state_user: *mut CroStakedState,
) -> CroResult {
    let staked_state = staked_state_user.as_mut().expect("get state");
    let from_address = from_ptr.as_mut().expect("get address");
    let tendermint_url = get_string(tenermint_url_string);
    println!("get staked state");

    let tendermint_client = WebsocketRpcClient::new(&tendermint_url).unwrap();
    let result = tendermint_client
        .query("txquery", &[])
        .unwrap()
        .bytes()
        .unwrap();
    let address = std::str::from_utf8(&result).unwrap();
    let address_args: Vec<&str> = address.split(':').collect();
    //println!("txquery host={} port={}", address_args[0], address_args[1]);
    let _transaction_obfuscation: DefaultTransactionObfuscation =
        DefaultTransactionObfuscation::new(
            address_args[0].to_string(),
            address_args[1].to_string(),
        );

    let tendermint_client = WebsocketRpcClient::new(&tendermint_url).unwrap();

    assert!(20 == from_address.raw.len());
    let bytes = tendermint_client
        .query("account", &from_address.raw)
        .unwrap()
        .bytes()
        .unwrap();
    let state = StakedState::decode(&mut bytes.as_slice()).unwrap();
    staked_state.nonce = state.nonce;
    staked_state.unbonded_from = state.unbonded_from;
    staked_state.bonded = state.bonded.into();
    staked_state.unbonded = state.unbonded.into();
    println!("staked state={:?}", state);

    CroResult::success()
}

/// tendermint_url_string: default "ws://localhost:26657/websocket"
#[no_mangle]
pub unsafe extern "C" fn cro_encrypt(
    tenermint_url_string: *const c_char,
    signed_transaction_user: *const u8,
    signed_transaction_length: u32,
    output: *mut u8,
    output_length: *mut u32,
) -> CroResult {
    let mut signed_transaction_encoded =
        std::slice::from_raw_parts(signed_transaction_user, signed_transaction_length as usize);
    let tendermint_url = get_string(tenermint_url_string);
    let tendermint_client = WebsocketRpcClient::new(&tendermint_url).unwrap();
    let result = tendermint_client
        .query("txquery", &[])
        .unwrap()
        .bytes()
        .unwrap();
    let address = std::str::from_utf8(&result).unwrap();
    let address_args: Vec<&str> = address.split(':').collect();
    println!("txquery host={} port={}", address_args[0], address_args[1]);
    let signed_transaction: SignedTransaction =
        SignedTransaction::decode(&mut signed_transaction_encoded).unwrap();
    let transaction_obfuscation: DefaultTransactionObfuscation = DefaultTransactionObfuscation::new(
        address_args[0].to_string(),
        address_args[1].to_string(),
    );
    let txaux = transaction_obfuscation.encrypt(signed_transaction).unwrap();
    let encoded: Vec<u8> = txaux.encode();
    ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
    (*output_length) = encoded.len() as u32;

    CroResult::success()
}

/// staked -> utxo
/// tendermint_url: ws://localhost:26657/websocket
#[no_mangle]
pub unsafe extern "C" fn cro_broadcast(
    tenermint_url_string: *const c_char,
    user_data: *const u8,
    data_length: u32,
) -> CroResult {
    let tendermint_url = get_string(tenermint_url_string);
    let tendermint_client = WebsocketRpcClient::new(&tendermint_url).unwrap();
    let data: &[u8] = std::slice::from_raw_parts(user_data, data_length as usize);
    let result = tendermint_client.broadcast_transaction(data).unwrap();
    println!("result={}", serde_json::to_string(&result).unwrap());
    CroResult::success()
}
