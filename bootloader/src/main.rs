#![no_main]
#![no_std]

use cortex_m::register::{msp, psp};
use cortex_m_rt::entry;
use hal::{prelude::*, stm32};
use panic_halt as _; // panic handler
use stm32f4xx_hal as hal;

static APP_ADDR: u32 = 0x08000000 + 0x400;

#[entry]
fn main() -> ! {
    let mut dp = cortex_m::Peripherals::take().unwrap();

    // Disable systick
    dp.SYST.disable_interrupt();

    // Load the stack pointer from the vector table (first entry) and set it
    let sp: u32 = unsafe { *(APP_ADDR as *const _) };
    let entry_point: u32 = unsafe { *((APP_ADDR + 4) as *const _) };

    unsafe {
        dp.SCB.vtor.write(APP_ADDR);
    }

    // Set the main stack pointer
    unsafe {
        msp::write(sp);
        psp::write(sp);
    }

    let start = unsafe { core::mem::transmute::<u32, extern "C" fn() -> !>(entry_point) };
    start()
}
