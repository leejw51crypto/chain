use crate::types::get_string;
use crate::types::CroStakedState;
use crate::types::{CroAddressPtr, CroResult};
use chain_core::init::coin::Coin;
pub use chain_core::init::network::Network;
use chain_core::state::account::{
    StakedState, StakedStateAddress, StakedStateOpAttributes, StakedStateOpWitness, UnbondTx,
    WithdrawUnbondedTx,
};
use chain_core::tx::data::access::{TxAccess, TxAccessPolicy};
use chain_core::tx::data::address::ExtendedAddr;
use chain_core::tx::data::attribute::TxAttributes;
use chain_core::tx::data::output::TxOut;
use chain_core::tx::TransactionId;
use chain_core::tx::TxAux;
use client_common::tendermint::types::AbciQueryExt;
use client_common::tendermint::{Client, WebsocketRpcClient};
use client_common::{PublicKey, SignedTransaction};
use client_core::cipher::DefaultTransactionObfuscation;
use client_core::TransactionObfuscation;
use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
use std::collections::BTreeSet;
use std::os::raw::c_char;
use std::ptr;
use std::str::FromStr;
use std::string::ToString;

/// staked -> staked
#[no_mangle]
/// # Safety
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
/// # Safety
pub unsafe extern "C" fn cro_withdraw(
    tenermint_url_string: *const c_char,
    network: u8,
    from_ptr: CroAddressPtr,
    to_ptr: CroAddressPtr,
    viewkeys: *const *const c_char, // viewkeys
    viewkey_count: i32,
    output: *mut u8,
    output_length: *mut u32,
) -> CroResult {
    let tendermint_url = get_string(tenermint_url_string);

    let from_address = from_ptr.as_mut().expect("get address");
    let to_address = to_ptr.as_mut().expect("get address");
    let from_private = &from_address.privatekey;
    let _from_public = &from_address.publickey;

    assert!(20 == from_address.raw.len());
    let tendermint_client = WebsocketRpcClient::new(&tendermint_url).unwrap();
    let bytes = tendermint_client
        .query("account", &from_address.raw)
        .unwrap()
        .bytes()
        .unwrap();
    let staked_state = StakedState::decode(&mut bytes.as_slice()).unwrap();

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

    let nonce = staked_state.nonce;
    let amount = staked_state.unbonded;
    let outputs = vec![TxOut::new_with_timelock(
        ExtendedAddr::from_str(&to_address.address).unwrap(),
        amount,
        staked_state.unbonded_from,
    )];

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
    println!("-----------------------------------------");
    let encoded = signed_transaction.encode();
    ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
    (*output_length) = encoded.len() as u32;

    println!("viewcount {}", viewkey_count);
    CroResult::success()
}

/// staked -> utxo
/// tendermint_url: ws://localhost:26657/websocket
#[no_mangle]
/// # Safety
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
/// # Safety
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
    println!("signed tx={:?}", signed_transaction);

    let transaction_obfuscation: DefaultTransactionObfuscation = DefaultTransactionObfuscation::new(
        // address_args[0].to_string(),
        // address_args[1].to_string(),
        address.to_string(),
        "localhost".to_string(),
    );
    println!("************************************  txaux");
    let txaux = transaction_obfuscation.encrypt(signed_transaction).unwrap();
    println!("txaux={:?}", txaux);
    let encoded: Vec<u8> = txaux.encode();
    println!("txaux encoded={}", encoded.len());
    ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
    (*output_length) = encoded.len() as u32;

    CroResult::success()
}

/// staked -> utxo
/// tendermint_url: ws://localhost:26657/websocket
#[no_mangle]
/// # Safety
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
