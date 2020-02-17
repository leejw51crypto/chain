use crate::types::get_string;
use crate::types::CroStakedState;
use crate::types::{CroAddress, CroAddressPtr, CroResult};
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
use client_common::{ErrorKind, Result, ResultExt};
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
fn do_cro_unbond(
    network: u8,
    nonce: u64,
    from_address: &CroAddress,
    to_address_user: &str,
    amount: u64,
) -> Result<Vec<u8>> {
    let to_address = StakedStateAddress::from_str(&to_address_user).chain(|| {
        (
            ErrorKind::DeserializationError,
            format!("Unable to deserialize to_address ({})", to_address_user),
        )
    })?;
    let value =
        Coin::new(amount).chain(|| (ErrorKind::DeserializationError, "Invalid Coin Amount"))?; // carson unit
    let attributes = StakedStateOpAttributes::new(network);
    let transaction: UnbondTx = UnbondTx::new(to_address, nonce, value, attributes);
    let from_private = &from_address.privatekey;
    let signature: StakedStateOpWitness = from_private
        .sign(transaction.id())
        .map(StakedStateOpWitness::new)?;
    let result = TxAux::UnbondStakeTx(transaction, signature);
    let encoded = result.encode();
    Ok(encoded)
}

/// staked -> staked
/// network: networkid
/// nonce: nonce of the staked state, use cro_get_staked_state to get this nonce
/// from_ptr: staking address
/// to_address_user:staking address
/// amount: carson unit   1 carson= 0.0000_0001 cro
/// output: signed tx encoded
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
    let to_address = get_string(to_address_user);
    let from_address = from_ptr.as_mut().expect("get address");

    match do_cro_unbond(network, nonce, from_address, &to_address, amount) {
        Ok(encoded) => {
            ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
            (*output_length) = encoded.len() as u32;

            CroResult::success()
        }
        Err(_) => CroResult::fail(),
    }
}

fn do_cro_withdraw(
    tendermint_url: &str,
    network: u8,
    from_address: &CroAddress,
    to_address: &CroAddress,
    viewkeys: Vec<String>, // viewkeys
) -> Result<Vec<u8>> {
    let from_private = &from_address.privatekey;
    assert!(20 == from_address.raw.len());
    let tendermint_client = WebsocketRpcClient::new(&tendermint_url)?;
    let bytes = tendermint_client
        .query("account", &from_address.raw)?
        .bytes()?;
    let staked_state = StakedState::decode(&mut bytes.as_slice()).chain(|| {
        (
            ErrorKind::DeserializationError,
            format!(
                "Cannot deserialize staked state for address: {}",
                hex::encode(bytes)
            ),
        )
    })?;
    let mut access_policies = BTreeSet::new();
    for a in viewkeys {
        let view_key = a.trim();
        let publickey = PublicKey::from_str(view_key)?;
        access_policies.insert(TxAccessPolicy {
            view_key: publickey.into(),
            access: TxAccess::AllData,
        });
    }
    let attributes = TxAttributes::new_with_access(network, access_policies.into_iter().collect());
    let nonce = staked_state.nonce;
    let amount = staked_state.unbonded;
    let outputs = vec![TxOut::new_with_timelock(
        ExtendedAddr::from_str(&to_address.address).chain(|| {
            (
                ErrorKind::DeserializationError,
                format!("Unable to deserialize to_address ({})", to_address.address),
            )
        })?,
        amount,
        staked_state.unbonded_from,
    )];
    let transaction = WithdrawUnbondedTx::new(nonce, outputs, attributes);
    let signature = from_private
        .sign(transaction.id())
        .map(StakedStateOpWitness::new)?;
    let signed_transaction = SignedTransaction::WithdrawUnbondedStakeTransaction(
        transaction,
        Box::new(staked_state),
        signature,
    );
    let encoded = signed_transaction.encode();
    Ok(encoded)
}

/// staked -> utxo
/// tendermint_url_string:  "ws://localhost:26657/websocket"
/// network: network-id 0xab
/// from_ptr: staking address
/// to_ptr: transfer address
/// viewkeys: viewkey list, this is string list
/// output: minimum 1000 bytes, signed tx encoded
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
    let array: &[*const c_char] = std::slice::from_raw_parts(viewkeys, viewkey_count as usize);
    let mut viewkeys: Vec<String> = vec![];
    for x in array {
        let a = get_string(*x);
        let view_key = a.trim();
        viewkeys.push(view_key.to_string());
    }
    match do_cro_withdraw(
        &tendermint_url,
        network,
        &from_address,
        &to_address,
        viewkeys,
    ) {
        Ok(encoded) => {
            ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
            (*output_length) = encoded.len() as u32;
            CroResult::success()
        }
        Err(_) => CroResult::fail(),
    }
}

