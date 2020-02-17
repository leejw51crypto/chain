use crate::types::get_string;
use crate::types::{CroAddress, CroAddressPtr, CroResult};
pub use chain_core::init::network::Network;
use chain_core::state::account::{
    CouncilNode, StakedStateAddress, StakedStateOpAttributes, StakedStateOpWitness, UnjailTx,
};
use chain_core::state::tendermint::TendermintValidatorPubKey;
use chain_core::state::validator::NodeJoinRequestTx;
use chain_core::tx::TransactionId;
use chain_core::tx::TxAux;
use client_common::{ErrorKind, Result, ResultExt};
use parity_scale_codec::Encode;
use std::ptr;
use std::str::FromStr;
use std::string::ToString;
fn do_cro_unjail(
    network: u8,
    nonce: u64,
    from_address: &CroAddress,
    to_address_user: &str,
) -> Result<Vec<u8>> {
    let to_address = StakedStateAddress::from_str(&to_address_user).chain(|| {
        (
            ErrorKind::DeserializationError,
            format!("Unable to deserialize to_address ({})", to_address_user),
        )
    })?;
    let attributes = StakedStateOpAttributes::new(network);
    let transaction: UnjailTx = UnjailTx::new(nonce, to_address, attributes);
    let from_private = &from_address.privatekey;
    let signature: StakedStateOpWitness = from_private
        .sign(transaction.id())
        .map(StakedStateOpWitness::new)?;
    let result = TxAux::UnjailTx(transaction, signature);
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
pub unsafe extern "C" fn cro_unjai(
    network: u8,
    nonce: u64,
    from_ptr: CroAddressPtr,
    to_address_user: *const std::os::raw::c_char,
    output: *mut u8,
    output_length: *mut u32,
) -> CroResult {
    let to_address = get_string(to_address_user);
    let from_address = from_ptr.as_mut().expect("get address");

    match do_cro_unjail(network, nonce, from_address, &to_address) {
        Ok(encoded) => {
            ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
            (*output_length) = encoded.len() as u32;

            CroResult::success()
        }
        Err(_) => CroResult::fail(),
    }
}

/// validator_pubkey: base64 pubkey,  32 bytes
fn do_cro_join(
    network: u8,
    nonce: u64,
    from_address: &CroAddress,
    to_address_user: &str,
    validator_name: &str,
    validator_contact: &str,
    validator_pubkey: &str,
) -> Result<Vec<u8>> {
    let to_address = StakedStateAddress::from_str(&to_address_user).chain(|| {
        (
            ErrorKind::DeserializationError,
            format!("Unable to deserialize to_address ({})", to_address_user),
        )
    })?;
    let attributes = StakedStateOpAttributes::new(network);
    let pubkey: TendermintValidatorPubKey =
        TendermintValidatorPubKey::from_base64(validator_pubkey.as_bytes()).chain(|| {
            (
                ErrorKind::DeserializationError,
                "Unable to get validator pubkey",
            )
        })?;

    let node_metadata = CouncilNode {
        name: validator_name.to_string(),
        security_contact: Some(validator_contact.to_string()),
        // 32 bytes
        consensus_pubkey: pubkey,
    };
    let transaction: NodeJoinRequestTx = NodeJoinRequestTx {
        nonce: nonce,
        address: to_address,
        attributes,
        node_meta: node_metadata,
    };
    let from_private = &from_address.privatekey;
    let signature: StakedStateOpWitness = from_private
        .sign(transaction.id())
        .map(StakedStateOpWitness::new)?;
    let result = TxAux::NodeJoinTx(transaction, signature);
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
pub unsafe extern "C" fn cro_join(
    network: u8,
    nonce: u64,
    from_ptr: CroAddressPtr,
    to_address_user: *const std::os::raw::c_char,
    validator_name_user: *const std::os::raw::c_char,
    validator_contact_user: *const std::os::raw::c_char,
    validator_pubkey_user: *const std::os::raw::c_char,
    output: *mut u8,
    output_length: *mut u32,
) -> CroResult {
    let to_address = get_string(to_address_user);
    let validator_name = get_string(validator_name_user);
    let validator_contact = get_string(validator_contact_user);
    let validator_pubkey = get_string(validator_pubkey_user);
    let from_address = from_ptr.as_mut().expect("get address");

    match do_cro_join(
        network,
        nonce,
        from_address,
        &to_address,
        &validator_name,
        &validator_contact,
        &validator_pubkey,
    ) {
        Ok(encoded) => {
            ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
            (*output_length) = encoded.len() as u32;

            CroResult::success()
        }
        Err(_) => CroResult::fail(),
    }
}
