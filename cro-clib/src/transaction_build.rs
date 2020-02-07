use crate::types::get_string;
use crate::types::{CroAddress, CroAddressPtr, CroResult};
use crate::types::{CroTx, CroTxPtr};
use chain_core::init::coin::Coin;
pub use chain_core::init::network::Network;
use chain_core::tx::data::access::{TxAccess, TxAccessPolicy};
use chain_core::tx::data::address::ExtendedAddr;
use chain_core::tx::data::attribute::TxAttributes;
use chain_core::tx::data::input::TxoPointer;
use chain_core::tx::data::output::TxOut;
use chain_core::tx::data::Tx;
use chain_core::tx::data::TxId;
use chain_core::tx::witness::TxInWitness;
use chain_core::tx::TransactionId;
use client_common::MultiSigAddress;
use client_common::{ErrorKind, Result, ResultExt};
use client_common::{PrivateKey, PublicKey};
use parity_scale_codec::Encode;

use std::collections::BTreeSet;
use std::os::raw::c_char;
use std::ptr;
use std::str::FromStr;
/// create tx
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_create_tx(tx_out: *mut CroTxPtr) -> CroResult {
    let tx = CroTx::default();
    let tx_box = Box::new(tx);
    ptr::write(tx_out, Box::into_raw(tx_box));
    CroResult::success()
}

fn do_cro_tx_add_txin(
    tx: &mut CroTx,
    txid_hex: &str,
    txindex: u16,
    addr: &str,
    coin: u64,
) -> Result<()> {
    let txid = hex::decode(&txid_hex).chain(|| {
        (
            ErrorKind::DeserializationError,
            "Unable to decode hex of txid",
        )
    })?;
    assert!(32 == txid.len());

    let mut txid_bytes: [u8; 32] = [0; 32];
    txid_bytes.copy_from_slice(&txid[0..32]);
    let txin_pointer = TxoPointer::new(txid_bytes, txindex as usize);
    let txin = TxOut::new(
        ExtendedAddr::from_str(&addr).chain(|| {
            (
                ErrorKind::DeserializationError,
                "Unable to decode extended addr",
            )
        })?,
        Coin::new(coin).chain(|| (ErrorKind::DeserializationError, "Unable to decode coin"))?,
    );

    tx.txin_pointer.push(txin_pointer);
    tx.txin_witness.push(None);
    tx.txin.push(txin);
    Ok(())
}

/// add txin
/// txid_string: 64 length hex-char , 32 bytes
/// addr_string: transfer address
/// cro_string: cro unit , example 0.0001
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_add_txin(
    tx_ptr: CroTxPtr,
    txid_string: *const c_char,
    txindex: u16,
    addr_string: *const c_char,
    coin: u64,
) -> CroResult {
    let mut tx = tx_ptr.as_mut().expect("get tx");
    let txid_hex = get_string(txid_string);
    let addr = get_string(addr_string);
    match do_cro_tx_add_txin(&mut tx, &txid_hex, txindex, &addr, coin) {
        Ok(_) => CroResult::success(),
        Err(_) => CroResult::fail(),
    }
}

/// add txin in bytes
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_add_txin_raw(
    tx_ptr: CroTxPtr,
    txid: [u8; 32],
    txindex: u16,
    addr: [u8; 32],
    coin: u64,
) -> CroResult {
    let tx = tx_ptr.as_mut().expect("get tx");
    let txin_pointer = TxoPointer::new(txid, txindex as usize);
    let txin = TxOut::new(
        ExtendedAddr::OrTree(addr),
        Coin::new(coin).expect("get coin in cro_tx_add_txin"),
    );

    tx.txin_pointer.push(txin_pointer);
    tx.txin_witness.push(None);
    tx.txin.push(txin);
    CroResult::success()
}

/// add viewkey
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_add_viewkey(
    tx_ptr: CroTxPtr,
    viewkey_string: *const c_char,
) -> CroResult {
    let tx = tx_ptr.as_mut().expect("get tx");
    let viewkey = get_string(viewkey_string);
    let hex: Vec<u8>;
    if let Ok(value) = hex::decode(&viewkey) {
        hex = value;
    } else {
        return CroResult::fail();
    }
    assert!(33 == hex.len());
    let pubkey: secp256k1::PublicKey = secp256k1::PublicKey::from_slice(&hex[..]).unwrap();
    tx.viewkey.push(pubkey);
    CroResult::success()
}

/// add viewkey in bytes
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_add_viewkey_raw(tx_ptr: CroTxPtr, viewkey: [u8; 33]) -> CroResult {
    let tx = tx_ptr.as_mut().expect("get tx");
    let pubkey: secp256k1::PublicKey = secp256k1::PublicKey::from_slice(&viewkey).unwrap();
    tx.viewkey.push(pubkey);
    CroResult::success()
}

