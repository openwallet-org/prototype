#![no_main]
#![no_std]

pub mod error;
mod safemem;

use core::convert::TryInto;
use error::{ErrStringType, WalletErr};

use bip39::{Language, Mnemonic, Seed};
use hex_literal::hex;
// use numtoa::NumToA;

use panic_halt as _; // panic handler
use stm32f4xx_hal as hal;

use hal::flash::FlashExt;

use k256::{
    ecdsa::{
        recoverable,
        signature::{Signature, Signer},
        SigningKey, VerifyingKey,
    },
    elliptic_curve::sec1::ToEncodedPoint,
};

use cortex_m_rt::entry;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::{prelude::*, stm32};
use usb_device::{
    class_prelude,
    device::{UsbDeviceBuilder, UsbVidPid},
    UsbError,
};

// use embedded_graphics::{image::Image, image::ImageRaw, pixelcolor::BinaryColor, prelude::*};
// use ssd1306::{prelude::*, Builder, I2CDIBuilder};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use heapless::{consts::*, ArrayLength, Vec};
use postcard::{from_bytes, to_vec};
use protocol::{Request, Response};
use tiny_hderive::{bip32::ExtendedPrivKey, bip44::ChildNumber};
use tiny_keccak::{Hasher, Keccak};

use aes_ccm::{
    aead::{consts::U8, AeadInPlace, NewAead},
    Aes128Ccm,
};

const FLASH_START: u32 = 0x0800_0000;
const FLASH_SIZE: u32 = 256 * 1024;
const STORAGE_START: u32 = FLASH_SIZE - 1024;
const SERIAL_ADDR: u32 = STORAGE_START;
const SEED_ADDR: u32 = STORAGE_START + 0xA;

type Result<T> = core::result::Result<T, WalletErr>;

struct Context {
    seed: Seed,
    pub idx: u32,
}

impl Context {
    pub fn set_idx(&mut self, idx: u32) -> &mut Self {
        self.idx = idx;
        self
    }
}

// A specifically sized buffer for the USB driver
static mut EP_MEMORY: [u32; 1024] = [0; 1024];

const MNEMONIC: &'static str = "panda eyebrow bullet gorilla call smoke muffin taste mesh discover soft ostrich alcohol speed nation flash devote level hobby quick inner drive ghost inside";
const AES_KEY: &[u8] = &hex!("C0 C1 C2 C3 C4 C5 C6 C7 C8 C9 CA CB CC CD CE CF");
const NONCE: &[u8] = &hex!("00 00 00 03 02 01 00 A0 A1 A2 A3 A4 A5");
const ASSOCIATED_DATA: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];

#[entry]
fn main() -> ! {
    // This unwrap is safe because we're the first/only to take() it
    let dp = stm32::Peripherals::take().unwrap();
    let rcc = dp.RCC.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(25.mhz())
        .sysclk(48.mhz())
        .require_pll48clk()
        .freeze();

    // On the current dev board, driving the LED corrupts the SWD
    // interface for some reason...so diabling
    // let gpioc = dp.GPIOC.split();
    // let mut led = gpioc.pc13.into_push_pull_output();

    // let _ = led.set_low();
    // let _ = led.set_high();

    // Do I2C related things
    // For I2C1, SCL=PB6, SDA=PB7, AF04
    // let gpiob = dp.GPIOB.split();
    // let scl = gpiob.pb6.into_alternate_af4_open_drain();
    // let sda = gpiob.pb7.into_alternate_af4_open_drain();
    // let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

    // let i2c_interface = I2CDIBuilder::new().init(i2c);
    // let mut disp: GraphicsMode<_, _> = Builder::new().connect(i2c_interface).into();
    // disp.init().unwrap();
    // disp.flush().unwrap();

    // // Display the rustacean
    // let raw_image: ImageRaw<BinaryColor> =
    //     ImageRaw::new(include_bytes!("../ssd1306-image.data"), 128, 64);
    // let image: Image<_, BinaryColor> = Image::new(&raw_image, Point::zero());
    // image.draw(&mut disp).unwrap();
    // disp.flush().unwrap();

    let gpioa = dp.GPIOA.split();
    let usb = USB {
        usb_global: dp.OTG_FS_GLOBAL,
        usb_device: dp.OTG_FS_DEVICE,
        usb_pwrclk: dp.OTG_FS_PWRCLK,
        pin_dm: gpioa.pa11.into_alternate_af10(),
        pin_dp: gpioa.pa12.into_alternate_af10(),
        hclk: clocks.hclk(),
    };
    let usb_bus = UsbBus::new(usb, unsafe { &mut EP_MEMORY });
    let mut serial = SerialPort::new(&usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0xDEAD, 0xBEEF))
        .manufacturer("noviinc")
        .product("NoviSigner")
        .serial_number("123")
        .device_class(USB_CLASS_CDC)
        .build();

    loop {
        if !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }

        // Initialize our receive buffer to zeros
        let mut buf = [0u8; 2048];

        let res = serial
            .read(&mut buf[..])
            .map_err(|e| match e {
                UsbError::WouldBlock => WalletErr::NoMsg,
                _ => WalletErr::StringErr(ErrStringType::from("failed to read usb")),
            })
            .and_then(|count| {
                // Turn the LED on, we've started processing a msg
                // let _ = led.set_low();
                // Deserialize the data into a Request
                Ok(from_bytes::<Request>(&buf[..count])?)
            })
            .and_then(|req| {
                // We've successfully deserialized into a Request -- process it
                answer_request(&req, &mut serial)
            });

        // If we have an actual error, send it to the host
        if let Err(WalletErr::StringErr(msg)) = res {
            respond_with_err(msg, &mut serial)
        }
        // Turn the LED off, in case it was turned on while processing a message
        // let _ = led.set_high();
    }
}

