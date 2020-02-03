use client_common::{PrivateKey, PublicKey};
use client_core::HDSeed;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_int;
pub type CroHDWalletPtr = *mut CroHDWallet;
pub type CroAddressPtr = *mut CroAddress;

pub const SUCCESS: i32 = 0;
pub const FAIL: i32 = -1;

pub type CroString = [u8; 200];
/// account types
#[repr(C)]
pub enum CroAccount {
    /// Account for transfer address
    Transfer = 0,
    /// Account for staking address
    Staking = 1,
    /// Account for viewkey
    Viewkey = 2,
}

#[derive(Clone)]
pub struct CroHDWallet {
    pub seed: HDSeed,
}

#[derive(Clone)]
pub struct CroAddress {
    pub privatekey: PrivateKey,
    pub publickey: PublicKey,
    pub raw: Vec<u8>,
    pub address: String,
}

#[repr(C)]
#[derive(Debug)]
pub struct CroResult {
    result: c_int,
}
impl CroResult {
    pub fn success() -> CroResult {
        CroResult { result: SUCCESS }
    }
    pub fn fail() -> CroResult {
        CroResult { result: FAIL }
    }
}

/// # Safety
pub unsafe fn get_string(src: *const c_char) -> String {
    CStr::from_ptr(src).to_string_lossy().into_owned()
}

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

#[repr(C)]
pub struct CroUtxo {
    pub address: [u8; 100], // holder
    pub coin: [u8; 100],    // amount in carson unit
}

#[repr(C)]
pub struct CroTxOut {
    pub address: [u8; 100], // utxo string address
    pub amount: [u8; 100],  // string amount, CRO unit , 1 CRO= 1,0000,0000 Carson
}
