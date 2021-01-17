use k256::elliptic_curve::sec1::ToEncodedPoint;
use tiny_keccak::{Hasher, Keccak};

use super::{Context, Result};
use crate::crypto::public_key;
use crate::utils::*;
use heapless::{consts::*, String};
const NUM_ADDRS: usize = 5;
const ADDR_SIZE: usize = 20;
pub fn addresses(ctx: &mut Context) -> Result<[u8; ADDR_SIZE * NUM_ADDRS]> {
    let idx = ctx.idx as usize;
    let mut buf = [0u8; ADDR_SIZE * NUM_ADDRS];
    for i in idx..idx + NUM_ADDRS {
        let start: usize = (i - idx) * ADDR_SIZE;
        let stop: usize = start + ADDR_SIZE; // Addresses are 20 bytes
        buf[start..stop].copy_from_slice(&address(ctx.set_idx(i as u32))?);
    }
    Ok(buf)
}

pub fn address(ctx: &Context) -> Result<[u8; ADDR_SIZE]> {
    let uncompressed_pubkey = public_key(ctx)?.to_encoded_point(false);
    let pubkey_bytes = uncompressed_pubkey.as_bytes();
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    let mut buf = [0u8; 32];
    hasher.finalize(&mut buf);
    let mut address = [0u8; ADDR_SIZE];
    address.copy_from_slice(&buf[12..]);
    Ok(address)
}

pub fn addr_str(ctx: &Context) -> String<U64> {
    let addr = address(&ctx).ok().unwrap();
    let mut addr_str = String::<_>::from("0x");
    addr[..3].iter().for_each(|b| {
        let _ = addr_str.push(hex_upper_nibble(b));
        let _ = addr_str.push(hex_lower_nibble(b));
    });
    let _ = addr_str.push_str("..");
    addr[17..].iter().for_each(|b| {
        let _ = addr_str.push(hex_upper_nibble(b));
        let _ = addr_str.push(hex_lower_nibble(b));
    });
    addr_str
}
