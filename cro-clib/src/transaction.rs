use chain_core::state::account::{
    CouncilNode, DepositBondTx, StakedState, StakedStateAddress, StakedStateOpAttributes,
    StakedStateOpWitness, UnbondTx, UnjailTx, WithdrawUnbondedTx,
};
use chain_core::tx::data::attribute::TxAttributes;
use chain_core::tx::data::Tx;
use client_common::{
    seckey::derive_enckey, Error, ErrorKind, PrivateKey, PublicKey, Result, ResultExt, SecKey,
    SignedTransaction, Storage, Transaction, TransactionInfo,
};
use std::str::FromStr;

use crate::types::get_string;
use crate::types::{CroAddressPtr, CroResult, CroString, CroTxOut, CroUtxo};
use chain_core::common::{Proof, H256};
pub use chain_core::init::network::Network;
use chain_core::init::{address::RedeemAddress, coin::Coin, config::InitConfig};
use chain_core::tx::data::access::{TxAccess, TxAccessPolicy};
use chain_core::tx::data::address::ExtendedAddr;
use chain_core::tx::fee::{LinearFee, Milli};
use chain_core::tx::witness::{TxInWitness, TxWitness};
use chain_core::tx::TransactionId;
use chain_core::tx::TxAux;
use chain_core::{
    init::coin::{sum_coins, CoinError},
    tx::data::{input::TxoPointer, output::TxOut, TxId},
};
use client_common::MultiSigAddress;
use client_core::transaction_builder::RawTransferTransactionBuilder;
use client_core::transaction_builder::WitnessedUTxO;
use client_core::unspent_transactions::{Operation, Sorter};
use client_core::{TransactionObfuscation, UnspentTransactions, WalletClient};
use std::collections::BTreeSet;
use std::os::raw::c_char;
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

    let return_address = ExtendedAddr::from_str(&get_string(return_address_user)).unwrap();
    let array: &[*const c_char] = std::slice::from_raw_parts(viewkeys, viewkey_count as usize);
    let spend_utxo_array: &[CroUtxo] =
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
    let attributes = TxAttributes::new_with_access(network, access_policies.into_iter().collect());

    // build tx
    let fee_algorithm = LinearFee::new(Milli::new(1, 1), Milli::new(1, 1));
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
    let mut fees = Coin::zero();
    let mut tx_ins_witness: Vec<TxInWitness> = vec![];
    let mut tx = Tx::default();
    let tx_ins: &mut Vec<TxoPointer> = &mut tx.inputs;
    let mut tx_outs: &mut Vec<TxOut> = &mut tx.outputs;
    let mut attributes: &mut TxAttributes = &mut tx.attributes;

    let mut spend_utxo: Vec<WitnessedUTxO> = vec![];
    for x in tx_ins {
        // search txout from TxoPointer
        let txout = TxOut {
            address: ExtendedAddr::from_str("").unwrap(),
            value: Coin::from_str("").unwrap(),
            valid_from: None,
        };
        let mut newone = WitnessedUTxO {
            prev_txo_pointer: x.clone(),
            prev_tx_out: txout.clone(),
            witness: None,
        };
        spend_utxo.push(newone);
    }
    assert!(spend_utxo.len() == tx.inputs.len());

    for x in utxo_array {
        let coin = crate::types::get_string_from_array(&x.coin);
        let addr = crate::types::get_string_from_array(&x.address);
        let address = ExtendedAddr::from_str(&addr).unwrap();
        let value = Coin::from_str(&coin).unwrap(); // carson unit
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
    let signed_transaction = SignedTransaction::TransferTransaction(tx, witness);

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
    let root_hash: H256 = multi_sig_address.root_hash();

    let signing_addr: &ExtendedAddr = &txout.address;
    let proof = multi_sig_address
        .generate_proof(public_keys.to_vec())
        .unwrap()
        .unwrap();
    let multi_sig_address_extended: ExtendedAddr = multi_sig_address.into();

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

    let mut utxos: Vec<TxOut> = vec![];

    let message: TxId = transaction.id();
    println!("cro_deposit {}", array.len());
    let mut in_witness: Vec<TxInWitness> = vec![];
    for x in array {
        let address = &x.address;

        let coin = crate::types::get_string_from_array(&x.coin);
        let addr = crate::types::get_string_from_array(&x.address);

        println!("******* {}", addr);
        let address = ExtendedAddr::from_str(&addr).unwrap();
        let value = Coin::from_str(&coin).unwrap(); // carson unit

        let txout = TxOut {
            address,
            value,
            valid_from: None,
        };
        println!("txid={} index={}", addr, coin);
        let tx_in_witness: TxInWitness =
            schnorr_sign(&message, &txout, &from_public, &from_private);
        in_witness.push(tx_in_witness);
    }
    let tx_witness: TxWitness = in_witness.into();

    // TxoPointer: txid, index  <- txout's location, which tx? which index?
    // TxOut: address, value
    //witness.0.push(value: T)

    let signed_transaction: SignedTransaction =
        SignedTransaction::DepositStakeTransaction(transaction, tx_witness);
    // ready
    CroResult::success()
}

/// staked -> staked
#[no_mangle]
pub unsafe extern "C" fn cro_unbond(
    network: u8,
    from_ptr: CroAddressPtr,
    to_address_user: *const std::os::raw::c_char,
    amount_user: *const std::os::raw::c_char,
) -> CroResult {
    let to_address = StakedStateAddress::from_str(&get_string(to_address_user)).unwrap();
    let value = Coin::from_str(&get_string(amount_user)).unwrap(); // carson unit
    println!("address {} {} cro", to_address, value);
    // get nonce from tendermint
    let nonce = 0;
    let attributes = StakedStateOpAttributes::new(network);
    let transaction: UnbondTx = UnbondTx::new(to_address, nonce, value, attributes);

    let from_address = from_ptr.as_mut().expect("get address");
    let from_private = &from_address.privatekey;
    let from_public = &from_address.publickey;
    let signature: StakedStateOpWitness = from_private
        .sign(transaction.id())
        .map(StakedStateOpWitness::new)
        .unwrap();

    let result = TxAux::UnbondStakeTx(transaction, signature);

    CroResult::success()
}

/// staked -> utxo
#[no_mangle]
pub unsafe extern "C" fn cro_withdraw(
    network: u8,
    from_ptr: CroAddressPtr,
    to_user: *const c_char,
    viewkeys: *const *const c_char, // viewkeys
    viewkey_count: i32,
) -> CroResult {
    let from_address = from_ptr.as_mut().expect("get address");
    let from_private = &from_address.privatekey;
    let from_public = &from_address.publickey;

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

    let signed_transaction = SignedTransaction::WithdrawUnbondedStakeTransaction(
        transaction,
        Box::new(staked_state),
        signature,
    );

    println!("viewcount {}", viewkey_count);
    CroResult::success()
}