fn do_cro_tx_prepare_for_signing(user_tx: &mut CroTx, network: u8) -> Result<()> {
    let mut tx: Tx = Tx::default();
    tx.inputs = user_tx.txin_pointer.clone();
    tx.outputs = user_tx.txout.clone();
    let mut access_policies = BTreeSet::new();
    for viewkey_user in &user_tx.viewkey {
        access_policies.insert(TxAccessPolicy {
            view_key: viewkey_user.clone(),
            access: TxAccess::AllData,
        });
    }
    tx.attributes = TxAttributes::new_with_access(network, access_policies.into_iter().collect());
    user_tx.tx = Some(tx);
    Ok(())
}

/// prepare tx for signing
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_prepare_for_signing(tx_ptr: CroTxPtr, network: u8) -> CroResult {
    let mut user_tx: &mut CroTx = tx_ptr.as_mut().expect("get tx");
    match do_cro_tx_prepare_for_signing(&mut user_tx, network) {
        Ok(_) => CroResult::success(),
        Err(_) => CroResult::fail(),
    }
}

/// extract bytes from singed tx
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_complete_signing(
    tx_ptr: CroTxPtr,
    output: *mut u8,
    output_length: *mut u32,
) -> CroResult {
    let user_tx: &mut CroTx = tx_ptr.as_mut().expect("get tx");
    let encoded: Vec<u8> = user_tx.tx.as_ref().expect("get tx core").encode();
    ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
    (*output_length) = encoded.len() as u32;
    CroResult::success()
}

fn do_cro_tx_sign_txin(
    address: &CroAddress,
    user_tx: &mut CroTx,
    which_tx_in_user: u16,
) -> Result<()> {
    let which_tx_in: usize = which_tx_in_user as usize;
    assert!(user_tx.tx.is_some());
    assert!(which_tx_in < user_tx.txin.len());
    assert!(which_tx_in < user_tx.txin_witness.len());
    let tx: &mut Tx = user_tx.tx.as_mut().chain(|| {
        (
            ErrorKind::DeserializationError,
            "Unable to decode hex of txid",
        )
    })?;
    let txid: TxId = tx.id();
    let witness: TxInWitness = schnorr_sign(&txid, &address.publickey, &address.privatekey)?;
    user_tx.txin_witness[which_tx_in] = Some(witness);
    assert!(user_tx.txin.len() == user_tx.txin_witness.len());
    Ok(())
}

/// sign for each txin
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_sign_txin(
    address_ptr: CroAddressPtr,
    tx_ptr: CroTxPtr,
    which_tx_in_user: u16,
) -> CroResult {
    let mut user_tx: &mut CroTx = tx_ptr.as_mut().expect("get tx");
    let address: &CroAddress = address_ptr.as_mut().expect("get address");
    match do_cro_tx_sign_txin(&address, &mut user_tx, which_tx_in_user) {
        Ok(_) => CroResult::success(),
        Err(_) => CroResult::fail(),
    }
}

fn schnorr_sign(
    message: &TxId,
    public_key: &PublicKey,
    private_key: &PrivateKey,
) -> Result<TxInWitness> {
    let public_keys: Vec<PublicKey> = vec![public_key.clone()];
    let multi_sig_address = MultiSigAddress::new(public_keys.to_vec(), public_keys[0].clone(), 1)?;

    let proof = multi_sig_address
        .generate_proof(public_keys.to_vec())?
        .chain(|| (ErrorKind::InvalidInput, "Unable to generate merkle proof"))?;
    Ok(TxInWitness::TreeSig(
        private_key.schnorr_sign(message)?,
        proof,
    ))
}

fn do_cro_tx_add_txout(tx: &mut CroTx, addr: &str, coin: u64) -> Result<()> {
    let txout = TxOut::new(
        ExtendedAddr::from_str(&addr).chain(|| {
            (
                ErrorKind::DeserializationError,
                "Unable to decode extended addr",
            )
        })?,
        Coin::new(coin).chain(|| (ErrorKind::DeserializationError, "Unable to decode coin"))?,
    );
    tx.txout.push(txout);
    Ok(())
}

/// add txout
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_add_txout(
    tx_ptr: CroTxPtr,
    addr_string: *const c_char,
    coin: u64,
) -> CroResult {
    let mut tx = tx_ptr.as_mut().expect("get tx");
    let addr = get_string(addr_string);
    match do_cro_tx_add_txout(&mut tx, &addr, coin) {
        Ok(_) => CroResult::success(),
        Err(_) => CroResult::fail(),
    }
}

/// add txout with bytes
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_tx_add_txout_raw(
    tx_ptr: CroTxPtr,
    addr: [u8; 32],
    coin: u64,
) -> CroResult {
    let tx = tx_ptr.as_mut().expect("get tx");
    let txout = TxOut::new(
        ExtendedAddr::OrTree(addr),
        Coin::new(coin).expect("get coin in cro_tx_add_txout_raw"),
    );
    tx.txout.push(txout);
    CroResult::success()
}

/// destroy tx
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_destroy_tx(tx: CroTxPtr) -> CroResult {
    Box::from_raw(tx);
    CroResult::success()
}
