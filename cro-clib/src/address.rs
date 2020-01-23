use crate::types::{CroAddressPtr, CroResult};
use std::ffi::CString;
use std::ptr;
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
pub unsafe extern "C" fn cro_export_private(
    address_ptr: CroAddressPtr,
    dst: *mut u8,
    dst_length: *mut u32,
) -> CroResult {
    let address = address_ptr.as_mut().expect("get address");
    let src = address.privatekey.serialize();
    if src.len() > dst_length as usize {
        return CroResult::fail();
    }
    ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    *(dst_length) = src.len() as u32;
    CroResult::success()
}

/// print address information
/// minimum byte length 100 is necessary
#[no_mangle]
/// # Safety
pub unsafe extern "C" fn cro_get_printed_address(
    address_ptr: CroAddressPtr,
    address_output: *mut u8,
    address_output_length: u32,
) -> CroResult {
    let address = address_ptr.as_mut().expect("get address");
    let src_string = CString::new(address.address.as_bytes()).expect("get cstring");
    let src = src_string.to_bytes_with_nul();
    if src.len() > address_output_length as usize {
        return CroResult::fail();
    }
    ptr::copy_nonoverlapping(src.as_ptr(), address_output, src.len());
    CroResult::success()
}

/// print address information
/// minimum 32 length is necessary
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
    if src.len() > address_output_length as usize {
        return CroResult::fail();
    }
    ptr::copy_nonoverlapping(src.as_ptr(), address_output, src.len());
    *address_output_length = src.len() as u32;

    CroResult::success()
}
