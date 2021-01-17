use super::{Context, FLASH_START, MNEMONIC, SEED_ADDR};
use bip39::{Language, Mnemonic, Seed};
use core::convert::TryInto;
use error::*;
use hal::{flash::FlashExt, stm32};
use heapless::{consts::*, ArrayLength, Vec};
use hex_literal::hex;
use stm32f4xx_hal as hal;
use tiny_hderive::{bip32::ExtendedPrivKey, bip44::ChildNumber};
use tiny_keccak::{Hasher, Keccak};

use aes_ccm::{
    aead::{consts::U8, AeadInPlace, NewAead},
    Aes256Ccm,
};

use k256::ecdsa::{recoverable, signature::Signer, SigningKey, VerifyingKey};

use crate::error;

type Result<T> = super::Result<T>;

const AES_KEY: &[u8] = &hex!("C0 C1 C2 C3 C4 C5 C6 C7 C8 C9 CA CB CC CD CE CF");
const NONCE: &[u8] = &hex!("00 00 00 03 02 01 00 A0 A1 A2 A3 A4 A5");
// const ASSOCIATED_DATA: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];

pub fn get_uid_raw() -> &'static [u8] {
    let ptr = 0x1FFF_7A10 as *const u8;
    unsafe { core::slice::from_raw_parts(ptr, 12) }
}

pub fn get_aes_key() -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(&get_uid_raw());
    hasher.update(AES_KEY);
    let mut buf = [0u8; 32];
    hasher.finalize(&mut buf);
    buf
}

pub fn sign_msg(ctx: &Context, msg: &[u8]) -> Result<recoverable::Signature> {
    Ok(secret_key(ctx)?.try_sign(&msg)?)
}

// This is the kind of thing we would do only in the factory
// This would allow us to load a seed phrase into static memory
// in a factory image, so all factory-produced wallets would have
// this same seed stored in memory in a protected way.
pub fn erase_seed_phrase() -> Result<()> {
    // If our mnemonic hasn't been erased yet, and we have it saved
    // to disk, overwite it with 0's now
    if !MNEMONIC.is_empty() && load_seed_plaintext_size()?.is_some() {
        // If the seed has been saved to disk encrypted,
        let dp = unsafe { stm32::Peripherals::steal() };
        let mut flash = dp.FLASH;
        let mut unlocked = flash.unlocked();

        // Erase the seed from flash
        let addr = MNEMONIC.as_ptr() as usize - FLASH_START as usize;
        for a in addr..addr + MNEMONIC.len() {
            unlocked.program(a, &[0; 1])?;
        }
        Ok(())
    } else {
        Err(WalletErr::from("failed to erase seed phrase"))
    }
}

pub fn save_seed_phrase_encr(s: &str) -> Result<()> {
    let mut buffer: Vec<u8, U512> = Vec::new();
    buffer
        .extend_from_slice(s.as_bytes())
        .map_err(|_| WalletErr::from("failed to extend buffer from seed"))?;

    // `U8` represents the tag size as a `typenum` unsigned (8-bytes here)
    let mut key = get_aes_key();
    let ccm = Aes256Ccm::<U8>::new((&key).into());
    // Clear the key from memory
    key.iter_mut().for_each(|b| *b = 0);

    // Encrypt `buffer` in-place, replacing the plaintext contents with ciphertext
    // Use the UID of the chip as the associated_data
    ccm.encrypt_in_place(NONCE.into(), &get_uid_raw(), &mut buffer)?;

    // Save to flash
    let dp = unsafe { stm32::Peripherals::steal() };
    let mut flash = dp.FLASH;
    let mut unlocked = flash.unlocked();

    // First write the size as 2 bytes
    unlocked.program(SEED_ADDR as usize, &(buffer.len() as u16).to_le_bytes())?;
    unlocked.program((SEED_ADDR + 2) as usize, &buffer)?;

    Ok(())
}

pub fn load_seed_plaintext_size() -> Result<Option<usize>> {
    // Load encrypted seed phrase from flash
    // The first four bytes are the length of the plaintext
    let sz_bytes = unsafe { core::slice::from_raw_parts((FLASH_START + SEED_ADDR) as *const _, 2) };
    let sz = u16::from_le_bytes(sz_bytes.try_into()?);
    // If the size is zero or 0xFFFFFFFF, it's not been set. Therefore we have no saved seed
    if sz == !0x0000 || sz == 0x0000 {
        Ok(None)
    } else {
        Ok(Some(u16::from_le_bytes(sz_bytes.try_into()?) as usize))
    }
}

/// Replaces the contents of `buffer` with decrypted data
pub fn load_seed_phrase<'a, N>(buffer: &'a mut Vec<u8, N>) -> Result<&'a str>
where
    N: ArrayLength<u8>,
{
    let sz =
        load_seed_plaintext_size()?.ok_or_else(|| WalletErr::from("no seed phrase to load"))?;
    // The plaintext is stored 2 bytes past the size
    let plaintext =
        unsafe { core::slice::from_raw_parts((FLASH_START + SEED_ADDR + 2) as *const u8, sz) };

    buffer
        .extend_from_slice(plaintext)
        .map_err(|_| WalletErr::from("could not push plaintext bytes to buffer"))?;

    // `U8` represents the tag size as a `typenum` unsigned (8-bytes here)
    let mut key = get_aes_key();
    let ccm = Aes256Ccm::<U8>::new((&key).into());
    // Clear the key from memory
    key.iter_mut().for_each(|b| *b = 0);

    // Decrypt `buffer` in-place, replacing its ciphertext contents with the original plaintext
    // Use the UID of the chip as the associated_data
    ccm.decrypt_in_place(NONCE.into(), &get_uid_raw(), buffer)
        .map_err(|_| WalletErr::from("failed to decrypt"))?;

    Ok(core::str::from_utf8(buffer)
        .map_err(|_| WalletErr::from("failed to decode decrypted seed as utf8; corrupt?"))?)
}

pub fn load_seed() -> Result<Seed> {
    // Decrypt the seed_phrase
    let mut buffer: Vec<u8, U512> = Vec::new();
    let seed_phrase = load_seed_phrase(&mut buffer)?;
    // Generate a mnemonic from it
    let m = Mnemonic::from_phrase(seed_phrase, Language::English)?;
    Ok(Seed::new(&m, ""))
}

pub fn secret_key(ctx: &Context) -> Result<SigningKey> {
    let account = ExtendedPrivKey::derive(ctx.seed.as_bytes(), "m/44'/60'/0'/0")?
        .child(ChildNumber::non_hardened_from_u32(ctx.idx))?;
    Ok(SigningKey::from_bytes(&account.secret())?)
}

pub fn public_key(ctx: &Context) -> Result<VerifyingKey> {
    Ok(VerifyingKey::from(&secret_key(ctx)?))
}
