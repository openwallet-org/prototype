#![no_main]
#![no_std]

mod crypto;
pub mod error;
mod ethereum;
mod safemem;
mod utils;

use crypto::*;
use ethereum::*;

use core::fmt::Write;
use error::{ErrStringType, WalletErr};

use bip39::Seed;

use panic_halt as _; // panic handler
use stm32f4xx_hal as hal;

use hal::{flash::FlashExt, i2c::I2c};

use k256::ecdsa::signature::Signature;

use cortex_m_rt::entry;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::{prelude::*, stm32};
use usb_device::{
    class_prelude,
    device::{UsbDeviceBuilder, UsbVidPid},
    UsbError,
};

// use embedded_graphics::{
//     fonts::{Font6x8, Text},
//     image::Image,
//     image::ImageRaw,
//     pixelcolor::BinaryColor,
//     prelude::*,
//     style::TextStyleBuilder,
// };
use ssd1306::{prelude::*, Builder, I2CDIBuilder};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use heapless::consts::*;
use postcard::{from_bytes, to_vec};
use protocol::{Request, Response};

const FLASH_START: u32 = 0x0800_0000;
const FLASH_SIZE: u32 = 256 * 1024;
const STORAGE_START: u32 = FLASH_SIZE - 1024;
const SERIAL_ADDR: u32 = STORAGE_START;
const SEED_ADDR: u32 = STORAGE_START + 0xA;
const MNEMONIC: &str = "panda eyebrow bullet gorilla call smoke muffin taste mesh discover soft ostrich alcohol speed nation flash devote level hobby quick inner drive ghost inside";

type Result<T> = core::result::Result<T, WalletErr>;

pub struct Context {
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
    let gpiob = dp.GPIOB.split();
    let scl = gpiob.pb6.into_alternate_af4_open_drain();
    let sda = gpiob.pb7.into_alternate_af4_open_drain();
    let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

    let i2c_interface = I2CDIBuilder::new().init(i2c);
    // let mut disp: GraphicsMode<_, _> = Builder::new().connect(i2c_interface).into();
    let mut disp: TerminalMode<_, _> = Builder::new().connect(i2c_interface).into();
    disp.init().unwrap();
    let _ = disp.set_brightness(Brightness::DIM);
    let _ = disp.clear();
    // disp.flush().unwrap();

    // let text_style = TextStyleBuilder::new(Font6x8)
    //     .text_color(BinaryColor::On)
    //     .build();

    // Text::new("Hello world!", Point::zero())
    //     .into_styled(text_style)
    //     .draw(&mut disp)
    //     .unwrap();

    // Text::new("Hello Rust!", Point::new(0, 16))
    //     .into_styled(text_style)
    //     .draw(&mut disp)
    //     .unwrap();
    // disp.flush().unwrap();

    // Display the rustacean
    // let raw_image: ImageRaw<BinaryColor> =
    //     ImageRaw::new(include_bytes!("../ssd1306-image.data"), 128, 64);
    // let image: Image<_, BinaryColor> = Image::new(&raw_image, Point::zero());
    // image.draw(&mut disp).unwrap();
    // disp.flush().unwrap();

    // disp.clear();
    // for c in 97..123 {
    //     let _ = disp.write_str(unsafe { core::str::from_utf8_unchecked(&[c]) });
    // }
    // for c in 65..91 {
    //     let _ = disp.write_str(unsafe { core::str::from_utf8_unchecked(&[c]) });
    // }

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

    let mut ctx = initialize().map_err(|_| ()).unwrap();

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
                // disp.clear();
                disp.write_str(&addr_str(&ctx))?;
                // let pos = disp
                //     .get_position()
                //     .map_err(|_| WalletErr::from("unable to get screen pos"))?;
                // if disp.set_position(0, pos.1 + 1).is_err() {
                //     disp.set_position(0, 0);
                // }

                answer_request(&req, &mut serial, &mut ctx)
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

fn answer_request<T>(r: &Request, s: &mut SerialPort<T>, ctx: &mut Context) -> Result<()>
where
    T: class_prelude::UsbBus,
{
    match r {
        Request::Ping => transmit_response(Response::Pong, s),
        Request::Sig(msg) => {
            let sig = sign_msg(&ctx, &msg)?;
            let sig_bytes = sig.as_bytes();
            transmit_response(Response::Sig(&sig_bytes), s)
        }
        Request::PubKey => {
            let pubkey_bytes = public_key(&ctx)?.to_bytes();
            transmit_response(Response::PubKey(&pubkey_bytes), s)
        }
        Request::Address(idx) => {
            let addr_bytes = address(ctx.set_idx(*idx))?;
            transmit_response(Response::Address(&addr_bytes), s)
        }
        Request::AddressList(idx) => {
            let addresses = addresses(&mut ctx.set_idx(*idx))?;
            transmit_response(Response::AddressList(&addresses), s)
        }
        Request::Serial => {
            if !is_serial_set() {
                write_serial()?;
            };
            transmit_response(Response::Serial(read_serial()), s)
        }
        Request::Info => transmit_response(
            Response::Info((
                load_seed_plaintext_size()?.is_some(),
                FLASH_START + STORAGE_START,
                get_uid_raw(),
            )),
            s,
        ),
    }
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
