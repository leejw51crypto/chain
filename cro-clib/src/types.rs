use chain_core::tx::data::input::TxoPointer;
use chain_core::tx::data::output::TxOut;
use chain_core::tx::data::Tx;
use chain_core::tx::fee::{LinearFee, Milli};
use chain_core::tx::witness::TxInWitness;
use client_common::{PrivateKey, PublicKey};
use client_core::transaction_builder::WitnessedUTxO;
use client_core::HDSeed;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_int;
pub type CroHDWalletPtr = *mut CroHDWallet;
pub type CroAddressPtr = *mut CroAddress;
pub type CroTxPtr = *mut CroTx;
pub type CroFeePtr = *mut CroFee;

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
#[derive(Clone)]
pub struct CroStakedState {
    pub nonce: u64,
    pub bonded: u64,
    pub unbonded: u64,
    pub unbonded_from: u64,
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

/// copy from c array to rust array
/// # Safety
pub unsafe fn copy_string(src: &str, dst: &mut [u8]) {
    dst[..src.len()].copy_from_slice(&src.as_bytes()[..src.len()]);
    dst[src.len()] = 0;
}

/// # Safety
pub unsafe fn get_string(src: *const c_char) -> String {
    CStr::from_ptr(src).to_string_lossy().into_owned()
}

#[repr(C)]
pub struct CroUtxo {
    pub address: [u8; 32], // holder
    pub value: u64,        // amount in carson unit
    pub valid_from: u64,   // time lock
}

#[derive(Clone, Default)]
pub struct CroTx {
    // out amount - in amount = fee
    // in
    // if signed, these 3 counts are the same
    pub txin_pointer: Vec<TxoPointer>,          // which utxo?
    pub txin_witness: Vec<Option<TxInWitness>>, // sinature
    pub txin: Vec<TxOut>,                       // utxo which will be consumed
    // out
    pub txout: Vec<TxOut>, // new utxo
    // viewkeys which can view thix tx
    pub viewkey: Vec<secp256k1::PublicKey>,

    // core tx
    pub tx: Option<Tx>,
}

#[derive(Clone)]
pub struct CroFee {
    pub fee: LinearFee,
}

impl Default for CroFee {
    fn default() -> Self {
        let fee = LinearFee::new(Milli::new(0, 0), Milli::new(0, 0));
        CroFee { fee }
    }
}
