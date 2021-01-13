#![cfg_attr(not(feature = "std"), no_std)]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request<'a> {
    Ping,
    Sig(&'a [u8]),
    Info,
    Serial,
    PubKey,
    Address(u32),
    AddressList(u32),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response<'a> {
    Pong,
    Sig(&'a [u8]),
    Info((bool, u32, &'a [u8])),
    Serial(&'a [u8]),
    PubKey(&'a [u8]),
    Address(&'a [u8]),
    AddressList(&'a [u8]),
    Err(&'a str),
}

pub fn version() -> u8 {
    0x0
}

#[cfg(feature = "std")]
impl std::fmt::Display for Response<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pong => write!(f, "Pong"),
            Self::Sig(b) => write!(f, "Sig: 0x{}", hex::encode(b)),
            Self::PubKey(b) => write!(f, "PubKey: 0x{}", hex::encode(b)),
            Self::Address(b) => write!(f, "Address: 0x{}", hex::encode(b)),
            Self::AddressList(b) => {
                let mut addr_str = String::new();
                for (idx, addr) in b.chunks_exact(20).enumerate() {
                    addr_str += &format!("\t{}: 0x{}\n", idx, hex::encode(addr));
                }
                addr_str.truncate(addr_str.len() - 1);
                write!(f, "Addresses: \n{}", addr_str)
            }
            Self::Serial(b) => write!(f, "Serial: 0x{}", hex::encode(b)),
            Self::Info(b) => write!(f, "Info: {}, 0x{:X} {}", b.0, b.1, hex::encode(b.2)),
            Self::Err(s) => write!(f, "Err: {}", s),
        }
    }
}
