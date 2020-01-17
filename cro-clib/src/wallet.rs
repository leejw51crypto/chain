use chain_core::init::network::Network;
use std::ffi::CStr;
use std::os::raw::c_char;

use chain_core::init::address::RedeemAddress;
use chain_core::state::account::StakedStateAddress;
use chain_core::tx::data::address::ExtendedAddr;
use client_common::MultiSigAddress;
use client_common::{PrivateKey, PublicKey};
use client_core::{HDSeed, Mnemonic};
use secstr::SecUtf8;
use std::ptr;
pub type HDWalletPtr = *mut HDWallet;
pub type AddressPtr = *mut Address;

pub const SUCCESS: i32 = 0;
pub const FAIL: i32 = 0;

#[derive(Clone)]
pub struct HDWallet {
    pub seed: HDSeed,
}

#[derive(Clone)]
pub struct Address {
    pub privatekey: PrivateKey,
    pub publickey: PublicKey,
    pub address: String,
}

#[allow(dead_code)]
unsafe fn get_string(src: *const c_char) -> String {
    CStr::from_ptr(src).to_string_lossy().into_owned()
}

#[allow(dead_code)]
fn copy_string(src: &str, dst: &mut [u8]) {
    dst[..src.len()].copy_from_slice(&src.as_bytes()[..src.len()]);
    dst[src.len()] = 0;
}

/// hdwallet creating using bip44 hdwallet
#[no_mangle]
pub unsafe extern "C" fn cro_create_hdwallet(
    wallet_out: *mut HDWalletPtr,
    mnemonics: *mut u8,
    mnemonics_length: i32,
) -> i32 {
    let mnemonic = Mnemonic::new();
    let phrase = mnemonic.unsecure_phrase();
    assert!(phrase.as_bytes().len() < mnemonics_length as usize);
    let wallet = HDWallet {
        seed: HDSeed::from(&mnemonic),
    };
    let wallet_box = Box::new(wallet);
    ptr::write(wallet_out, Box::into_raw(wallet_box));
    ptr::copy_nonoverlapping(
        phrase.as_bytes().as_ptr(),
        mnemonics,
        phrase.as_bytes().len(),
    );
    SUCCESS
}

/// restore bip44 hdwallet from mnemonics which user gives
#[no_mangle]
pub unsafe extern "C" fn cro_restore_hdwallet(
    mnemonics_string: *const c_char,
    wallet_out: *mut HDWalletPtr,
) -> i32 {
    let mnemonics = get_string(mnemonics_string);
    let mnemonics_sec = SecUtf8::from(mnemonics);
    let mnemonic = Mnemonic::from_secstr(&mnemonics_sec).unwrap();
    let wallet = HDWallet {
        seed: HDSeed::from(&mnemonic),
    };
    let wallet_box = Box::new(wallet);
    ptr::write(wallet_out, Box::into_raw(wallet_box));
    SUCCESS
}

/// create staking address from bip44 hdwallet
#[no_mangle]
pub unsafe extern "C" fn cro_create_staking_address(
    wallet_ptr: HDWalletPtr,
    address_out: *mut AddressPtr,
    index: u32,
) -> i32 {
    if wallet_ptr.is_null() {
        return FAIL;
    }
    let wallet = wallet_ptr.as_mut().expect("get wallet");
    let (public, private) = wallet
        .seed
        .derive_key_pair(Network::Devnet, 0, index)
        .unwrap();
    let address = StakedStateAddress::BasicRedeem(RedeemAddress::from(&public));

    let ret = Address {
        privatekey: private,
        publickey: public,
        address: address.to_string(),
    };
    let address_box = Box::new(ret);
    ptr::write(address_out, Box::into_raw(address_box));
    SUCCESS
}

/// print address information
#[no_mangle]
pub unsafe extern "C" fn print_address(address_ptr: AddressPtr) -> i32 {
    let address = address_ptr.as_mut().expect("get address");
    println!("{}", address.address.to_string());
    SUCCESS
}
/// create utxo address from bip44 wallet, which is for withdrawal, transfer amount
#[no_mangle]
pub unsafe extern "C" fn cro_create_transfer_address(
    wallet_ptr: HDWalletPtr,
    address_out: *mut AddressPtr,
    index: u32,
) -> i32 {
    if wallet_ptr.is_null() {
        return FAIL;
    }
    let wallet = wallet_ptr.as_mut().expect("get wallet");
    let (public, private) = wallet
        .seed
        .derive_key_pair(Network::Devnet, 1, index)
        .unwrap();
    let public_keys = vec![public.clone()];
    let multi_sig_address = MultiSigAddress::new(public_keys, public.clone(), 1).unwrap();

    let address: ExtendedAddr = multi_sig_address.into();
    let ret = Address {
        privatekey: private,
        publickey: public,
        address: address.to_string(),
    };
    let address_box = Box::new(ret);
    ptr::write(address_out, Box::into_raw(address_box));

    SUCCESS
}

/// create viewkey, which is for encrypted tx
#[no_mangle]
pub unsafe extern "C" fn cro_create_viewkey(wallet_ptr: HDWalletPtr, _index: i32) -> i32 {
    let _wallet = wallet_ptr.as_mut().expect("get wallet");
    println!("create viewkey");
    SUCCESS
}

/// destroy bip44 hdwallet
#[no_mangle]
pub unsafe extern "C" fn cro_destroy_hdwallet(hdwallet: HDWalletPtr) -> i32 {
    Box::from_raw(hdwallet);
    SUCCESS
}

/// destroy address
#[no_mangle]
pub unsafe extern "C" fn cro_destroy_address(addr: AddressPtr) -> i32 {
    Box::from_raw(addr);
    SUCCESS
}