fn do_cro_get_staked_state(from_address: &CroAddress, tendermint_url: &str) -> Result<StakedState> {
    let tendermint_client = WebsocketRpcClient::new(&tendermint_url)?;
    let result = tendermint_client.query("txquery", &[])?.bytes()?;
    let address = std::str::from_utf8(&result).chain(|| {
        (
            ErrorKind::ConnectionError,
            "Unable to decode txquery address",
        )
    })?;
    let address_args: Vec<&str> = address.split(':').collect();
    let _transaction_obfuscation: DefaultTransactionObfuscation =
        DefaultTransactionObfuscation::new(
            address_args[0].to_string(),
            address_args[1].to_string(),
        );

    let tendermint_client = WebsocketRpcClient::new(&tendermint_url)?;

    assert!(20 == from_address.raw.len());
    let bytes = tendermint_client
        .query("account", &from_address.raw)?
        .bytes()?;
    let state = StakedState::decode(&mut bytes.as_slice()).chain(|| {
        (
            ErrorKind::DeserializationError,
            format!(
                "Cannot deserialize staked state for address: {}",
                hex::encode(bytes)
            ),
        )
    })?;
    Ok(state)
}

/// staked -> utxo
/// tenermint_url_string: ws://localhost:26657/websocket
/// staked_state_user: retrieved state will be written
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
    match do_cro_get_staked_state(from_address, &tendermint_url) {
        Ok(state) => {
            staked_state.nonce = state.nonce;
            staked_state.unbonded_from = state.unbonded_from;
            staked_state.bonded = state.bonded.into();
            staked_state.unbonded = state.unbonded.into();
            CroResult::success()
        }
        Err(_) => CroResult::fail(),
    }
}

fn do_cro_encrypt(tendermint_url: &str, signed_transaction_encoded: Vec<u8>) -> Result<Vec<u8>> {
    let tendermint_client = WebsocketRpcClient::new(&tendermint_url)?;
    let result = tendermint_client.query("txquery", &[])?.bytes()?;
    let address = std::str::from_utf8(&result)
        .chain(|| (ErrorKind::DeserializationError, "Unable to decode address"))?;
    let signed_transaction: SignedTransaction =
        SignedTransaction::decode(&mut signed_transaction_encoded.as_slice()).chain(|| {
            (
                ErrorKind::DeserializationError,
                "Unable to decode signed transaction",
            )
        })?;
    let transaction_obfuscation: DefaultTransactionObfuscation =
        DefaultTransactionObfuscation::new(address.to_string(), "localhost".to_string());
    let txaux = transaction_obfuscation.encrypt(signed_transaction)?;
    let encoded: Vec<u8> = txaux.encode();
    Ok(encoded)
}

/// tendermint_url_string: default "ws://localhost:26657/websocket"
/// signed_transaction_user: signed tx encoded to encrypt
/// output: encrypted result will be written
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_encrypt(
    tenermint_url_string: *const c_char,
    signed_transaction_user: *const u8,
    signed_transaction_length: u32,
    output: *mut u8,
    output_length: *mut u32,
) -> CroResult {
    let signed_transaction_encoded =
        std::slice::from_raw_parts(signed_transaction_user, signed_transaction_length as usize)
            .to_vec();
    let tendermint_url = get_string(tenermint_url_string);

    match do_cro_encrypt(&tendermint_url, signed_transaction_encoded) {
        Ok(encoded) => {
            ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
            (*output_length) = encoded.len() as u32;
            CroResult::success()
        }
        Err(_) => CroResult::fail(),
    }
}

fn do_cro_broadcast(tendermint_url: &str, data: &[u8]) -> Result<String> {
    let tendermint_client = WebsocketRpcClient::new(&tendermint_url)?;
    let result = tendermint_client.broadcast_transaction(data)?;
    let json =
        serde_json::to_string(&result).chain(|| (ErrorKind::InvalidInput, "tx broadcast fail"))?;
    Ok(json)
}

/// staked -> utxo
/// tendermint_url: ws://localhost:26657/websocket
/// user_data: tx data to send
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_broadcast(
    tenermint_url_string: *const c_char,
    user_data: *const u8,
    data_length: u32,
) -> CroResult {
    let tendermint_url = get_string(tenermint_url_string);
    let data: &[u8] = std::slice::from_raw_parts(user_data, data_length as usize);
    match do_cro_broadcast(&tendermint_url, &data) {
        Ok(_) => CroResult::success(),
        Err(_) => CroResult::fail(),
    }
}
