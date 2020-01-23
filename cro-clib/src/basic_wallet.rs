use crate::types::{CroAddress, CroAddressPtr, CroResult};
use chain_core::init::address::RedeemAddress;

use chain_core::state::account::StakedStateAddress;
use chain_core::tx::data::address::ExtendedAddr;
use client_common::MultiSigAddress;
use client_common::{PrivateKey, PublicKey};
use std::ptr;

/// # Safety
unsafe extern "C" fn do_cro_basic_create_staking_address(
    address_out: *mut CroAddressPtr,
    private: PrivateKey,
) -> CroResult {
    let public = PublicKey::from(&private);
    let address = StakedStateAddress::BasicRedeem(RedeemAddress::from(&public));

    match address {
        StakedStateAddress::BasicRedeem(redeem) => {
            assert!(20 == redeem.0.len());
            let raw = redeem.to_vec();

            let ret = CroAddress {
                privatekey: private,
                publickey: public,
                raw,
                address: address.to_string(),
            };
            let address_box = Box::new(ret);
            ptr::write(address_out, Box::into_raw(address_box));
            CroResult::success()
        }
    }
}

/// create staking address
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_basic_create_staking_address(
    address_out: *mut CroAddressPtr,
) -> CroResult {
    let private = PrivateKey::new().expect("get private key");
    do_cro_basic_create_staking_address(address_out, private)
}

/// restore staking address
/// input_length: maximum size of input
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_basic_restore_staking_address(
    address_out: *mut CroAddressPtr,
    input: *const u8,
    input_length: u32,
) -> CroResult {
    assert!(32 == input_length);
    let array: &[u8] = std::slice::from_raw_parts(input, input_length as usize);
    match PrivateKey::deserialize_from(array) {
        Ok(deserialized) => do_cro_basic_create_staking_address(address_out, deserialized),
        Err(_) => CroResult::fail(),
    }
}

/// # Safety
unsafe extern "C" fn do_cro_basic_create_transfer_address(
    address_out: *mut CroAddressPtr,
    private: PrivateKey,
) -> CroResult {
    let public = PublicKey::from(&private);

    let public_keys = vec![public.clone()];
    let multi_sig_address = MultiSigAddress::new(public_keys, public.clone(), 1).unwrap();

    let address: ExtendedAddr = multi_sig_address.into();

    match address {
        ExtendedAddr::OrTree(hash) => {
            let raw = hash.to_vec();
            // this is H256 hash
            assert!(32 == raw.len());

            let ret = CroAddress {
                privatekey: private,
                publickey: public,
                raw,
                address: address.to_string(),
            };
            let address_box = Box::new(ret);
            ptr::write(address_out, Box::into_raw(address_box));

            CroResult::success()
        }
    }
}

/// create staking address
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_basic_create_transfer_address(
    address_out: *mut CroAddressPtr,
) -> CroResult {
    let private = PrivateKey::new().expect("get private key");
    do_cro_basic_create_transfer_address(address_out, private)
}

/// restore staking address
/// input_length: maximum size of input
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_basic_restore_transfer_address(
    address_out: *mut CroAddressPtr,
    input: *const u8,
    input_length: u32,
) -> CroResult {
    assert!(32 == input_length);
    let array: &[u8] = std::slice::from_raw_parts(input, input_length as usize);
    match PrivateKey::deserialize_from(array) {
        Ok(deserialized) => do_cro_basic_create_transfer_address(address_out, deserialized),
        Err(_) => CroResult::fail(),
    }
}

/// # Safety
unsafe extern "C" fn do_cro_basic_create_viewkey(
    address_out: *mut CroAddressPtr,
    private: PrivateKey,
) -> CroResult {
    let public = PublicKey::from(&private);

    let raw: Vec<u8> = public.serialize();
    assert!(65 == raw.len());
    let ret = CroAddress {
        privatekey: private,
        publickey: public.clone(),
        raw,
        address: public.to_string(),
    };
    let address_box = Box::new(ret);
    ptr::write(address_out, Box::into_raw(address_box));

    CroResult::success()
}

/// create viewkey, which is for encrypted tx
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_basic_create_viewkey(address_out: *mut CroAddressPtr) -> CroResult {
    let private = PrivateKey::new().expect("get private key");
    do_cro_basic_create_viewkey(address_out, private)
}

/// restore staking address
/// input_length: maximum size of input
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_basic_restore_viewkey(
    address_out: *mut CroAddressPtr,
    input: *const u8,
    input_length: u32,
) -> CroResult {
    assert!(32 == input_length);
    let array: &[u8] = std::slice::from_raw_parts(input, input_length as usize);
    match PrivateKey::deserialize_from(array) {
        Ok(deserialized) => do_cro_basic_create_viewkey(address_out, deserialized),
        Err(_) => CroResult::fail(),
    }
}