fn initialize() -> Result<Context> {
    if load_seed_plaintext_size()?.is_none() {
        save_seed_phrase_encr(MNEMONIC)?;
        erase_seed_phrase()?;
    }
    let seed = load_seed()?;
    let ctx = Context { seed, idx: 0 };
    Ok(ctx)
}

fn answer_request<T>(r: &Request, s: &mut SerialPort<T>) -> Result<()>
where
    T: class_prelude::UsbBus,
{
    match r {
        Request::Ping => transmit_response(Response::Pong, s),
        Request::Sig(msg) => {
            let mut ctx = initialize()?;
            let sig = sign_msg(&mut ctx, &msg)?;
            let sig_bytes = sig.as_bytes();
            transmit_response(Response::Sig(&sig_bytes), s)
        }
        Request::PubKey => {
            let ctx = initialize()?;
            let pubkey_bytes = public_key(&ctx)?.to_bytes();
            transmit_response(Response::PubKey(&pubkey_bytes), s)
        }
        Request::Address(idx) => {
            let mut ctx = initialize()?;
            let addr_bytes = address(ctx.set_idx(*idx))?;
            transmit_response(Response::Address(&addr_bytes), s)
        }
        Request::AddressList(idx) => {
            let mut ctx = initialize()?;
            let addresses = addresses(&mut ctx.set_idx(*idx))?;
            transmit_response(Response::AddressList(&addresses), s)
        }
        Request::Serial => {
            let serial = read_serial();
            let serial = if serial[0] == 0xFF {
                write_serial()?;
                read_serial()
            } else {
                serial
            };
            transmit_response(Response::Serial(serial), s)
        }
        Request::Info => transmit_response(
            Response::Info((
                load_seed_plaintext_size()?.is_some(),
                FLASH_START + STORAGE_START,
            )),
            s,
        ),
    }
}

fn sign_msg(ctx: &Context, msg: &[u8]) -> Result<recoverable::Signature> {
    Ok(secret_key(ctx)?.try_sign(&msg)?)
}

fn erase_seed_phrase() -> Result<()> {
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

fn save_seed_phrase_encr(s: &str) -> Result<()> {
    let mut buffer: Vec<u8, U512> = Vec::new();
    buffer
        .extend_from_slice(s.as_bytes())
        .map_err(|_| WalletErr::from("failed to extend buffer from seed"))?;

    // `U8` represents the tag size as a `typenum` unsigned (8-bytes here)
    let ccm = Aes128Ccm::<U8>::new(AES_KEY.into());

    // Encrypt `buffer` in-place, replacing the plaintext contents with ciphertext
    ccm.encrypt_in_place(NONCE.into(), &ASSOCIATED_DATA, &mut buffer)?;

    // Save to flash
    let dp = unsafe { stm32::Peripherals::steal() };
    let mut flash = dp.FLASH;
    let mut unlocked = flash.unlocked();

    // First write the size as 4 bytes
    unlocked.program(SEED_ADDR as usize, &buffer.len().to_le_bytes())?;
    unlocked.program((SEED_ADDR + 4) as usize, &buffer)?;

    Ok(())
}

fn load_seed_plaintext_size() -> Result<Option<usize>> {
    // Load encrypted seed phrase from flash
    // The first four bytes are the length of the plaintext
    let sz_bytes = unsafe { core::slice::from_raw_parts((FLASH_START + SEED_ADDR) as *const _, 4) };
    let sz = u32::from_le_bytes(sz_bytes.try_into()?);
    // If the size is zero or 0xFFFFFFFF, it's not been set. Therefore we have no saved seed
    if sz == !0x00000000 || sz == 0x00000000 {
        Ok(None)
    } else {
        Ok(Some(usize::from_le_bytes(sz_bytes.try_into()?)))
    }
}

/// Replaces the contents of `buffer` with decrypted data
fn load_seed_phrase<'a, N>(buffer: &'a mut Vec<u8, N>) -> Result<&'a str>
where
    N: ArrayLength<u8>,
{
    let sz =
        load_seed_plaintext_size()?.ok_or_else(|| WalletErr::from("no seed phrase to load"))?;
    // The plaintext is stored 4 bytes past the size
    let plaintext =
        unsafe { core::slice::from_raw_parts((FLASH_START + SEED_ADDR + 4) as *const u8, sz) };

    buffer
        .extend_from_slice(plaintext)
        .map_err(|_| WalletErr::from("could not push plaintext bytes to buffer"))?;

    // `U8` represents the tag size as a `typenum` unsigned (8-bytes here)
    let ccm = Aes128Ccm::<U8>::new(AES_KEY.into());

    // Decrypt `buffer` in-place, replacing its ciphertext contents with the original plaintext
    ccm.decrypt_in_place(NONCE.into(), &ASSOCIATED_DATA, buffer)
        .map_err(|_| WalletErr::from("failed to decrypt"))?;

    Ok(unsafe { core::str::from_utf8_unchecked(buffer) })
}

