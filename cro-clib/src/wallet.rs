use crate::types::get_string;
use crate::types::{
    CroAccount, CroAddress, CroAddressPtr, CroHDWallet, CroHDWalletPtr, CroNetwork, CroResult,
};
use chain_core::init::address::RedeemAddress;
use chain_core::init::network::Network;
use chain_core::state::account::StakedStateAddress;
use chain_core::tx::data::address::ExtendedAddr;
use client_common::MultiSigAddress;
use client_core::{HDSeed, Mnemonic};
use secstr::SecUtf8;
use std::os::raw::c_char;
use std::ptr;
impl From<CroNetwork> for Network {
    fn from(src: CroNetwork) -> Self {
        match src {
            CroNetwork::Mainnet => Network::Mainnet,
            CroNetwork::Testnet => Network::Testnet,
            CroNetwork::Devnet => Network::Devnet,
        }
    }
}

#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_create_hdwallet(
    wallet_out: *mut CroHDWalletPtr,
    mnemonics: *mut u8,
    mnemonics_length: u32,
) -> CroResult {
    let mnemonic = Mnemonic::new();
    let phrase = mnemonic.unsecure_phrase();
    assert!(phrase.as_bytes().len() < mnemonics_length as usize);
    let wallet = CroHDWallet {
        seed: HDSeed::from(&mnemonic),
    };
    let wallet_box = Box::new(wallet);
    ptr::write(wallet_out, Box::into_raw(wallet_box));
    ptr::copy_nonoverlapping(
        phrase.as_bytes().as_ptr(),
        mnemonics,
        phrase.as_bytes().len(),
    );
    CroResult::success()
}

#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_restore_hdwallet(
    mnemonics_string: *const c_char,
    wallet_out: *mut CroHDWalletPtr,
) -> CroResult {
    let mnemonics = get_string(mnemonics_string);
    let mnemonics_sec = SecUtf8::from(mnemonics);
    let mnemonic = Mnemonic::from_secstr(&mnemonics_sec).unwrap();
    let wallet = CroHDWallet {
        seed: HDSeed::from(&mnemonic),
    };
    let wallet_box = Box::new(wallet);
    ptr::write(wallet_out, Box::into_raw(wallet_box));
    CroResult::success()
}

/// create staking address from bip44 hdwallet
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_create_staking_address(
    wallet_ptr: CroHDWalletPtr,
    network: CroNetwork,
    address_out: *mut CroAddressPtr,
    index: u32,
) -> CroResult {
    if wallet_ptr.is_null() {
        return CroResult::fail();
    }
    let wallet = wallet_ptr.as_mut().expect("get wallet");
    let (public, private) = wallet
        .seed
        .derive_key_pair(network.into(), CroAccount::Staking as u32, index)
        .unwrap();
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

/// print address information
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_print_address(address_ptr: CroAddressPtr) -> CroResult {
    let address = address_ptr.as_mut().expect("get address");
    println!("{}", address.address.to_string());
    CroResult::success()
}

/// print address information
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_get_printed_address(
    address_ptr: CroAddressPtr,
    address_output: *mut u8,
    address_output_length: u32,
) -> CroResult {
    let address = address_ptr.as_mut().expect("get address");
    let address_string = address.address.to_string();
    let mut src_string = address_string.as_bytes().to_vec();
    src_string.push(0);
    let src = &src_string[..];
    assert!(src.len() < address_output_length as usize);
    ptr::copy_nonoverlapping(src.as_ptr(), address_output, src.len());

    CroResult::success()
}

/// print address information
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_get_raw_address(
    address_ptr: CroAddressPtr,
    address_output: *mut u8,
    address_output_length: *mut u32,
) -> CroResult {
    if address_output.is_null() {
        return CroResult::fail();
    }
    if address_output_length.is_null() {
        return CroResult::fail();
    }
    let address = address_ptr.as_mut().expect("get address");

    let src_bytes = address.raw.clone();

    let src = &src_bytes[..];
    assert!(src.len() < address_output_length as usize);
    ptr::copy_nonoverlapping(src.as_ptr(), address_output, src.len());
    *address_output_length = src.len() as u32;

    CroResult::success()
}

/// create utxo address from bip44 wallet, which is for withdrawal, transfer amount
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_create_transfer_address(
    wallet_ptr: CroHDWalletPtr,
    network: CroNetwork,
    address_out: *mut CroAddressPtr,
    index: u32,
) -> CroResult {
    if wallet_ptr.is_null() {
        return CroResult::fail();
    }
    let wallet = wallet_ptr.as_mut().expect("get wallet");
    let (public, private) = wallet
        .seed
        .derive_key_pair(network.into(), CroAccount::Transfer as u32, index)
        .unwrap();
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

/// create viewkey, which is for encrypted tx
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_create_viewkey(
    wallet_ptr: CroHDWalletPtr,
    network: CroNetwork,
    address_out: *mut CroAddressPtr,
    index: u32,
) -> CroResult {
    if wallet_ptr.is_null() {
        return CroResult::fail();
    }
    let wallet = wallet_ptr.as_mut().expect("get wallet");
    let (public, private) = wallet
        .seed
        .derive_key_pair(network.into(), CroAccount::Viewkey as u32, index)
        .unwrap();
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

/// destroy bip44 hdwallet
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_destroy_hdwallet(hdwallet: CroHDWalletPtr) -> CroResult {
    Box::from_raw(hdwallet);
    CroResult::success()
}

/// destroy address
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_destroy_address(addr: CroAddressPtr) -> CroResult {
    Box::from_raw(addr);
    CroResult::success()
}
