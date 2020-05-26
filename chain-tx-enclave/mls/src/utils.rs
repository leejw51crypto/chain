use rustls::internal::msgs::codec::{u24, Codec, Reader};

/// more efficient then `codec::encode_vec_u24`
#[inline]
pub fn encode_vec_u8_u24(bytes: &mut Vec<u8>, items: &[u8]) {
    debug_assert!(items.len() <= 0xff_ffff);
    u24(items.len() as u32).encode(bytes);
    bytes.extend_from_slice(items);
}

/// more efficient then `codec::encode_vec_u16`
#[inline]
pub fn encode_vec_u8_u16(bytes: &mut Vec<u8>, items: &[u8]) {
    debug_assert!(items.len() <= 0xffff);
    (items.len() as u16).encode(bytes);
    bytes.extend_from_slice(items);
}

/// more efficient then `codec::encode_vec_u8`
#[inline]
pub fn encode_vec_u8_u8(bytes: &mut Vec<u8>, items: &[u8]) {
    debug_assert!(items.len() <= 0xff);
    (items.len() as u8).encode(bytes);
    bytes.extend_from_slice(items);
}

/// more efficient then `codec::read_vec_u24`
#[inline]
pub fn read_vec_u8_u24_limited(r: &mut Reader, max_bytes: usize) -> Option<Vec<u8>> {
    let len = u24::read(r)?.0 as usize;
    if len > max_bytes {
        return None;
    }
    r.take(len).map(|slice| slice.to_vec())
}

/// more efficient then `codec::read_vec_u16`
#[inline]
pub fn read_vec_u8_u16(r: &mut Reader) -> Option<Vec<u8>> {
    let len = usize::from(u16::read(r)?);
    r.take(len).map(|slice| slice.to_vec())
}

/// more efficient then `codec::read_vec_u8`
#[inline]
pub fn read_vec_u8_u8(r: &mut Reader) -> Option<Vec<u8>> {
    let len = usize::from(u8::read(r)?);
    r.take(len).map(|slice| slice.to_vec())
}