fn load_seed() -> Result<Seed> {
    // Decrypt the seed_phrase
    let mut buffer: Vec<u8, U512> = Vec::new();
    let seed_phrase = load_seed_phrase(&mut buffer)?;
    // Generate a mnemonic from it
    let m = Mnemonic::from_phrase(seed_phrase, Language::English)?;
    Ok(Seed::new(&m, ""))
}

fn secret_key(ctx: &Context) -> Result<SigningKey> {
    let account = ExtendedPrivKey::derive(ctx.seed.as_bytes(), "m/44'/60'/0'/0")?
        .child(ChildNumber::non_hardened_from_u32(ctx.idx))?;
    Ok(SigningKey::from_bytes(&account.secret())?)
}

fn public_key(ctx: &Context) -> Result<VerifyingKey> {
    Ok(VerifyingKey::from(&secret_key(ctx)?))
}

const NUM_ADDRS: usize = 5;
const ADDR_SIZE: usize = 20;
fn addresses(ctx: &mut Context) -> Result<[u8; ADDR_SIZE * NUM_ADDRS]> {
    let idx = ctx.idx as usize;
    let mut buf = [0u8; ADDR_SIZE * NUM_ADDRS];
    for i in idx..idx + NUM_ADDRS {
        let start: usize = (i - idx) * ADDR_SIZE;
        let stop: usize = start + ADDR_SIZE; // Addresses are 20 bytes
        buf[start..stop].copy_from_slice(&address(ctx.set_idx(i as u32))?);
    }
    Ok(buf)
}

fn address(ctx: &Context) -> Result<[u8; ADDR_SIZE]> {
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

fn read_serial<'a>() -> &'a [u8] {
    unsafe { core::slice::from_raw_parts((FLASH_START + SERIAL_ADDR) as *const u8, 10) }
}

const SERIAL_LEN: usize = 10;

fn is_serial_set() -> bool {
    let serial = unsafe {
        core::slice::from_raw_parts(
            ((<stm32::FLASH as FlashExt>::address() as u32) + SERIAL_ADDR) as *const u8,
            SERIAL_LEN,
        )
    };
    !serial
        .iter()
        .fold(true, |is_zero, b| is_zero & (*b == 0x00 || *b == 0xFF))
}

fn write_serial() -> Result<()> {
    // TODO: get the serial from manufacturing process
    if !is_serial_set() {
        // If our serial has not yet been written to disk, do so
        let mut serial_bytes = [0x42u8; 10];
        let dp = unsafe { stm32::Peripherals::steal() };
        let mut flash = dp.FLASH;
        let mut unlocked = flash.unlocked();
        // Increment the first byte so we know how many times this has been written
        // TODO: remove/refactor -- this is only for debug
        serial_bytes[0] += 1;
        unlocked.program(SERIAL_ADDR as usize, &serial_bytes[..])?;
    }
    Ok(())
}

fn transmit_response<T>(r: Response, serial: &mut SerialPort<T>) -> Result<()>
where
    T: class_prelude::UsbBus,
{
    let mut data = to_vec::<U1000, _>(&r)?;
    let _ = data.push(protocol::version());
    let _ = serial.write(&data)?;
    Ok(())
}

fn respond_with_err<T>(msg: ErrStringType, serial: &mut SerialPort<T>)
where
    T: class_prelude::UsbBus,
{
    // Create a Response::Err from our msg, silently fail
    let resp = Response::Err(msg.as_str());
    if let Ok(mut data) = to_vec::<U1000, _>(&resp) {
        let _ = data.push(protocol::version());
        let _ = serial.write(&data);
    } // else do nothing
}
